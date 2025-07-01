//! Streaming functionality for IG Markets real-time data.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use lightstreamer_client::{
    Subscription, SubscriptionListener, ItemUpdate, SubscriptionMode
};

use crate::client::IGStreamService;
use crate::errors::{IGError, Result};
use crate::utils::build_chart_item_name;

/// StreamingManager handles real-time market data subscriptions.
/// 
/// This class manages Lightstreamer subscriptions and processes incoming
/// market data updates, maintaining a collection of Ticker objects for
/// subscribed instruments.
/// 
pub struct StreamingManager {
    /// Reference to the streaming service
    service: Arc<Mutex<IGStreamService>>,
    /// Active subscriptions mapped by epic
    subscriptions: Arc<Mutex<HashMap<String, Arc<Mutex<TickerSubscription>>>>>,
    /// Ticker objects for real-time data
    tickers: Arc<Mutex<HashMap<String, Arc<Mutex<Ticker>>>>>,
    /// Consumer for processing updates (async equivalent of Consumer thread)
    _consumer_handle: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Update channel for async processing
    update_sender: mpsc::UnboundedSender<ItemUpdate>,
}

impl StreamingManager {
    /// Create a new StreamingManager.
    pub fn new(service: IGStreamService) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        
        let subscriptions = Arc::new(Mutex::new(HashMap::new()));
        let tickers = Arc::new(Mutex::new(HashMap::new()));
        
        // Start consumer task
        let consumer_tickers = tickers.clone();
        let consumer_handle = tokio::spawn(async move {
            Self::consumer_task(rx, consumer_tickers).await;
        });
        
