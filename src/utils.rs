//! Utility functions and helpers for the IG Markets trading library.

use std::collections::HashMap;
use chrono::{DateTime, Utc, TimeZone};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

use crate::errors::{IGError, Result};
use crate::types::{AccountType, ApiVersion, Resolution};

/// IG Markets API endpoints
pub const BASE_URL_DEMO: &str = "https://demo-api.ig.com/gateway/deal";
pub const BASE_URL_LIVE: &str = "https://api.ig.com/gateway/deal";

/// Authentication endpoints
pub const AUTH_ENDPOINT: &str = "/session";
pub const REFRESH_ENDPOINT: &str = "/session/refresh-token";

/// Market data endpoints
pub const MARKETS_ENDPOINT: &str = "/markets";
pub const PRICES_ENDPOINT: &str = "/prices";

/// Account endpoints
pub const ACCOUNTS_ENDPOINT: &str = "/accounts";
pub const POSITIONS_ENDPOINT: &str = "/positions";
pub const ORDERS_ENDPOINT: &str = "/workingorders";

/// Trading endpoints
pub const DEALS_ENDPOINT: &str = "/positions/otc";

/// Session endpoints
pub const SESSION_ENDPOINT: &str = "/session";

/// Default headers for IG API requests
pub fn default_headers(version: ApiVersion) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_string(), "application/json".to_string());
    headers.insert("Accept".to_string(), "application/json".to_string());
    headers.insert("Version".to_string(), version.as_str().to_string());
    headers.insert("User-Agent".to_string(), format!("{}/{}", crate::LIB_NAME, crate::LIB_VERSION));
    headers
}

/// Add authentication headers to request
pub fn add_auth_headers(
    headers: &mut HashMap<String, String>,
    cst: Option<&str>,
    security_token: Option<&str>,
) {
    if let Some(cst_token) = cst {
        headers.insert("CST".to_string(), cst_token.to_string());
    }
    if let Some(x_security_token) = security_token {
        headers.insert("X-SECURITY-TOKEN".to_string(), x_security_token.to_string());
    }
}

/// Add API key header
pub fn add_api_key_header(headers: &mut HashMap<String, String>, api_key: &str) {
    headers.insert("X-IG-API-KEY".to_string(), api_key.to_string());
}

/// Build full URL for API endpoint
pub fn build_url(base_url: &str, endpoint: &str) -> String {
    format!("{}{}", base_url, endpoint)
}

/// Parse IG date format to DateTime<Utc>
pub fn parse_ig_date(date_str: &str) -> Result<DateTime<Utc>> {
    // IG uses various date formats, try common ones
    let formats = [
        "%Y-%m-%dT%H:%M:%S%.3f", // ISO with milliseconds
        "%Y-%m-%dT%H:%M:%S",     // ISO without milliseconds
        "%Y/%m/%d %H:%M:%S",     // Alternative format
        "%d/%m/%Y %H:%M:%S",     // UK format
    ];

    for format in &formats {
        if let Ok(naive_dt) = chrono::NaiveDateTime::parse_from_str(date_str, format) {
            return Ok(Utc.from_utc_datetime(&naive_dt));
        }
    }

    Err(IGError::invalid_market_data(format!("Unable to parse date: {}", date_str)))
}

/// Format DateTime for IG API requests
pub fn format_ig_date(date: &DateTime<Utc>) -> String {
    date.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Convert resolution to IG API format
pub fn resolution_to_ig_format(resolution: Resolution) -> &'static str {
    resolution.as_str()
}

/// Parse decimal from string, handling IG-specific formats
pub fn parse_decimal(value: &str) -> Result<Decimal> {
    if value.is_empty() || value == "null" || value == "N/A" {
        return Ok(Decimal::ZERO);
    }

    value.parse::<Decimal>()
        .map_err(|_| IGError::invalid_market_data(format!("Invalid decimal value: {}", value)))
}

/// Format decimal for IG API requests
pub fn format_decimal(value: &Decimal) -> String {
    value.to_string()
}

/// Calculate price from scaled value
pub fn scale_price(scaled_value: i64, scaling_factor: i32) -> Decimal {
    let scaling_divisor = 10_i64.pow(scaling_factor as u32);
    Decimal::new(scaled_value, scaling_factor as u32)
}

/// Scale price for API requests
pub fn unscale_price(price: &Decimal, scaling_factor: i32) -> i64 {
    let scaling_multiplier = 10_i64.pow(scaling_factor as u32);
    (price * Decimal::from(scaling_multiplier)).to_i64().unwrap_or(0)
}

/// Extract epic from item name (for CHART subscriptions)
pub fn extract_epic_from_item_name(item_name: &str) -> Option<String> {
    // CHART:EPIC:TICK format
    if item_name.starts_with("CHART:") && item_name.ends_with(":TICK") {
        let parts: Vec<&str> = item_name.split(':').collect();
        if parts.len() == 3 {
            return Some(parts[1].to_string());
        }
    }
    None
}

/// Build CHART subscription item name for an epic
pub fn build_chart_item_name(epic: &str) -> String {
    format!("CHART:{}:TICK", epic)
}

