//! Error types for the IG Markets trading library.

use thiserror::Error;
use std::fmt;

pub type Result<T> = std::result::Result<T, IGError>;

/// Comprehensive error types for IG Markets operations
#[derive(Error, Debug)]
pub enum IGError {
    /// Lightstreamer client errors
    #[error("Lightstreamer error: {0}")]
    Lightstreamer(#[from] lightstreamer_client::LightstreamerError),

    /// HTTP client errors
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// JSON parsing errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing errors
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Authentication errors with IG-specific codes
    #[error("Authentication failed: {code} - {message}")]
    Authentication { code: String, message: String },

    /// API errors from IG server
    #[error("IG API error {code}: {message}")]
    Api { code: String, message: String },

    /// Rate limiting errors
    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    /// Invalid credentials
    #[error("Invalid credentials: {message}")]
    InvalidCredentials { message: String },

    /// Session errors
    #[error("Session error: {message}")]
    Session { message: String },

    /// Invalid market data
    #[error("Invalid market data: {message}")]
    InvalidMarketData { message: String },

    /// Trading errors
    #[error("Trading error: {message}")]
    Trading { message: String },

    /// Configuration errors
    #[error("Configuration error: {message}")]
    Configuration { message: String },

    /// Connection errors
    #[error("Connection error: {message}")]
    Connection { message: String },

    /// Generic errors
    #[error("Error: {message}")]
    Generic { message: String },

    /// Unsupported operation errors
    #[error("Unsupported operation: {message}")]
    Unsupported { message: String },
}

impl IGError {
    /// Create an authentication error
    pub fn authentication<C: Into<String>, M: Into<String>>(code: C, message: M) -> Self {
        Self::Authentication {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create an API error
    pub fn api<C: Into<String>, M: Into<String>>(code: C, message: M) -> Self {
        Self::Api {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Create a rate limit error
    pub fn rate_limit<M: Into<String>>(message: M) -> Self {
        Self::RateLimit {
            message: message.into(),
        }
    }

    /// Create an invalid credentials error
    pub fn invalid_credentials<M: Into<String>>(message: M) -> Self {
        Self::InvalidCredentials {
            message: message.into(),
        }
    }

    /// Create a session error
    pub fn session<M: Into<String>>(message: M) -> Self {
        Self::Session {
            message: message.into(),
        }
    }

    /// Create an invalid market data error
    pub fn invalid_market_data<M: Into<String>>(message: M) -> Self {
        Self::InvalidMarketData {
            message: message.into(),
        }
    }

    /// Create a trading error
    pub fn trading<M: Into<String>>(message: M) -> Self {
        Self::Trading {
            message: message.into(),
        }
    }

    /// Create a configuration error
    pub fn configuration<M: Into<String>>(message: M) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    /// Create a connection error
    pub fn connection<M: Into<String>>(message: M) -> Self {
        Self::Connection {
            message: message.into(),
        }
    }

    /// Create a generic error
    pub fn generic<M: Into<String>>(message: M) -> Self {
        Self::Generic {
            message: message.into(),
        }
    }

    /// Create an unsupported operation error
    pub fn unsupported<M: Into<String>>(message: M) -> Self {
        Self::Unsupported {
            message: message.into(),
        }
    }

    /// Create an API error with status code
    pub fn api_error(status_code: u16, message: String) -> Self {
        Self::Api {
            code: status_code.to_string(),
            message,
        }
    }

    /// Check if error is related to authentication
    pub fn is_authentication(&self) -> bool {
        matches!(
            self,
            Self::Authentication { .. } | Self::InvalidCredentials { .. }
        )
    }

    /// Check if error is related to rate limiting
    pub fn is_rate_limit(&self) -> bool {
        matches!(self, Self::RateLimit { .. })
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Http(e) => e.is_timeout() || e.is_connect(),
            Self::Connection { .. } => true,
            Self::RateLimit { .. } => true,
            _ => false,
        }
    }

    /// Get error code if available
    pub fn code(&self) -> Option<&str> {
        match self {
            Self::Authentication { code, .. } => Some(code),
            Self::Api { code, .. } => Some(code),
            _ => None,
        }
    }

    /// Get error message
    pub fn message(&self) -> &str {
        match self {
            Self::Authentication { message, .. } => message,
            Self::Api { message, .. } => message,
            Self::RateLimit { message } => message,
            Self::InvalidCredentials { message } => message,
            Self::Session { message } => message,
            Self::InvalidMarketData { message } => message,
            Self::Trading { message } => message,
            Self::Configuration { message } => message,
            Self::Connection { message } => message,
            Self::Generic { message } => message,
            Self::Unsupported { message } => message,
            _ => "Unknown error",
        }
    }
}