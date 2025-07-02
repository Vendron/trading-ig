//! Simplified streaming functionality for IG Markets real-time data.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use async_trait::async_trait;
use log::{debug, info, warn, error};

use lightstreamer_client::{
    Subscription, SubscriptionListener, ItemUpdate, SubscriptionMode
};

use crate::client::IGStreamService;
use crate::errors::{IGError, Result};

/// Callback trait for receiving raw ItemUpdates directly
pub trait RawDataCallback: Send + Sync {
    fn on_raw_update(&self, item_update: &ItemUpdate);
}

/// Simplified StreamingManager handles real-time market data subscriptions.
/// 
/// This class manages Lightstreamer subscriptions and can either:
/// 1. Process data into Ticker objects (legacy mode)
/// 2. Send raw ItemUpdates directly to callbacks (low-latency mode)
pub struct StreamingManager {
    /// Reference to the streaming service
    service: IGStreamService,
    /// Active subscriptions mapped by epic
    subscriptions: HashMap<String, TickerSubscription>,
    /// Ticker objects for real-time data (legacy mode)
    tickers: Arc<Mutex<HashMap<String, Arc<Mutex<Ticker>>>>>,
    /// Update channel for async processing (legacy mode)
    update_sender: Option<mpsc::UnboundedSender<ItemUpdate>>,
    /// Raw data callback for direct updates (low-latency mode)
    raw_callback: Option<Arc<dyn RawDataCallback>>,
}

impl StreamingManager {
    /// Create a new StreamingManager in legacy mode (with ticker aggregation).
    pub fn new(service: IGStreamService) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let tickers = Arc::new(Mutex::new(HashMap::new()));
        
        // Start consumer task
        let consumer_tickers = tickers.clone();
        tokio::spawn(async move {
            Self::consumer_task(rx, consumer_tickers).await;
        });
        
        Self {
            service,
            subscriptions: HashMap::new(),
            tickers,
            update_sender: Some(tx),
            raw_callback: None,
        }
    }

    /// Create a new StreamingManager in low-latency mode (direct raw data).
    pub fn new_with_raw_callback(service: IGStreamService, callback: Arc<dyn RawDataCallback>) -> Self {
        Self {
            service,
            subscriptions: HashMap::new(),
            tickers: Arc::new(Mutex::new(HashMap::new())),
            update_sender: None,
            raw_callback: Some(callback),
        }
    }

    /// Consumer task that processes ItemUpdate messages (legacy mode only).
    async fn consumer_task(
        mut receiver: mpsc::UnboundedReceiver<ItemUpdate>,
        tickers: Arc<Mutex<HashMap<String, Arc<Mutex<Ticker>>>>>,
    ) {
        log::info!("Consumer: Running");
        
        while let Some(item) = receiver.recv().await {
            // Process different data types efficiently (no verbose logging)
            if let Some(name) = item.item_name() {
                if name.starts_with("CHART:") {
                    if let Err(e) = Self::handle_ticker_update(&item, &tickers).await {
                        log::error!("Consumer: Failed to handle ticker update: {}", e);
                    }
                }
            }
        }
    }

    /// Handle ticker update from ItemUpdate (legacy mode only).
    async fn handle_ticker_update(
        item: &ItemUpdate,
        tickers: &Arc<Mutex<HashMap<String, Arc<Mutex<Ticker>>>>>,
    ) -> Result<()> {
        let epic = if let Some(item_name) = item.item_name() {
            Ticker::identifier(item_name)
        } else {
            return Ok(());
        };

        // Get or create ticker with live reference
        let ticker_ref = {
            let mut tickers_map = tickers.lock()
                .map_err(|_| IGError::generic("Failed to acquire tickers lock"))?;
            
            tickers_map.entry(epic.clone())
                .or_insert_with(|| Arc::new(Mutex::new(Ticker::new(epic.clone()))))
                .clone()
        };
        
        // Update the ticker with new data (fast, no logging overhead)
        let mut ticker = ticker_ref.lock()
            .map_err(|_| IGError::generic("Failed to acquire ticker lock"))?;
        
        ticker.populate(item.get_changed_fields())?;

        Ok(())
    }

    /// Start a tick subscription for the specified epic.
    pub async fn start_tick_subscription(&mut self, epic: &str) -> Result<()> {
        // Create TickerSubscription
        let mut tick_sub = TickerSubscription::new(epic.to_string());
        
        // Add listener based on mode
        let listener = if let Some(ref callback) = self.raw_callback {
            // Low-latency mode: send directly to callback
            TickerListener::new_with_raw_callback(callback.clone())
        } else if let Some(ref sender) = self.update_sender {
            // Legacy mode: send to consumer task
            TickerListener::new_with_channel(sender.clone())
        } else {
            return Err(IGError::generic("StreamingManager not properly initialized"));
        };
        
        tick_sub.add_listener(listener)?;
        
        // Subscribe via service
        if let Some(lightstreamer_client) = self.service.lightstreamer_client() {
            lightstreamer_client.subscribe(tick_sub.subscription().clone()).await
                .map_err(|e| IGError::connection(format!("Failed to subscribe: {}", e)))?;
            
            log::info!("Started tick subscription for epic: {}", epic);
        } else {
            return Err(IGError::session("No Lightstreamer client available"));
        }
        
        // Store subscription
        self.subscriptions.insert(epic.to_string(), tick_sub);

        Ok(())
    }

    /// Stop tick subscription for the specified epic.
    pub async fn stop_tick_subscription(&mut self, epic: &str) -> Result<()> {
        if let Some(_subscription) = self.subscriptions.remove(epic) {
            log::info!("Stopped tick subscription for epic: {}", epic);
        }
        Ok(())
    }

    /// Get ticker for the specified epic (legacy mode only).
    pub async fn ticker(&self, epic: &str) -> Result<Arc<Mutex<Ticker>>> {
        // This method only works in legacy mode
        if self.raw_callback.is_some() {
            return Err(IGError::generic("ticker() method not available in raw callback mode"));
        }

        // Wait for ticker to become available
        let mut attempts = 0;
        let max_attempts = 12;
        
        while attempts < max_attempts {
            log::debug!("Waiting for ticker...");
            
            // Check if ticker exists
            {
                let tickers = self.tickers.lock()
                    .map_err(|_| IGError::generic("Failed to acquire tickers lock"))?;
                if let Some(ticker) = tickers.get(epic) {
                    return Ok(ticker.clone());
                }
            }
            
            attempts += 1;
            sleep(Duration::from_millis(250)).await;
        }
        
        Err(IGError::generic(format!("No ticker found for {}, giving up", epic)))
    }

    /// Stop all subscriptions.
    pub async fn stop_subscriptions(&mut self) -> Result<()> {
        log::info!("Unsubscribing from all subscriptions");
        
        let epics: Vec<String> = self.subscriptions.keys().cloned().collect();
        for epic in epics {
            if let Err(e) = self.stop_tick_subscription(&epic).await {
                log::error!("Failed to stop subscription for {}: {}", epic, e);
            }
        }
        
        Ok(())
    }
}

