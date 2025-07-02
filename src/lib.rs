//! # Trading IG - Rust Implementation
//!
//! A comprehensive Rust implementation of the IG Markets trading API with Lightstreamer support.
//! This library provides both REST API access and real-time streaming capabilities for trading
//! and market data on the IG Markets platform.

pub mod client;
pub mod streaming;
pub mod types;
pub mod errors;
pub mod utils;

// Re-export main types for convenience
pub use client::{IGService, IGStreamService};
pub use streaming::{StreamingManager, TickerSubscription, Ticker};
pub use types::*;
pub use errors::{IGError, Result};

// Re-export lightstreamer types for convenience
pub use lightstreamer_client::{
    LightstreamerClient, ConnectionDetails, ConnectionOptions,
    Subscription, SubscriptionListener, ItemUpdate, ClientListener,
    SubscriptionMode, ClientStatus
};

pub const LIB_NAME: &str = "trading-ig";
pub const LIB_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lib_info() {
        assert_eq!(LIB_NAME, "trading-ig");
        assert!(!LIB_VERSION.is_empty());
    }
} 