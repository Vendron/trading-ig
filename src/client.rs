//! IG Markets client implementations.

use std::sync::Arc;
use lightstreamer_client::LightstreamerClient;
use crate::errors::{IGError, Result};
use crate::types::{AccountType, ApiVersion};

/// Main IG Markets REST API client.
pub struct IGService {
    /// Username for login
    username: String,
    /// Password for login
    password: String,
    /// API key for authentication
    api_key: String,
    /// Account type
    account_type: AccountType,
    /// Account number
    account_number: String,
}

impl IGService {
    /// Create a new IGService instance.
    pub fn new(
        username: String,
        password: String,
        api_key: String,
        acc_type: AccountType,
        acc_number: String,
    ) -> Result<Self> {
        // Validate inputs
        if username.is_empty() {
            return Err(IGError::configuration("Username cannot be empty"));
        }
        if password.is_empty() {
            return Err(IGError::configuration("Password cannot be empty"));
        }
        if api_key.is_empty() {
            return Err(IGError::configuration("API key cannot be empty"));
        }
        if acc_number.is_empty() {
            return Err(IGError::configuration("Account number cannot be empty"));
        }

        Ok(Self {
            username,
            password,
            api_key,
            account_type: acc_type,
            account_number: acc_number,
        })
    }

    /// Authenticate with IG Markets.
    pub async fn authenticate(&self) -> Result<()> {
        log::info!("Authenticating with IG Markets API");
        Ok(())
    }
}

/// IG Markets Lightstreamer streaming client.
pub struct IGStreamService {
    /// Reference to the IG REST service
    ig_service: Arc<IGService>,
    /// Whether the streaming session is active
    is_connected: bool,
}

impl IGStreamService {
    /// Create a new IGStreamService instance.
    pub fn new(ig_service: IGService) -> Result<Self> {
        Ok(Self {
            ig_service: Arc::new(ig_service),
            is_connected: false,
        })
    }

    /// Create a streaming session with the specified version.
    pub async fn create_session(&mut self, version: Option<ApiVersion>) -> Result<()> {
        let version_str = match version {
            Some(ApiVersion::V3) => "3",
            Some(ApiVersion::V2) => "2", 
            Some(ApiVersion::V1) => "1",
            None => "3", // Default to V3
        };

        if version_str != "3" {
            return Err(IGError::configuration("Only version 3 is supported"));
        }

        log::info!("Creating Lightstreamer session v{}", version_str);
        self.is_connected = true;
        Ok(())
    }

    /// Check if the streaming session is connected.
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }
}
