//! IG Markets specific data types and structures.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// IG Markets account types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccountType {
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
            Self::Demo => "DEMO",
            Self::Live => "LIVE",
        }
    }
}

// Removed unused complex account and auth types - these add unnecessary complexity for basic streaming

// Removed extensive unused market data structures - these add complexity without benefit for basic streaming

// Removed unused Position and Order structures - not needed for basic streaming functionality

// Removed unused historical price data structures - not needed for real-time streaming

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
    pub last_traded_price: Option<Decimal>,
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
#[derive(Debug, Clone)]
pub struct SessionTokens {
    /// CST token for Lightstreamer authentication
    pub cst_token: String,
    /// X-SECURITY-TOKEN for Lightstreamer authentication
    pub security_token: String,
    /// Lightstreamer endpoint URL
    pub lightstreamer_endpoint: String,
}

/// OAuth token information for v3 sessions
#[derive(Debug, Clone)]
pub struct OAuthToken {
    /// Access token
    pub access_token: String,
    /// Refresh token for renewing expired access tokens
    pub refresh_token: String,
    /// Token type (usually "Bearer")
    pub token_type: String,
    /// Token expiry time in seconds
    pub expires_in: u64,
}

/// IG REST API version
#[derive(Debug, Clone, PartialEq, Eq)]
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