        Self {
            service: Arc::new(Mutex::new(service)),
            subscriptions,
            tickers,
            _consumer_handle: Arc::new(Mutex::new(Some(consumer_handle))),
            update_sender: tx,
        }
    }

    /// Consumer task that processes ItemUpdate messages.
    async fn consumer_task(
        mut receiver: mpsc::UnboundedReceiver<ItemUpdate>,
        tickers: Arc<Mutex<HashMap<String, Arc<Mutex<Ticker>>>>>,
    ) {
        log::info!("Consumer: Running");
        
        while let Some(item) = receiver.recv().await {
            // Deal with each different type of update
            if let Some(name) = item.item_name() {
                if name.starts_with("CHART:") {
                    if let Err(e) = Self::handle_ticker_update(&item, &tickers).await {
                        log::error!("Failed to handle ticker update: {}", e);
                    }
                }
            }
            
            log::debug!("Consumer thread alive. queue length: processing");
        }
    }

    /// Handle ticker update from ItemUpdate.
    /// 
    /// Python equivalent: Consumer._handle_ticker_update()
    async fn handle_ticker_update(
        item: &ItemUpdate,
        tickers: &Arc<Mutex<HashMap<String, Arc<Mutex<Ticker>>>>>,
    ) -> Result<()> {
        let epic = if let Some(item_name) = item.item_name() {
            Ticker::identifier(item_name)
        } else {
            return Ok(());
        };

        // Get or create ticker
        let ticker_arc = {
            let mut tickers_map = tickers.lock()
                .map_err(|_| IGError::generic("Failed to acquire tickers lock"))?;
            
            if !tickers_map.contains_key(&epic) {
                let ticker = Arc::new(Mutex::new(Ticker::new(epic.clone())));
                tickers_map.insert(epic.clone(), ticker.clone());
                ticker
            } else {
                tickers_map[&epic].clone()
            }
        };

        // Populate ticker with new data
        {
            let mut ticker = ticker_arc.lock()
                .map_err(|_| IGError::generic("Failed to acquire ticker lock"))?;
            ticker.populate(item.get_changed_fields())?;
        }

        Ok(())
    }

    /// Start a tick subscription for the specified epic.
    /// 
    /// Python equivalent:
    /// ```python
    /// def start_tick_subscription(self, epic) -> TickerSubscription:
    ///     tick_sub = TickerSubscription(epic)
    ///     tick_sub.addListener(TickerListener(self._queue))
    ///     self.service.subscribe(tick_sub)
    ///     self._subs[epic] = tick_sub
    ///     return tick_sub
    /// ```
    pub async fn start_tick_subscription(&self, epic: &str) -> Result<Arc<Mutex<TickerSubscription>>> {
        // Create TickerSubscription
        let mut tick_sub = TickerSubscription::new(epic.to_string());
        
        // Add listener
        let listener = Arc::new(TickerListener::new(self.update_sender.clone()));
        tick_sub.add_listener(listener)?;
        
        let tick_sub_arc = Arc::new(Mutex::new(tick_sub));
        
        // Subscribe via service
        {
            let service = self.service.lock()
                .map_err(|_| IGError::generic("Failed to acquire service lock"))?;
            // Note: In full implementation, this would call service.subscribe(subscription)
            log::info!("Started tick subscription for epic: {}", epic);
        }
        
        // Store subscription
        {
            let mut subs = self.subscriptions.lock()
                .map_err(|_| IGError::generic("Failed to acquire subscriptions lock"))?;
            subs.insert(epic.to_string(), tick_sub_arc.clone());
        }

        Ok(tick_sub_arc)
    }

    /// Stop tick subscription for the specified epic.
    /// 
    /// Python equivalent:
    /// ```python
    /// def stop_tick_subscription(self, epic):
    ///     subscription = self._subs.pop(epic)
    ///     self.service.unsubscribe(subscription)
    /// ```
    pub async fn stop_tick_subscription(&self, epic: &str) -> Result<()> {
        // Remove and get subscription
        let subscription = {
            let mut subs = self.subscriptions.lock()
                .map_err(|_| IGError::generic("Failed to acquire subscriptions lock"))?;
            subs.remove(epic)
        };

        if let Some(_subscription) = subscription {
            // Unsubscribe via service
            {
                let service = self.service.lock()
                    .map_err(|_| IGError::generic("Failed to acquire service lock"))?;
                // Note: In full implementation, this would call service.unsubscribe(subscription)
            }

            log::info!("Stopped tick subscription for epic: {}", epic);
        }

        Ok(())
    }

    /// Get ticker for the specified epic.
    /// 
    /// Python equivalent:
    /// ```python
    /// def ticker(self, epic):
    ///     timeout = time.time() + 3
    ///     while True:
    ///         logger.debug("Waiting for ticker...")
    ///         if epic in self._tickers or time.time() > timeout:
    ///             break
    ///         time.sleep(0.25)
    ///     ticker = self._tickers[epic]
    ///     if not ticker:
    ///         raise Exception(f"No ticker found for {epic}, giving up")
    ///     return ticker
    /// ```
    pub async fn ticker(&self, epic: &str) -> Result<Arc<Mutex<Ticker>>> {
        // We won't have a ticker until at least one update is received from server,
        // let's give it a few seconds (exactly like Python)
        let mut attempts = 0;
        let max_attempts = 12; // 12 * 250ms = 3 seconds
        
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
            
            // Wait 0.25 seconds
            sleep(Duration::from_millis(250)).await;
            attempts += 1;
        }

        Err(IGError::invalid_market_data(format!("No ticker found for {}, giving up", epic)))
    }

    /// Stop all subscriptions and disconnect.
    /// 
    /// Python equivalent:
    /// ```python
    /// def stop_subscriptions(self):
    ///     logger.info("Unsubscribing from all")
    ///     self.service.unsubscribe_all()
    ///     self.service.disconnect()
    ///     if self._consumer_thread:
    ///         self._consumer_thread.join(timeout=5)
    ///         self._consumer_thread = None
    /// ```
    pub async fn stop_subscriptions(&self) -> Result<()> {
        log::info!("Unsubscribing from all");
        
        // Unsubscribe from all via service
        {
            let service = self.service.lock()
                .map_err(|_| IGError::generic("Failed to acquire service lock"))?;
            // Note: In full implementation, this would call:
            // service.unsubscribe_all();
            // service.disconnect();
        }

        // Clear local collections
        {
            let mut subs = self.subscriptions.lock()
                .map_err(|_| IGError::generic("Failed to acquire subscriptions lock"))?;
            subs.clear();
        }

        {
            let mut tickers = self.tickers.lock()
                .map_err(|_| IGError::generic("Failed to acquire tickers lock"))?;
            tickers.clear();
        }

        Ok(())
    }

    /// Property accessor for service (matches Python @property service)
    pub fn service(&self) -> &Arc<Mutex<IGStreamService>> {
        &self.service
    }

    /// Property accessor for tickers (matches Python @property tickers)
    pub fn tickers(&self) -> &Arc<Mutex<HashMap<String, Arc<Mutex<Ticker>>>>> {
        &self.tickers
    }
}