/// Simplified TickerSubscription represents a subscription for tick prices.
pub struct TickerSubscription {
    /// The underlying Lightstreamer subscription (wrapped for thread safety)
    subscription: Arc<std::sync::Mutex<Subscription>>,
    /// Epic identifier
    epic: String,
}

impl TickerSubscription {
    pub const TICKER_FIELDS: &'static [&'static str] = &[
        "BID",                    // Bid price
        "OFR",                    // Offer/Ask price  
        "LTP",                    // Last traded price
        "LTV",                    // Last traded volume
        "TTV",                    // Total traded volume (incremental)
        "UTM",                    // Update timestamp (Unix milliseconds)
        "DAY_OPEN_MID",          // Day opening mid price
        "DAY_NET_CHG_MID",       // Day net change (mid price)
        "DAY_PERC_CHG_MID",      // Day percentage change (mid price)
        "DAY_HIGH",              // Day high price
        "DAY_LOW"                // Day low price
    ];

    /// Create a new TickerSubscription.
    pub fn new(epic: String) -> Self {
        let subscription = Subscription::new(
            SubscriptionMode::Distinct,                    // DISTINCT mode
            vec![format!("CHART:{}:TICK", epic)],         // CHART subscription
            Self::TICKER_FIELDS.iter().map(|s| s.to_string()).collect(),  // All fields
        );

        Self {
            subscription: Arc::new(std::sync::Mutex::new(subscription)),
            epic,
        }
    }

    /// Get the epic identifier.
    pub fn epic(&self) -> &str {
        &self.epic
    }

    /// Add a listener to the subscription.
    pub fn add_listener(&mut self, listener: TickerListener) -> Result<()> {
        let mut subscription = self.subscription.lock()
            .map_err(|_| IGError::generic("Failed to acquire subscription lock"))?;
        subscription.add_listener(Arc::new(listener));
        Ok(())
    }

    /// Get reference to the subscription (for lightstreamer client).
    pub fn subscription(&self) -> &Arc<std::sync::Mutex<Subscription>> {
        &self.subscription
    }
}

