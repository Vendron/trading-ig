//! IG Markets specific data types and structures.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// IG Markets account types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
    /// CFD account
    #[serde(rename = "CFD")]
    Cfd,
    /// Spread betting account
    #[serde(rename = "SPREADBET")]
    SpreadBet,
    /// Physical account
    #[serde(rename = "PHYSICAL")]
    Physical,
    /// Demo account
    #[serde(rename = "DEMO")]
    Demo,
    /// Live account
    #[serde(rename = "LIVE")]
    Live,
}

impl AccountType {
    /// Convert to string representation for API calls
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Cfd => "CFD",
            Self::SpreadBet => "SPREADBET", 
            Self::Physical => "PHYSICAL",
            Self::Demo => "DEMO",
            Self::Live => "LIVE",
        }
    }
}

/// Account information from IG Markets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Account ID
    pub account_id: String,
    /// Account name
    pub account_name: String,
    /// Account type
    pub account_type: AccountType,
    /// Whether this is the preferred account
    pub preferred: bool,
    /// Current balance
    pub balance: Option<AccountBalance>,
    /// Currency
    pub currency: String,
}

/// Account balance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountBalance {
    /// Available balance
    pub available: Decimal,
    /// Current balance
    pub balance: Decimal,
    /// Deposit amount
    pub deposit: Decimal,
    /// Profit/Loss
    pub profit_loss: Decimal,
}

/// Authentication response from IG Markets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    /// OAuth access token
    pub access_token: String,
    /// Refresh token
    pub refresh_token: String,
    /// Token scope
    pub scope: String,
    /// Token type (Bearer)
    pub token_type: String,
    /// Token expiry in seconds
    pub expires_in: u64,
    /// Account info
    pub account_info: Option<Account>,
    /// Lightstreamer endpoint
    pub lightstreamer_endpoint: Option<String>,
}

/// Market information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    /// Instrument details
    pub instrument: Instrument,
    /// Dealing rules
    pub dealing_rules: DealingRules,
    /// Market snapshot
    pub snapshot: MarketSnapshot,
}

/// Instrument information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instrument {
    /// Epic identifier
    pub epic: String,
    /// Expiry date
    pub expiry: Option<String>,
    /// Instrument name
    pub name: String,
    /// Lot size
    pub lot_size: Option<Decimal>,
    /// Instrument type
    pub instrument_type: String,
    /// Currency
    pub currencies: Vec<Currency>,
}

/// Currency information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Currency {
    /// Currency code
    pub code: String,
    /// Symbol
    pub symbol: String,
    /// Base exchange rate
    pub base_exchange_rate: Option<Decimal>,
    /// Exchange rate scale
    pub exchange_rate_scale: Option<i32>,
    /// Whether this is the default currency
    pub is_default: bool,
}

/// Dealing rules for an instrument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DealingRules {
    /// Minimum step distance
    pub min_step_distance: MinStepDistance,
    /// Minimum deal size
    pub min_deal_size: MinDealSize,
    /// Minimum controlled risk distance
    pub min_controlled_risk_distance: MinControlledRiskDistance,
    /// Minimum normal stop or limit distance
    pub min_normal_stop_or_limit_distance: MinNormalStopOrLimitDistance,
    /// Maximum stop or limit distance
    pub max_stop_or_limit_distance: MaxStopOrLimitDistance,
    /// Market orders allowed
    pub market_orders_allowed: bool,
    /// Trailing stops allowed
    pub trailing_stops_allowed: bool,
}

/// Minimum step distance rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinStepDistance {
    /// Unit
    pub unit: String,
    /// Value
    pub value: Decimal,
}

/// Minimum deal size rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinDealSize {
    /// Unit
    pub unit: String,
    /// Value
    pub value: Decimal,
}

/// Minimum controlled risk distance rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinControlledRiskDistance {
    /// Unit
    pub unit: String,
    /// Value
    pub value: Decimal,
}

/// Minimum normal stop or limit distance rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinNormalStopOrLimitDistance {
    /// Unit
    pub unit: String,
    /// Value
    pub value: Decimal,
}

/// Maximum stop or limit distance rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaxStopOrLimitDistance {
    /// Unit
    pub unit: String,
    /// Value
    pub value: Decimal,
}

/// Market snapshot data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketSnapshot {
    /// Market status
    pub market_status: String,
    /// Net change
    pub net_change: Decimal,
    /// Percentage change
    pub percentage_change: Decimal,
    /// Update time
    pub update_time: String,
    /// Delay time
    pub delay_time: i32,
    /// Bid price
    pub bid: Decimal,
    /// Offer price
    pub offer: Decimal,
    /// High price
    pub high: Decimal,
    /// Low price
    pub low: Decimal,
    /// Binary odds
    pub binary_odds: Option<Decimal>,
    /// Decimal places factor
    pub decimal_places_factor: i32,
    /// Scaling factor
    pub scaling_factor: i32,
    /// Controlled risk extra spread
    pub controlled_risk_extra_spread: Decimal,
}

