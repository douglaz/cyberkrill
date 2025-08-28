//! Jade protocol implementation

use crate::error::{Error, Result};
use crate::messages::{Request, ResponseBody, error_codes, methods};
use crate::serial::SerialConnection;
use crate::types::Network;
use log::{debug, info};
use serde_json::{Value, json};

/// Low-level protocol handler for Jade communication
pub struct JadeProtocol {
    connection: SerialConnection,
    message_counter: u32,
}

impl JadeProtocol {
    /// Create new protocol handler with connection
    pub fn new(connection: SerialConnection) -> Self {
        Self {
            connection,
            message_counter: 0,
        }
    }

    /// Get next message ID
    fn next_id(&mut self) -> String {
        self.message_counter += 1;
        self.message_counter.to_string()
    }

    /// Send request and get response
    pub async fn call(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let id = self.next_id();
        let request = if let Some(params) = params {
            Request::with_params(id.clone(), method, params)
        } else {
            Request::new(id.clone(), method)
        };

        let response = self.connection.request(&request).await?;

        // Verify response ID matches request
        if response.id != id {
            return Err(Error::InvalidResponse);
        }

        // Handle response
        match response.body {
            ResponseBody::Result { result } => Ok(result),
            ResponseBody::Error { error } => {
                // Handle specific error codes
                match error.code {
                    error_codes::USER_CANCELLED | error_codes::USER_DECLINED => {
                        Err(Error::UserCancelled)
                    }
                    error_codes::HW_LOCKED => Err(Error::DeviceLocked),
                    _ => Err(Error::JadeError {
                        code: error.code,
                        message: error.message,
                    }),
                }
            }
        }
    }

    /// Get device version information
    pub async fn get_version_info(&mut self) -> Result<Value> {
        self.call(methods::GET_VERSION_INFO, None).await
    }

    /// Authenticate user with network
    pub async fn auth_user(&mut self, network: Network) -> Result<()> {
        info!("Starting auth_user for network: {network:?}");

        let params = json!({
            "network": network.as_jade_str()
        });

        // Send initial auth request
        let id = self.next_id();
        let request = Request::with_params(id.clone(), methods::AUTH_USER, params);
        debug!("Sending auth_user request with id: {id}");
        let response = self.connection.request(&request).await?;

        match response.body {
            ResponseBody::Result { result } => {
                // Check if already authenticated
                if let Some(true) = result.as_bool() {
                    info!("Device already authenticated");
                    return Ok(());
                }

                // If not a simple true, we need PIN server auth
                // For now, return an error indicating PIN server is needed
                #[cfg(not(feature = "pinserver"))]
                return Err(Error::Other(
                    "PIN authentication required but pinserver feature not enabled".to_string(),
                ));

                #[cfg(feature = "pinserver")]
                {
                    // The result contains the first HTTP request
                    info!("PIN authentication required, starting PIN server flow");
                    self.handle_pinserver_auth_with_initial(network, result, &id)
                        .await?;
                    Ok(())
                }
            }
            ResponseBody::Error { error } => match error.code {
                error_codes::USER_CANCELLED | error_codes::USER_DECLINED => {
                    Err(Error::UserCancelled)
                }
                error_codes::HW_LOCKED => Err(Error::DeviceLocked),
                _ => Err(Error::JadeError {
                    code: error.code,
                    message: error.message,
                }),
            },
        }
    }

    /// Logout from device
    pub async fn logout(&mut self) -> Result<()> {
        self.call(methods::LOGOUT, None).await?;
        Ok(())
    }

    /// Get extended public key
    pub async fn get_xpub(&mut self, path: &[u32], network: Network) -> Result<String> {
        let params = json!({
            "path": path,
            "network": network.as_jade_str()
        });

        let result = self.call(methods::GET_XPUB, Some(params)).await?;

        result
            .as_str()
            .map(String::from)
            .ok_or(Error::InvalidResponse)
    }

    /// Get receive address
    pub async fn get_receive_address(
        &mut self,
        network: Network,
        path: &[u32],
        variant: Option<&str>,
    ) -> Result<String> {
        let mut params = json!({
            "path": path,
            "network": network.as_jade_str()
        });

        if let Some(variant) = variant {
            params["variant"] = json!(variant);
        }

        let result = self
            .call(methods::GET_RECEIVE_ADDRESS, Some(params))
            .await?;

        result
            .as_str()
            .map(String::from)
            .ok_or(Error::InvalidResponse)
    }

    /// Sign a PSBT
    pub async fn sign_psbt(&mut self, network: Network, psbt_bytes: &[u8]) -> Result<Value> {
        // Encode PSBT as base64 for transmission
        let psbt_base64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, psbt_bytes);