/// Ticker represents real-time price data for an instrument.
#[derive(Debug, Clone)]
pub struct Ticker {
    /// Epic identifier
    epic: String,
    /// Last update timestamp
    timestamp: Option<DateTime<Utc>>,
    /// Current bid price
    bid: Option<Decimal>,
    /// Current offer price (ask)
    offer: Option<Decimal>,
    /// Last traded price
    last_traded_price: Option<Decimal>,
    /// Last traded volume
    last_traded_volume: Option<i64>,
    /// Incremental volume (TTV in IG)
    incr_volume: Option<i64>,
    /// Day open mid price
    day_open_mid: Option<Decimal>,
    /// Day net change mid
    day_net_change_mid: Option<Decimal>,
    /// Day percentage change mid
    day_percent_change_mid: Option<Decimal>,
    /// Day high price
    day_high: Option<Decimal>,
    /// Day low price
    day_low: Option<Decimal>,
}

impl Ticker {
    /// Create a new Ticker.
    pub fn new(epic: String) -> Self {
        Self {
            epic,
            timestamp: None,
            bid: None,
            offer: None,
            last_traded_price: None,
            last_traded_volume: None,
            incr_volume: None,
            day_open_mid: None,
            day_net_change_mid: None,
            day_percent_change_mid: None,
            day_high: None,
            day_low: None,
        }
    }

    /// Get the epic identifier.
    pub fn epic(&self) -> &str {
        &self.epic
    }

    /// Extract epic from item name 
    /// Example: "CHART:IX.D.SPTRD.DAILY.IP:TICK".
    pub fn identifier(item_name: &str) -> String {
        // Extract epic from "CHART:EPIC:TICK" format
        item_name.split(':')
            .nth(1)
            .unwrap_or("")
            .to_string()
    }