/// Position information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    /// Position data
    pub position: PositionData,
    /// Market data
    pub market: Market,
}

/// Position data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionData {
    /// Contract size
    pub contract_size: Decimal,
    /// Created date
    pub created_date: String,
    /// Created date in UTC
    pub created_date_utc: String,
    /// Deal ID
    pub deal_id: String,
    /// Deal reference
    pub deal_reference: String,
    /// Deal size
    pub size: Decimal,
    /// Direction
    pub direction: String,
    /// Level
    pub level: Decimal,
    /// Limit level
    pub limit_level: Option<Decimal>,
    /// Stop level
    pub stop_level: Option<Decimal>,
    /// Trailing step
    pub trailing_step: Option<Decimal>,
    /// Trailing stop distance
    pub trailing_stop_distance: Option<Decimal>,
    /// Currency
    pub currency: String,
    /// Controlled risk
    pub controlled_risk: bool,
}

/// Order information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    /// Order data
    pub order: OrderData,
    /// Market data
    pub market: Market,
}

/// Order data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderData {
    /// Deal ID
    pub deal_id: String,
    /// Direction
    pub direction: String,
    /// Epic
    pub epic: String,
    /// Order level
    pub level: Decimal,
    /// Order size
    pub size: Decimal,
    /// Order status
    pub status: String,
    /// Order type
    pub order_type: String,
    /// Time in force
    pub time_in_force: String,
    /// Created date
    pub created_date: String,
    /// Good till date
    pub good_till_date: Option<String>,
    /// Guaranteed stop
    pub guaranteed_stop: bool,
    /// Limit distance
    pub limit_distance: Option<Decimal>,
    /// Stop distance
    pub stop_distance: Option<Decimal>,
    /// Currency code
    pub currency_code: String,
}

/// Price data for charts and historical data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    /// Snapshot time
    pub snapshot_time: String,
    /// Snapshot time in UTC
    pub snapshot_time_utc: String,
    /// Open price
    pub open_price: OpenPrice,
    /// Close price
    pub close_price: ClosePrice,
    /// High price
    pub high_price: HighPrice,
    /// Low price
    pub low_price: LowPrice,
    /// Last traded volume
    pub last_traded_volume: i64,
}

/// Open price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenPrice {
    /// Bid price
    pub bid: Decimal,
    /// Ask price
    pub ask: Decimal,
    /// Last traded price
    pub last_traded: Option<Decimal>,
}

/// Close price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClosePrice {
    /// Bid price
    pub bid: Decimal,
    /// Ask price
    pub ask: Decimal,
    /// Last traded price
    pub last_traded: Option<Decimal>,
}

/// High price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HighPrice {
    /// Bid price
    pub bid: Decimal,
    /// Ask price
    pub ask: Decimal,
    /// Last traded price
    pub last_traded: Option<Decimal>,
}

/// Low price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LowPrice {
    /// Bid price
    pub bid: Decimal,
    /// Ask price
    pub ask: Decimal,
    /// Last traded price
    pub last_traded: Option<Decimal>,
}

/// Streaming price update (from CHART:EPIC:TICK subscription)
#[derive(Debug, Clone)]
pub struct StreamingPrice {
    /// Epic identifier
    pub epic: String,
    /// Update timestamp
    pub timestamp: DateTime<Utc>,
    /// Bid price
    pub bid: Option<Decimal>,
    /// Ask/Offer price
    pub offer: Option<Decimal>,
    /// Last traded price
    pub last_traded: Option<Decimal>,
    /// Volume
    pub volume: Option<i64>,
    /// High price
    pub high: Option<Decimal>,
    /// Low price
    pub low: Option<Decimal>,
    /// Change amount
    pub change: Option<Decimal>,
    /// Change percentage
    pub change_pct: Option<Decimal>,
    /// Market status
    pub market_status: Option<String>,
}

/// Session tokens for Lightstreamer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionTokens {
    /// Current session ID
    pub current_session_id: String,
    /// Lightstreamer endpoint
    pub lightstreamer_endpoint: String,
    /// Active account ID
    pub active_account_id: String,
    /// Client ID
    pub client_id: String,
    /// Account type
    pub account_type: AccountType,
    /// Currency
    pub currency: String,
    /// Timezone offset
    pub timezone_offset: i32,
    /// Locale
    pub locale: String,
}