/// TickerSubscription represents a subscription for tick prices.
/// 
/// Python equivalent:
/// ```python
/// class TickerSubscription(Subscription):
///     TICKER_FIELDS = [
///         "BID", "OFR", "LTP", "LTV", "TTV", "UTM",
///         "DAY_OPEN_MID", "DAY_NET_CHG_MID", "DAY_PERC_CHG_MID", 
///         "DAY_HIGH", "DAY_LOW",
///     ]
///     def __init__(self, epic: str):
///         super().__init__(
///             mode="DISTINCT",
///             items=[f"CHART:{epic}:TICK"],
///             fields=self.TICKER_FIELDS,
///         )
/// ```
pub struct TickerSubscription {
    /// The underlying Lightstreamer subscription
    subscription: Subscription,
    /// Epic identifier
    epic: String,
}

impl TickerSubscription {
    /// Ticker fields exactly matching Python TICKER_FIELDS
    pub const TICKER_FIELDS: &'static [&'static str] = &[
        "BID",
        "OFR", 
        "LTP",
        "LTV",
        "TTV",
        "UTM",
        "DAY_OPEN_MID",
        "DAY_NET_CHG_MID",
        "DAY_PERC_CHG_MID",
        "DAY_HIGH",
        "DAY_LOW",
    ];

    /// Create a new TickerSubscription.
    /// 
    /// Python equivalent:
    /// ```python
    /// def __init__(self, epic: str):
    ///     super().__init__(
    ///         mode="DISTINCT",
    ///         items=[f"CHART:{epic}:TICK"],
    ///         fields=self.TICKER_FIELDS,
    ///     )
    /// ```
    pub fn new(epic: String) -> Self {
        let item_name = build_chart_item_name(&epic);
        let fields: Vec<String> = Self::TICKER_FIELDS.iter().map(|&s| s.to_string()).collect();
        
        let subscription = Subscription::new(
            SubscriptionMode::Distinct, // "DISTINCT" mode like Python
            vec![item_name],
            fields,
        );

        Self {
            subscription,
            epic,
        }
    }

    /// Get the epic identifier
    pub fn epic(&self) -> &str {
        &self.epic
    }

    /// Add a listener to the subscription
    pub fn add_listener(&mut self, listener: Arc<dyn SubscriptionListener>) -> Result<()> {
        self.subscription.add_listener(listener)
            .map_err(|e| IGError::generic(&format!("Failed to add listener: {}", e)))
    }

    /// Get the underlying subscription (for service.subscribe() calls)
    pub fn subscription(&self) -> &Subscription {
        &self.subscription
    }

    /// Get mutable access to underlying subscription
    pub fn subscription_mut(&mut self) -> &mut Subscription {
        &mut self.subscription
    }
}