    /// Populate ticker with latest data from Lightstreamer.
    /// Implements Python-style state management: merges deltas with previous state.
    pub fn populate(&mut self, values: &lightstreamer_client::FieldMap) -> Result<()> {
        // Python-style state management: merge new values with existing state
        // Only update fields that have actually changed (non-null values)
        // Empty/null values in Lightstreamer protocol mean "no change, keep previous"
        
        use lightstreamer_client::FieldValue;
        
        // UTM (timestamp) - only if changed
        if let Some(FieldValue::Integer(timestamp_ms)) = values.get("UTM") {
            self.timestamp = Some(DateTime::from_timestamp(timestamp_ms / 1000, 0).unwrap_or_default());
        }

        // BID - only if changed (Python behavior: null = no change)
        match values.get("BID") {
            Some(FieldValue::Float(bid_float)) => {
                self.bid = Some(Decimal::from_f64_retain(*bid_float).unwrap_or_default());
            }
            Some(FieldValue::String(bid_str)) => {
                if let Ok(bid_float) = bid_str.parse::<f64>() {
                    self.bid = Some(Decimal::from_f64_retain(bid_float).unwrap_or_default());
                }
            }
            Some(FieldValue::Integer(bid_int)) => {
                self.bid = Some(Decimal::from_i64(*bid_int).unwrap_or_default());
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value (Python behavior)
                // self.bid unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.bid unchanged
            }
        }

        // OFR (Offer/Ask) - only if changed
        match values.get("OFR") {
            Some(FieldValue::Float(offer_float)) => {
                self.offer = Some(Decimal::from_f64_retain(*offer_float).unwrap_or_default());
            }
            Some(FieldValue::String(offer_str)) => {
                if let Ok(offer_float) = offer_str.parse::<f64>() {
                    self.offer = Some(Decimal::from_f64_retain(offer_float).unwrap_or_default());
                }
            }
            Some(FieldValue::Integer(offer_int)) => {
                self.offer = Some(Decimal::from_i64(*offer_int).unwrap_or_default());
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.offer unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.offer unchanged
            }
        }

        // LTP (Last Traded Price) - only if changed
        match values.get("LTP") {
            Some(FieldValue::Float(ltp_float)) => {
                self.last_traded_price = Some(Decimal::from_f64_retain(*ltp_float).unwrap_or_default());
            }
            Some(FieldValue::String(ltp_str)) => {
                if let Ok(ltp_float) = ltp_str.parse::<f64>() {
                    self.last_traded_price = Some(Decimal::from_f64_retain(ltp_float).unwrap_or_default());
                }
            }
            Some(FieldValue::Integer(ltp_int)) => {
                self.last_traded_price = Some(Decimal::from_i64(*ltp_int).unwrap_or_default());
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.last_traded_price unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.last_traded_price unchanged
            }
        }

        // LTV (Last Traded Volume) - only if changed
        match values.get("LTV") {
            Some(FieldValue::Integer(ltv_int)) => {
                self.last_traded_volume = Some(*ltv_int);
            }
            Some(FieldValue::String(ltv_str)) => {
                if let Ok(ltv_int) = ltv_str.parse::<i64>() {
                    self.last_traded_volume = Some(ltv_int);
                }
            }
            Some(FieldValue::Float(ltv_float)) => {
                self.last_traded_volume = Some(*ltv_float as i64);
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.last_traded_volume unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.last_traded_volume unchanged
            }
        }

        // TTV (Total Traded Volume / Incremental) - only if changed
        match values.get("TTV") {
            Some(FieldValue::Integer(ttv_int)) => {
                self.incr_volume = Some(*ttv_int);
            }
            Some(FieldValue::String(ttv_str)) => {
                if let Ok(ttv_int) = ttv_str.parse::<i64>() {
                    self.incr_volume = Some(ttv_int);
                }
            }
            Some(FieldValue::Float(ttv_float)) => {
                self.incr_volume = Some(*ttv_float as i64);
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.incr_volume unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.incr_volume unchanged
            }
        }

        // DAY_OPEN_MID - only if changed
        match values.get("DAY_OPEN_MID") {
            Some(FieldValue::Float(open_float)) => {
                self.day_open_mid = Some(Decimal::from_f64_retain(*open_float).unwrap_or_default());
            }
            Some(FieldValue::String(open_str)) => {
                if let Ok(open_float) = open_str.parse::<f64>() {
                    self.day_open_mid = Some(Decimal::from_f64_retain(open_float).unwrap_or_default());
                }
            }
            Some(FieldValue::Integer(open_int)) => {
                self.day_open_mid = Some(Decimal::from_i64(*open_int).unwrap_or_default());
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.day_open_mid unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.day_open_mid unchanged
            }
        }

        // DAY_NET_CHG_MID - only if changed
        match values.get("DAY_NET_CHG_MID") {
            Some(FieldValue::Float(net_chg_float)) => {
                self.day_net_change_mid = Some(Decimal::from_f64_retain(*net_chg_float).unwrap_or_default());
            }
            Some(FieldValue::Integer(net_chg_int)) => {
                self.day_net_change_mid = Some(Decimal::from_i64(*net_chg_int).unwrap_or_default());
            }
            Some(FieldValue::String(net_chg_str)) => {
                if let Ok(net_chg_float) = net_chg_str.parse::<f64>() {
                    self.day_net_change_mid = Some(Decimal::from_f64_retain(net_chg_float).unwrap_or_default());
                }
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.day_net_change_mid unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.day_net_change_mid unchanged
            }
        }

        // DAY_PERC_CHG_MID - only if changed
        match values.get("DAY_PERC_CHG_MID") {
            Some(FieldValue::Float(perc_chg_float)) => {
                self.day_percent_change_mid = Some(Decimal::from_f64_retain(*perc_chg_float).unwrap_or_default());
            }
            Some(FieldValue::String(perc_chg_str)) => {
                if let Ok(perc_chg_float) = perc_chg_str.parse::<f64>() {
                    self.day_percent_change_mid = Some(Decimal::from_f64_retain(perc_chg_float).unwrap_or_default());
                }
            }
            Some(FieldValue::Integer(perc_chg_int)) => {
                self.day_percent_change_mid = Some(Decimal::from_i64(*perc_chg_int).unwrap_or_default());
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.day_percent_change_mid unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.day_percent_change_mid unchanged
            }
        }

        // DAY_HIGH - only if changed
        match values.get("DAY_HIGH") {
            Some(FieldValue::Float(high_float)) => {
                self.day_high = Some(Decimal::from_f64_retain(*high_float).unwrap_or_default());
            }
            Some(FieldValue::String(high_str)) => {
                if let Ok(high_float) = high_str.parse::<f64>() {
                    self.day_high = Some(Decimal::from_f64_retain(high_float).unwrap_or_default());
                }
            }
            Some(FieldValue::Integer(high_int)) => {
                self.day_high = Some(Decimal::from_i64(*high_int).unwrap_or_default());
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.day_high unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.day_high unchanged
            }
        }

        // DAY_LOW - only if changed
        match values.get("DAY_LOW") {
            Some(FieldValue::Float(low_float)) => {
                self.day_low = Some(Decimal::from_f64_retain(*low_float).unwrap_or_default());
            }
            Some(FieldValue::String(low_str)) => {
                if let Ok(low_float) = low_str.parse::<f64>() {
                    self.day_low = Some(Decimal::from_f64_retain(low_float).unwrap_or_default());
                }
            }
            Some(FieldValue::Integer(low_int)) => {
                self.day_low = Some(Decimal::from_i64(*low_int).unwrap_or_default());
            }
            Some(FieldValue::Null) => {
                // Null means "no change" - keep existing value
                // self.day_low unchanged
            }
            None => {
                // Field not present in update - keep existing value
                // self.day_low unchanged
            }
        }

        Ok(())
    }