/// IG REST API version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiVersion {
    /// Version 1
    V1,
    /// Version 2
    V2,
    /// Version 3 (includes Lightstreamer session tokens)
    V3,
}

impl ApiVersion {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::V1 => "1",
            Self::V2 => "2", 
            Self::V3 => "3",
        }
    }
}

/// Request for creating a deal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDealRequest {
    /// Currency code
    pub currency_code: String,
    /// Direction (BUY/SELL)
    pub direction: String,
    /// Epic identifier
    pub epic: String,
    /// Expiry
    pub expiry: Option<String>,
    /// Force open
    pub force_open: bool,
    /// Guaranteed stop
    pub guaranteed_stop: bool,
    /// Order level for stop/limit orders
    pub level: Option<Decimal>,
    /// Limit distance
    pub limit_distance: Option<Decimal>,
    /// Limit level
    pub limit_level: Option<Decimal>,
    /// Quote ID for quote deals
    pub quote_id: Option<String>,
    /// Deal size
    pub size: Decimal,
    /// Stop distance
    pub stop_distance: Option<Decimal>,
    /// Stop level
    pub stop_level: Option<Decimal>,
    /// Time in force
    pub time_in_force: Option<String>,
    /// Deal type (MARKET/LIMIT/STOP)
    pub deal_type: String,
    /// Trailing stop
    pub trailing_stop: Option<bool>,
    /// Trailing stop distance
    pub trailing_stop_distance: Option<Decimal>,
    /// Trailing stop increment
    pub trailing_stop_increment: Option<Decimal>,
}

/// Response from creating a deal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDealResponse {
    /// Deal reference
    pub deal_reference: String,
    /// Deal status
    pub deal_status: String,
    /// Reason for rejection (if any)
    pub reason: Option<String>,
    /// Epic identifier
    pub epic: Option<String>,
    /// Expiry date
    pub expiry: Option<String>,
    /// Deal ID
    pub deal_id: Option<String>,
    /// Affected deals
    pub affected_deals: Option<Vec<AffectedDeal>>,
    /// Level
    pub level: Option<Decimal>,
    /// Size
    pub size: Option<Decimal>,
    /// Direction
    pub direction: Option<String>,
    /// Stop level
    pub stop_level: Option<Decimal>,
    /// Limit level
    pub limit_level: Option<Decimal>,
    /// Date
    pub date: Option<String>,
    /// Guaranteed stop
    pub guaranteed_stop: Option<bool>,
    /// Trailing stop
    pub trailing_stop: Option<bool>,
    /// Profit
    pub profit: Option<Decimal>,
    /// Profit currency
    pub profit_currency: Option<String>,
}

/// Affected deal information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedDeal {
    /// Deal ID
    pub deal_id: String,
    /// Status
    pub status: String,
}

/// Historical price resolution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    /// 1 second
    #[serde(rename = "SECOND")]
    Second,
    /// 1 minute
    #[serde(rename = "MINUTE")]
    Minute,
    /// 2 minutes
    #[serde(rename = "MINUTE_2")]
    Minute2,
    /// 3 minutes
    #[serde(rename = "MINUTE_3")]
    Minute3,
    /// 5 minutes
    #[serde(rename = "MINUTE_5")]
    Minute5,
    /// 10 minutes
    #[serde(rename = "MINUTE_10")]
    Minute10,
    /// 15 minutes
    #[serde(rename = "MINUTE_15")]
    Minute15,
    /// 30 minutes
    #[serde(rename = "MINUTE_30")]
    Minute30,
    /// 1 hour
    #[serde(rename = "HOUR")]
    Hour,
    /// 2 hours
    #[serde(rename = "HOUR_2")]
    Hour2,
    /// 3 hours
    #[serde(rename = "HOUR_3")]
    Hour3,
    /// 4 hours
    #[serde(rename = "HOUR_4")]
    Hour4,
    /// 1 day
    #[serde(rename = "DAY")]
    Day,
    /// 1 week
    #[serde(rename = "WEEK")]
    Week,
    /// 1 month
    #[serde(rename = "MONTH")]
    Month,
}

impl Resolution {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Second => "SECOND",
            Self::Minute => "MINUTE",
            Self::Minute2 => "MINUTE_2",
            Self::Minute3 => "MINUTE_3",
            Self::Minute5 => "MINUTE_5",
            Self::Minute10 => "MINUTE_10",
            Self::Minute15 => "MINUTE_15",
            Self::Minute30 => "MINUTE_30",
            Self::Hour => "HOUR",
            Self::Hour2 => "HOUR_2",
            Self::Hour3 => "HOUR_3",
            Self::Hour4 => "HOUR_4",
            Self::Day => "DAY",
            Self::Week => "WEEK",
            Self::Month => "MONTH",
        }
    }
} 