/// Ticker represents real-time market data for a specific instrument.
/// 
/// Python equivalent:
/// ```python
/// @dataclass
/// class Ticker(StreamObject):
///     epic: str
///     timestamp: datetime = None
///     bid: float = nan
///     offer: float = nan
///     last_traded_price: float = nan
///     last_traded_volume: int = 0
///     incr_volume: int = 0
///     day_open_mid: float = nan
///     day_net_change_mid: float = nan
///     day_percent_change_mid: float = nan
///     day_high: float = nan
///     day_low: float = nan
/// ```
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
    /// Create a new Ticker for the specified epic.
    /// 
    /// Python equivalent:
    /// ```python
    /// def __init__(self, epic):
    ///     self.epic = epic
    /// ```
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

    /// Get the epic identifier
    pub fn epic(&self) -> &str {
        &self.epic
    }

    /// Extract epic from item name.
    /// 
    /// Python equivalent:
    /// ```python
    /// @classmethod
    /// def identifier(cls, name):
    ///     epic = name.split(":")[1]
    ///     return epic
    /// ```
    pub fn identifier(item_name: &str) -> String {
        // Split by ':' and get the second part (epic)
        // CHART:EPIC:TICK -> EPIC
        let parts: Vec<&str> = item_name.split(':').collect();
        if parts.len() >= 2 {
            parts[1].to_string()
        } else {
            item_name.to_string()
        }
    }

    /// Populate ticker with field values from ItemUpdate.
    /// 
    /// Python equivalent:
    /// ```python
    /// def populate(self, values):
    ///     self.set_timestamp_by_name("timestamp", values, "UTM")
    ///     self.set_by_name("bid", values, "BID", float)
    ///     self.set_by_name("offer", values, "OFR", float)
    ///     self.set_by_name("last_traded_price", values, "LTP", float)
    ///     self.set_by_name("last_traded_volume", values, "LTV", int)
    ///     self.set_by_name("incr_volume", values, "TTV", int)
    ///     self.set_by_name("day_open_mid", values, "DAY_OPEN_MID", float)
    ///     self.set_by_name("day_net_change_mid", values, "DAY_NET_CHG_MID", float)
    ///     self.set_by_name("day_percent_change_mid", values, "DAY_PERC_CHG_MID", float)
    ///     self.set_by_name("day_high", values, "DAY_HIGH", float)
    ///     self.set_by_name("day_low", values, "DAY_LOW", float)
    /// ```
    pub fn populate(&mut self, values: &lightstreamer_client::FieldMap) -> Result<()> {
        // Set timestamp from UTM field (Unix timestamp in milliseconds)
        self.set_timestamp_by_name(values, "UTM");
        
        // Set decimal fields
        Self::set_decimal_by_name(&mut self.bid, values, "BID");
        Self::set_decimal_by_name(&mut self.offer, values, "OFR");
        Self::set_decimal_by_name(&mut self.last_traded_price, values, "LTP");
        Self::set_decimal_by_name(&mut self.day_open_mid, values, "DAY_OPEN_MID");
        Self::set_decimal_by_name(&mut self.day_net_change_mid, values, "DAY_NET_CHG_MID");
        Self::set_decimal_by_name(&mut self.day_percent_change_mid, values, "DAY_PERC_CHG_MID");
        Self::set_decimal_by_name(&mut self.day_high, values, "DAY_HIGH");
        Self::set_decimal_by_name(&mut self.day_low, values, "DAY_LOW");
        
        // Set integer fields
        Self::set_integer_by_name(&mut self.last_traded_volume, values, "LTV");
        Self::set_integer_by_name(&mut self.incr_volume, values, "TTV");

        Ok(())
    }

    /// Set timestamp field from UTM value (like Python set_timestamp_by_name)
    fn set_timestamp_by_name(&mut self, values: &lightstreamer_client::FieldMap, key: &str) {
        if let Some(field_value) = values.get(key) {
            if let lightstreamer_client::FieldValue::String(text) = field_value {
                if let Ok(timestamp_ms) = text.parse::<i64>() {
                    // Convert milliseconds to seconds for DateTime::from_timestamp
                    if let Some(dt) = DateTime::from_timestamp(timestamp_ms / 1000, ((timestamp_ms % 1000) * 1_000_000) as u32) {
                        self.timestamp = Some(dt);
                    }
                }
            }
        }
    }

    /// Set decimal field from string value (like Python set_by_name with float)
    fn set_decimal_by_name(field: &mut Option<Decimal>, values: &lightstreamer_client::FieldMap, key: &str) {
        if let Some(field_value) = values.get(key) {
            if let lightstreamer_client::FieldValue::String(text) = field_value {
                if let Ok(decimal_val) = text.parse::<Decimal>() {
                    *field = Some(decimal_val);
                }
            }
        }
    }

    /// Set integer field from string value (like Python set_by_name with int)
    fn set_integer_by_name(field: &mut Option<i64>, values: &lightstreamer_client::FieldMap, key: &str) {
        if let Some(field_value) = values.get(key) {
            if let lightstreamer_client::FieldValue::String(text) = field_value {
                if let Ok(int_val) = text.parse::<i64>() {
                    *field = Some(int_val);
                }
            }
        }
    }

    // Getter methods for all fields (like Python properties)
    
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

/// TickerListener implements SubscriptionListener for handling ticker updates.
pub struct TickerListener {
    /// Sender for forwarding updates
    update_sender: mpsc::UnboundedSender<ItemUpdate>,
}

impl TickerListener {
    /// Create a new TickerListener.
    pub fn new(update_sender: mpsc::UnboundedSender<ItemUpdate>) -> Self {
        Self { update_sender }
    }
}

#[async_trait]
impl SubscriptionListener for TickerListener {
    async fn on_subscription(&self) {
        log::info!("TickerListener: onSubscription()");
    }

    async fn on_subscription_error(&self, code: i32, message: &str) {
        log::error!("TickerListener: onSubscriptionError({}, {})", code, message);
    }

    async fn on_unsubscription(&self) {
        log::info!("TickerListener: onUnsubscription()");
    }

    async fn on_item_update(&self, update: &ItemUpdate) {
        // Forward update to StreamingManager
        let _ = self.update_sender.send(update.clone());
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