        let params = json!({
            "network": network.as_jade_str(),
            "psbt": psbt_base64
        });

        self.call(methods::SIGN_PSBT, Some(params)).await
    }

    /// Sign a message
    pub async fn sign_message(
        &mut self,
        path: &[u32],
        message: &str,
        use_ae_protocol: bool,
    ) -> Result<String> {
        let params = json!({
            "path": path,
            "message": message,
            "use_ae_protocol": use_ae_protocol
        });

        let result = self.call(methods::SIGN_MESSAGE, Some(params)).await?;

        result
            .as_str()
            .map(String::from)
            .ok_or(Error::InvalidResponse)
    }

    #[cfg(feature = "pinserver")]
    async fn handle_pinserver_auth_with_initial(
        &mut self,
        _network: Network,
        initial_result: Value,
        auth_id: &str,
    ) -> Result<()> {
        use reqwest::Client;

        info!("Starting PIN server authentication");
        let client = Client::new();

        // Process the initial HTTP request from the auth_user response
        if let Some(http_req) = initial_result.get("http_request") {
            self.process_http_request(&client, http_req).await?;
        } else {
            return Err(Error::Other(
                "Expected http_request in auth response".to_string(),
            ));
        }

        // Continue processing any additional HTTP requests
        self.handle_pinserver_auth_loop(&client, auth_id).await
    }

    #[cfg(feature = "pinserver")]
    async fn handle_pinserver_auth_loop(
        &mut self,
        client: &reqwest::Client,
        auth_id: &str,
    ) -> Result<()> {
        loop {
            info!("Waiting for next message from Jade in PIN auth loop...");
            // Read next message from Jade
            let response = self.connection.receive_response().await?;

            info!(
                "Received response with id: {} (looking for: {})",
                response.id, auth_id
            );

            match response.body {
                ResponseBody::Result { result } => {
                    // Check if this is another HTTP request
                    if let Some(http_req) = result.get("http_request") {
                        info!("Received another HTTP request from Jade");
                        self.process_http_request(client, http_req).await?;
                        continue;
                    }

                    // If it's a boolean true, check if it's the final auth response
                    if let Some(true) = result.as_bool() {
                        if response.id == auth_id {
                            info!("Authentication successful!");
                            return Ok(());
                        } else {
                            info!(
                                "Jade acknowledged message with id: {}, continuing...",
                                response.id
                            );
                            // This might be the final success, wait a bit and return
                            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                            return Ok(());
                        }
                    }

                    debug!("Unexpected result in PIN auth loop: {result:?}");
                    continue;
                }
                ResponseBody::Error { error } => {
                    return Err(Error::JadeError {
                        code: error.code,
                        message: error.message,
                    });
                }
            }
        }
    }

    #[cfg(feature = "pinserver")]
    async fn process_http_request(
        &mut self,
        client: &reqwest::Client,
        http_req: &Value,
    ) -> Result<()> {
        // Extract the HTTP request parameters
        let params = http_req
            .get("params")
            .ok_or_else(|| Error::Other("Missing params in http_request".to_string()))?;

        let urls = params["urls"]
            .as_array()
            .ok_or_else(|| Error::Other("Missing urls in http_request".to_string()))?;

        let url = urls
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::Other("No URL provided".to_string()))?;

        let method = params["method"].as_str().unwrap_or("POST");
        let data = params.get("data");
        let on_reply = http_req
            .get("on-reply")
            .and_then(|v| v.as_str())
            .unwrap_or("pin");

        debug!("Making {method} request to {url}");

        // Make the HTTP request
        let http_response = if method == "POST" {
            let mut req = client.post(url);
            if let Some(data) = data {
                req = req.json(data);
            }
            req.send()
                .await
                .map_err(|e| Error::Other(format!("HTTP request failed: {e}")))?
        } else {
            client
                .get(url)
                .send()
                .await
                .map_err(|e| Error::Other(format!("HTTP request failed: {e}")))?
        };

        // Get response body as text
        let body = http_response
            .text()
            .await
            .map_err(|e| Error::Other(format!("Failed to read response: {e}")))?;

        debug!("PIN server response: {} bytes", body.len());

        // Parse response as JSON if possible
        let response_data = if let Ok(json) = serde_json::from_str::<Value>(&body) {
            json
        } else {
            json!({"body": body})
        };

        // Send response back to Jade with the correct method name
        let reply_id = self.next_id();
        let reply = Request::with_params(
            reply_id.clone(),
            on_reply, // Use the on-reply field as the method name
            response_data,
        );

        debug!("Sending {on_reply} reply to Jade with id: {reply_id}");
        // Don't wait for a response here, just send the reply
        self.connection.send_request(&reply).await?;

        Ok(())
    }
}