/// Validate epic format
pub fn validate_epic(epic: &str) -> Result<()> {
    if epic.is_empty() {
        return Err(IGError::invalid_market_data("Epic cannot be empty"));
    }

    // Basic validation - epics should not contain certain characters
    let invalid_chars = [' ', '\t', '\n', '\r', '|', ':'];
    if epic.chars().any(|c| invalid_chars.contains(&c)) {
        return Err(IGError::invalid_market_data("Epic contains invalid characters"));
    }

    Ok(())
}

/// Check if response indicates rate limiting
pub fn is_rate_limited(response_text: &str) -> bool {
    response_text.contains("exceeded-api-key-allowance")
        || response_text.contains("exceeded-account-allowance")
        || response_text.contains("exceeded-account-trading-allowance")
}

/// Check if response indicates invalid token
pub fn is_token_invalid(response_text: &str) -> bool {
    response_text.contains("oauth-token-invalid") 
        || response_text.contains("client-token-invalid")
}

/// Extract error code and message from IG API error response
pub fn parse_ig_error(response_text: &str) -> (String, String) {
    if let Ok(error_json) = serde_json::from_str::<serde_json::Value>(response_text) {
        let error_code = error_json
            .get("errorCode")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();
        
        let error_message = error_json
            .get("errorMessage")
            .and_then(|v| v.as_str())
            .or_else(|| error_json.get("message").and_then(|v| v.as_str()))
            .unwrap_or("Unknown error")
            .to_string();
        
        (error_code, error_message)
    } else {
        ("PARSE_ERROR".to_string(), response_text.to_string())
    }
}

/// Calculate position size based on risk parameters
pub fn calculate_position_size(
    account_balance: &Decimal,
    risk_percentage: &Decimal,
    entry_price: &Decimal,
    stop_loss_price: &Decimal,
    point_value: &Decimal,
) -> Result<Decimal> {
    if risk_percentage <= &Decimal::ZERO || risk_percentage > &Decimal::from(100) {
        return Err(IGError::trading("Risk percentage must be between 0 and 100"));
    }

    let risk_amount = account_balance * risk_percentage / Decimal::from(100);
    let price_difference = (entry_price - stop_loss_price).abs();
    
    if price_difference == Decimal::ZERO {
        return Err(IGError::trading("Entry price and stop loss cannot be the same"));
    }

    let position_size = risk_amount / (price_difference * point_value);
    Ok(position_size)
}

/// Convert account type to string for API requests
pub fn account_type_to_string(account_type: &AccountType) -> &'static str {
    account_type.as_str()
}

/// Generate correlation ID for request tracking
pub fn generate_correlation_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// Sanitize log messages to prevent injection
pub fn sanitize_log_message(message: &str) -> String {
    message
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
        .chars()
        .filter(|c| c.is_ascii_graphic() || c.is_ascii_whitespace())
        .collect()
}

/// Calculate percentage change
pub fn calculate_percentage_change(old_value: &Decimal, new_value: &Decimal) -> Decimal {
    if old_value == &Decimal::ZERO {
        return Decimal::ZERO;
    }
    ((new_value - old_value) / old_value) * Decimal::from(100)
}

/// Round price to appropriate decimal places based on instrument
pub fn round_price(price: &Decimal, decimal_places: u32) -> Decimal {
    price.round_dp(decimal_places)
}

/// Validate deal size against minimum requirements
pub fn validate_deal_size(size: &Decimal, min_size: &Decimal) -> Result<()> {
    if size < min_size {
        return Err(IGError::trading(format!(
            "Deal size {} is below minimum size {}",
            size, min_size
        )));
    }
    Ok(())
}

/// Calculate stop loss and take profit levels
pub fn calculate_levels(
    entry_price: &Decimal,
    direction: &str,
    stop_distance: Option<&Decimal>,
    limit_distance: Option<&Decimal>,
) -> (Option<Decimal>, Option<Decimal>) {
    let is_buy = direction.to_uppercase() == "BUY";
    
    let stop_level = stop_distance.map(|distance| {
        if is_buy {
            entry_price - distance
        } else {
            entry_price + distance
        }
    });
    
    let limit_level = limit_distance.map(|distance| {
        if is_buy {
            entry_price + distance
        } else {
            entry_price - distance
        }
    });
    
    (stop_level, limit_level)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_extract_epic_from_item_name() {
        assert_eq!(
            extract_epic_from_item_name("CHART:IX.D.SPTRD.MONTH1.IP:TICK"),
            Some("IX.D.SPTRD.MONTH1.IP".to_string())
        );
        assert_eq!(extract_epic_from_item_name("invalid"), None);
    }

    #[test]
    fn test_build_chart_item_name() {
        assert_eq!(
            build_chart_item_name("IX.D.SPTRD.MONTH1.IP"),
            "CHART:IX.D.SPTRD.MONTH1.IP:TICK"
        );
    }

    #[test]
    fn test_validate_epic() {
        assert!(validate_epic("IX.D.SPTRD.MONTH1.IP").is_ok());
        assert!(validate_epic("").is_err());
        assert!(validate_epic("invalid epic").is_err());
    }

    #[test]
    fn test_calculate_percentage_change() {
        let old_value = Decimal::from(100);
        let new_value = Decimal::from(110);
        let change = calculate_percentage_change(&old_value, &new_value);
        assert_eq!(change, Decimal::from(10));
    }
} 