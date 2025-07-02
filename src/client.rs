//! IG Markets client implementations.

use std::sync::Arc;
use std::time::{SystemTime, Duration, UNIX_EPOCH};
use reqwest::Client as HttpClient;
use serde_json::{json, Value};
use lightstreamer_client::{LightstreamerClient, ConnectionDetails, ConnectionOptions};
use crate::errors::{IGError, Result};
use crate::types::{AccountType, ApiVersion, OAuthToken};

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
    /// HTTP client for REST API calls
    http_client: HttpClient,
    /// Base URL for IG API (demo vs live)
    base_url: String,
    /// OAuth access token for v3 sessions
    access_token: Option<String>,
    /// Refresh token for v3 sessions (critical for session renewal)
    refresh_token: Option<String>,
    /// Token expiry time (Unix timestamp in seconds)
    valid_until: Option<u64>,
    /// CST token for Lightstreamer
    cst_token: Option<String>,
    /// X-SECURITY-TOKEN for Lightstreamer
    security_token: Option<String>,
    /// Lightstreamer endpoint
    lightstreamer_endpoint: Option<String>,
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

        // Determine base URL based on account type
        let base_url = match acc_type {
            AccountType::Demo => "https://demo-api.ig.com/gateway/deal".to_string(),
            AccountType::Live => "https://api.ig.com/gateway/deal".to_string(),
        };

        Ok(Self {
            username,
            password,
            api_key,
            account_type: acc_type,
            account_number: acc_number,
            http_client: HttpClient::new(),
            base_url,
            access_token: None,
            refresh_token: None,
            valid_until: None,
            cst_token: None,
            security_token: None,
            lightstreamer_endpoint: None,
        })
    }

    /// Check session status and refresh if needed (critical for v3 API).
    async fn check_session(&mut self) -> Result<()> {
        log::debug!("Checking session status...");
        
        // Check if token has expired (v3 tokens expire in 60 seconds)
        if let Some(valid_until) = self.valid_until {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
                
            if now >= valid_until {
                log::info!("Session token has expired, attempting refresh...");
                
                if let Some(_) = &self.refresh_token {
                    // Try to refresh the session
                    match self.refresh_session().await {
                        Ok(_) => {
                            log::info!("Session refreshed successfully");
                            return Ok(());
                        }
                        Err(e) => {
                            log::warn!("Session refresh failed: {}, re-authenticating...", e);
                            // Clear tokens and re-authenticate
                            self.refresh_token = None;
                            self.valid_until = None;
                            self.access_token = None;
                            return self.authenticate().await;
                        }
                    }
                } else {
                    log::info!("No refresh token available, re-authenticating...");
                    return self.authenticate().await;
                }
            }
        }
        
        Ok(())
    }

    /// Refresh the v3 session using the refresh token
    async fn refresh_session(&mut self) -> Result<()> {
        let refresh_token = self.refresh_token.as_ref()
            .ok_or_else(|| IGError::session("No refresh token available"))?;

        log::info!("Refreshing session for user '{}'", self.username);

        let refresh_payload = json!({
            "refresh_token": refresh_token
        });

        let response = self.http_client
            .post(&format!("{}/session/refresh-token", self.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json; charset=UTF-8")
            .header("X-IG-API-KEY", &self.api_key)
            .header("Version", "1")
            .json(&refresh_payload)
            .send()
            .await
            .map_err(|e| IGError::connection(format!("Refresh request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IGError::session(format!("Session refresh failed {}: {}", status, error_text)));
        }

        // Parse the OAuth response and update tokens
        let body: Value = response.json().await?;
        self.handle_oauth(&body)?;

        log::info!("Session refreshed successfully");
        Ok(())
    }

    /// Handle OAuth token response (both for initial auth and refresh).
    /// This replicates the Python _handle_oauth() method.
    fn handle_oauth(&mut self, oauth_body: &Value) -> Result<()> {
        let access_token = oauth_body.get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IGError::session("No access_token in OAuth response"))?;

        let token_type = oauth_body.get("token_type")
            .and_then(|v| v.as_str())
            .unwrap_or("Bearer");

        let refresh_token = oauth_body.get("refresh_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| IGError::session("No refresh_token in OAuth response"))?;

        let expires_in = oauth_body.get("expires_in")
            .and_then(|v| v.as_u64())
            .unwrap_or(60); // Default to 60 seconds for v3 tokens

        // Update tokens
        self.access_token = Some(access_token.to_string());
        self.refresh_token = Some(refresh_token.to_string());
        
        // Calculate expiry time
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.valid_until = Some(now + expires_in);

        log::debug!("Updated OAuth tokens - expires in {} seconds", expires_in);
        Ok(())
    }

    /// Authenticate with IG Markets and get session tokens (Version 3).
    pub async fn authenticate(&mut self) -> Result<()> {
        log::info!("Authenticating with IG Markets API...");

        // Always use Version 3 for v3 session tokens and Lightstreamer support
        let auth_payload = json!({
            "identifier": self.username,
            "password": self.password
        });

        let response = self.http_client
            .post(&format!("{}/session", self.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json; charset=UTF-8")
            .header("X-IG-API-KEY", &self.api_key)
            .header("Version", "3")  // Use Version 3 for OAuth + Lightstreamer
            .json(&auth_payload)
            .send()
            .await
            .map_err(|e| IGError::connection(format!("Authentication request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IGError::authentication(status.to_string(), error_text));
        }

        // Extract CST/Security tokens from headers for Lightstreamer compatibility
        if let Some(cst) = response.headers().get("CST") {
            self.cst_token = Some(cst.to_str().unwrap_or("").to_string());
        }
        if let Some(security) = response.headers().get("X-SECURITY-TOKEN") {
            self.security_token = Some(security.to_str().unwrap_or("").to_string());
        }

        // Parse response body to get OAuth tokens and lightstreamerEndpoint
        let body: Value = response.json().await?;
        log::debug!("IG Authentication response received");

        // Handle OAuth tokens for v3 sessions
        if let Some(oauth_token) = body.get("oauthToken") {
            self.handle_oauth(oauth_token)?;
        }

        // Extract lightstreamerEndpoint from IG response
        if let Some(endpoint) = body.get("lightstreamerEndpoint").and_then(|v| v.as_str()) {
            self.lightstreamer_endpoint = Some(endpoint.to_string());
            log::info!("Lightstreamer endpoint: {}", endpoint);
        }

        // For Version 3, fetch additional session tokens for Lightstreamer
        log::info!("Fetching session tokens for Lightstreamer authentication...");
        self.read_session_tokens().await?;

        log::info!("Successfully authenticated with IG Markets");
        Ok(())
    }

    /// Fetch CST and X-SECURITY-TOKEN for Lightstreamer authentication.
    async fn read_session_tokens(&mut self) -> Result<()> {
        // Note: No check_session() call here to avoid recursion

        let mut request = self.http_client
            .get(&format!("{}/session", self.base_url))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json; charset=UTF-8")
            .header("X-IG-API-KEY", &self.api_key)
            .header("Version", "1")
            .header("IG-ACCOUNT-ID", &self.account_number)
            .query(&[("fetchSessionTokens", "true")]);

        // Add OAuth authorization header if available
        if let Some(access_token) = &self.access_token {
            request = request.header("Authorization", format!("Bearer {}", access_token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| IGError::connection(format!("Read session request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IGError::session(format!("Read session failed {}: {}", status, error_text)));
        }

        // Extract CST and X-SECURITY-TOKEN from headers
        if let Some(cst) = response.headers().get("CST") {
            self.cst_token = Some(cst.to_str().unwrap_or("").to_string());
        }
        if let Some(security) = response.headers().get("X-SECURITY-TOKEN") {
            self.security_token = Some(security.to_str().unwrap_or("").to_string());
        }

        if self.cst_token.is_none() || self.security_token.is_none() {
            return Err(IGError::session("Failed to extract CST/X-SECURITY-TOKEN tokens"));
        }

        log::info!("Successfully fetched session tokens for Lightstreamer");
        Ok(())
    }

    /// Make an authenticated API request with automatic session refresh.
    /// This ensures tokens are always valid before making requests.
    pub async fn make_request(&mut self, method: &str, endpoint: &str, body: Option<Value>) -> Result<Value> {
        // Critical: Check and refresh session before every API call
        self.check_session().await?;

        let url = format!("{}{}", self.base_url, endpoint);
        let mut request = match method {
            "GET" => self.http_client.get(&url),
            "POST" => self.http_client.post(&url),
            "PUT" => self.http_client.put(&url),
            "DELETE" => self.http_client.delete(&url),
            _ => return Err(IGError::generic(format!("Unsupported HTTP method: {}", method))),
        };

        // Add standard headers
        request = request
            .header("Content-Type", "application/json")
            .header("Accept", "application/json; charset=UTF-8")
            .header("X-IG-API-KEY", &self.api_key)
            .header("IG-ACCOUNT-ID", &self.account_number);

        // Add OAuth authorization header for v3 sessions
        if let Some(access_token) = &self.access_token {
            request = request.header("Authorization", format!("Bearer {}", access_token));
        }

        // Add body for POST/PUT requests
        if let Some(body) = body {
            request = request.json(&body);
        }

        let response = request.send().await
            .map_err(|e| IGError::connection(format!("API request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(IGError::api_error(status.as_u16(), error_text));
        }

        let response_body: Value = response.json().await?;
        Ok(response_body)
    }

    /// Get the Lightstreamer endpoint.
    pub fn lightstreamer_endpoint(&self) -> Option<&str> {
        self.lightstreamer_endpoint.as_deref()
    }

    /// Get CST token for Lightstreamer authentication.
    pub fn cst_token(&self) -> Option<&str> {
        self.cst_token.as_deref()
    }

    /// Get security token for Lightstreamer authentication.
    pub fn security_token(&self) -> Option<&str> {
        self.security_token.as_deref()
    }

    /// Get account number.
    pub fn account_number(&self) -> &str {
        &self.account_number
    }

    /// Get OAuth access token for Version 3 authentication.
    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    /// Check if session is valid (not expired).
    pub fn is_session_valid(&self) -> bool {
        if let Some(valid_until) = self.valid_until {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            now < valid_until
        } else {
            false
        }
    }
}

/// IG Markets Lightstreamer streaming client.
pub struct IGStreamService {
    /// Reference to the IG REST service
    ig_service: Arc<IGService>,
    /// Lightstreamer client
    lightstreamer_client: Option<LightstreamerClient>,
    /// Whether the streaming session is active
    is_connected: bool,
}

impl IGStreamService {
    /// Create a new IGStreamService instance.
    pub fn new(ig_service: IGService) -> Result<Self> {
        Ok(Self {
            ig_service: Arc::new(ig_service),
            lightstreamer_client: None,
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

        // Get the Lightstreamer endpoint from the authenticated IG service
        let lightstreamer_endpoint = self.ig_service.lightstreamer_endpoint()
            .ok_or_else(|| IGError::session("No Lightstreamer endpoint available. Please authenticate first."))?;

        // Create Lightstreamer client
        let mut connection_details = ConnectionDetails::new();
        connection_details.set_server_address(lightstreamer_endpoint.to_string());
        connection_details.set_adapter_set(Some("DEFAULT".to_string())); // IG Markets adapter set
        
        // Set Lightstreamer authentication as per IG specification:
        // Username: Account number  
        // Password: "CST-{cst}|XST-{security_token}" (even for Version 3)
        
        // Set username as account number
        connection_details.set_user(Some(self.ig_service.account_number().to_string()));
        
        // Use CST/XST format for Lightstreamer authentication (both V2 and V3)
        if let (Some(cst_token), Some(security_token)) = (self.ig_service.cst_token(), self.ig_service.security_token()) {
            let ls_password = format!("CST-{}|XST-{}", cst_token, security_token);
            connection_details.set_password(Some(ls_password.clone()));
            log::info!("Using CST/XST tokens for Lightstreamer authentication");
            log::info!("  CST: {}...", &cst_token[..std::cmp::min(20, cst_token.len())]);
            log::info!("  XST: {}...", &security_token[..std::cmp::min(20, security_token.len())]);
        } else {
            return Err(IGError::session("Missing CST/Security tokens for Lightstreamer authentication"));
        }

        let lightstreamer_client = LightstreamerClient::new(
            Some(lightstreamer_endpoint.to_string()),
            Some("DEFAULT".to_string()), // IG Markets uses DEFAULT adapter set
        )?;

        // Set connection details
        *lightstreamer_client.connection_details.write().await = connection_details;

        // Connect to Lightstreamer
        log::info!("Connecting to Lightstreamer at: {}", lightstreamer_endpoint);
        lightstreamer_client.connect().await?;

        self.lightstreamer_client = Some(lightstreamer_client);
        self.is_connected = true;
        
        log::info!("Successfully connected to Lightstreamer");
        Ok(())
    }

    /// Check if the streaming session is connected.
    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// Get a reference to the Lightstreamer client.
    pub fn lightstreamer_client(&self) -> Option<&LightstreamerClient> {
        self.lightstreamer_client.as_ref()
    }
}