    // Simplified getter methods (no need for complex Option handling)
    
    pub fn timestamp(&self) -> Option<DateTime<Utc>> {
        self.timestamp
    }

    pub fn bid(&self) -> Option<Decimal> {
        self.bid
    }

    pub fn offer(&self) -> Option<Decimal> {
        self.offer
    }

    pub fn last_traded_price(&self) -> Option<Decimal> {
        self.last_traded_price
    }

    pub fn last_traded_volume(&self) -> Option<i64> {
        self.last_traded_volume
    }

    pub fn incr_volume(&self) -> Option<i64> {
        self.incr_volume
    }

    pub fn day_open_mid(&self) -> Option<Decimal> {
        self.day_open_mid
    }

    pub fn day_net_change_mid(&self) -> Option<Decimal> {
        self.day_net_change_mid
    }

    pub fn day_percent_change_mid(&self) -> Option<Decimal> {
        self.day_percent_change_mid
    }

    pub fn day_high(&self) -> Option<Decimal> {
        self.day_high
    }

    pub fn day_low(&self) -> Option<Decimal> {
        self.day_low
    }
}

/// TickerListener forwards updates either to a channel (legacy) or raw callback (low-latency).
pub struct TickerListener {
    /// Mode of operation
    mode: TickerListenerMode,
}

/// Mode enum for TickerListener
enum TickerListenerMode {
    /// Legacy mode: send to channel for processing
    Channel(mpsc::UnboundedSender<ItemUpdate>),
    /// Low-latency mode: call raw callback directly
    RawCallback(Arc<dyn RawDataCallback>),
}

impl TickerListener {
    /// Create a new TickerListener with channel (legacy mode).
    pub fn new_with_channel(update_sender: mpsc::UnboundedSender<ItemUpdate>) -> Self {
        Self { 
            mode: TickerListenerMode::Channel(update_sender)
        }
    }

    /// Create a new TickerListener with raw callback (low-latency mode).
    pub fn new_with_raw_callback(callback: Arc<dyn RawDataCallback>) -> Self {
        Self {
            mode: TickerListenerMode::RawCallback(callback)
        }
    }
}

#[async_trait]
impl SubscriptionListener for TickerListener {
    async fn on_subscription(&self) {
        log::info!("TickerListener onSubscription()");
    }

    async fn on_subscription_error(&self, code: i32, message: &str) {
        log::error!("TickerListener onSubscriptionError(): '{}' {}", code, message);
    }

    async fn on_unsubscription(&self) {
        log::info!("TickerListener onUnsubscription()");
    }

    async fn on_item_update(&self, update: &ItemUpdate) {
        match &self.mode {
            TickerListenerMode::Channel(sender) => {
                // Legacy mode: send to channel for processing
                let _ = sender.send(update.clone());
            }
            TickerListenerMode::RawCallback(callback) => {
                // Low-latency mode: call callback directly
                callback.on_raw_update(update);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::IGService;
    use crate::types::AccountType;

    #[test]
    fn test_ticker_creation() {
        let ticker = Ticker::new("IX.D.SPTRD.MONTH1.IP".to_string());
        assert_eq!(ticker.epic(), "IX.D.SPTRD.MONTH1.IP");
        assert!(ticker.bid().is_none());
    }

    #[test]
    fn test_ticker_identifier() {
        let epic = Ticker::identifier("CHART:IX.D.SPTRD.MONTH1.IP:TICK");
        assert_eq!(epic, "IX.D.SPTRD.MONTH1.IP");
    }

    #[test]
    fn test_ticker_subscription_creation() {
        let subscription = TickerSubscription::new("IX.D.SPTRD.MONTH1.IP".to_string());
        assert_eq!(subscription.epic(), "IX.D.SPTRD.MONTH1.IP");
    }
} 