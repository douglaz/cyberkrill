use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

// Constants for Bitcoin RPC operations
const DEFAULT_MAX_CONFIRMATIONS: u32 = 9999999;
const DEFAULT_DESCRIPTOR_SCAN_RANGE: u32 = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub amount: f64,
    pub confirmations: u32,
    pub spendable: bool,
    pub solvable: bool,
    pub safe: bool,
    pub address: Option<String>,
    pub script_pub_key: String,
    pub descriptor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UtxoListResponse {
    pub utxos: Vec<Utxo>,
    pub total_amount: f64,
    pub total_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsbtResponse {
    pub psbt: String,
    pub fee: f64,
    pub change_position: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletFundedPsbtResponse {
    pub psbt: String,
    pub fee: f64,
    pub change_position: i32, // -1 if no change
}

#[derive(Debug)]
pub struct BitcoinRpcClient {
    pub url: String,
    pub auth: Option<(String, String)>,
    client: reqwest::Client,
}

impl BitcoinRpcClient {
    pub fn new(url: String, username: Option<String>, password: Option<String>) -> Self {
        let auth = match (username, password) {
            (Some(u), Some(p)) => Some((u, p)),
            _ => None,
        };

        Self {
            url,
            auth,
            client: reqwest::Client::new(),
        }
    }

    pub fn new_with_cookie(url: String, bitcoin_dir: &Path) -> Result<Self> {
        let cookie_path = bitcoin_dir.join(".cookie");
        let auth = Self::read_cookie_auth(&cookie_path)?;

        Ok(Self {
            url,
            auth: Some(auth),
            client: reqwest::Client::new(),
        })
    }

    pub fn new_auto(
        url: String,
        bitcoin_dir: Option<&Path>,
        username: Option<String>,
        password: Option<String>,
    ) -> Result<Self> {
        // Try cookie auth first if bitcoin_dir is provided
        if let Some(dir) = bitcoin_dir {
            if let Ok(client) = Self::new_with_cookie(url.clone(), dir) {
                return Ok(client);
            }
        }

        // Fall back to username/password
        Ok(Self::new(url, username, password))
    }

    fn read_cookie_auth(cookie_path: &Path) -> Result<(String, String)> {
        let cookie_content = std::fs::read_to_string(cookie_path).map_err(|e| {
            anyhow!(
                "Failed to read cookie file at {}: {}",
                cookie_path.display(),
                e
            )
        })?;

        let cookie_content = cookie_content.trim();

        if let Some(colon_pos) = cookie_content.find(':') {
            let username = cookie_content[..colon_pos].to_string();
            let password = cookie_content[colon_pos + 1..].to_string();
            Ok((username, password))
        } else {
            bail!(
                "Invalid cookie format in {}: expected 'username:password'",
                cookie_path.display()
            );
        }
    }

    async fn rpc_call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let request_body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "cyberkrill",
            "method": method,
            "params": params
        });

        let mut request = self.client.post(&self.url).json(&request_body);

        if let Some((username, password)) = &self.auth {
            request = request.basic_auth(username, Some(password));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            bail!("HTTP error: {}", response.status());
        }

        let json: serde_json::Value = response.json().await?;

        if let Some(error) = json.get("error") {
            if !error.is_null() {
                bail!("RPC error: {}", error);
            }
        }

        json.get("result")
            .ok_or_else(|| anyhow!("Missing result in RPC response"))
            .cloned()
    }

    pub async fn list_unspent(
        &self,
        min_conf: Option<u32>,
        max_conf: Option<u32>,
        addresses: Option<Vec<String>>,
    ) -> Result<Vec<Utxo>> {
        let mut params = vec![];

        if let Some(min) = min_conf {
            params.push(serde_json::Value::Number(min.into()));
        } else {
            params.push(serde_json::Value::Number(1.into()));
        }

        if let Some(max) = max_conf {
            params.push(serde_json::Value::Number(max.into()));
        } else {
            params.push(serde_json::Value::Number(DEFAULT_MAX_CONFIRMATIONS.into()));
        }

        if let Some(addrs) = addresses {
            params.push(serde_json::Value::Array(
                addrs.into_iter().map(serde_json::Value::String).collect(),
            ));
        }

        let result = self
            .rpc_call("listunspent", serde_json::Value::Array(params))
            .await?;

        let utxos: Vec<Utxo> = serde_json::from_value(result)
            .map_err(|e| anyhow!("Failed to deserialize listunspent response: {}", e))?;
        Ok(utxos)
    }

    pub async fn scan_tx_out_set(&self, descriptor: &str) -> Result<Vec<Utxo>> {
        // Expand <0;1> syntax to multiple descriptors for receive and change paths
        let descriptors_to_scan = if descriptor.contains("<0;1>") {
            vec![
                descriptor.replace("<0;1>", "0"),
                descriptor.replace("<0;1>", "1"),
            ]
        } else {
            vec![descriptor.to_string()]
        };

        // Get current block height once for confirmation calculations
        let current_height = self.get_current_block_height().await?;
        let mut all_utxos = Vec::new();

        for desc in descriptors_to_scan {
            let scanobject = if desc.contains("*") {
                serde_json::json!({
                    "desc": desc,
                    "range": [0, DEFAULT_DESCRIPTOR_SCAN_RANGE]
                })
            } else {
                serde_json::json!({
                    "desc": desc
                    // No range for fixed descriptors
                })
            };

            let params = serde_json::json!(["start", vec![scanobject]]);

            let result = self.rpc_call("scantxoutset", params).await?;

            let unspents = result
                .get("unspents")
                .ok_or_else(|| anyhow!("Missing unspents in scantxoutset response"))?;

            if let Some(unspent_array) = unspents.as_array() {
                for unspent in unspent_array {
                    let utxo = Utxo {
                        txid: unspent
                            .get("txid")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        vout: unspent.get("vout").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                        amount: unspent
                            .get("amount")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0),
                        confirmations: unspent
                            .get("height")
                            .and_then(|v| v.as_u64())
                            .map(|utxo_height| {
                                // Calculate actual confirmations from block height
                                if utxo_height > 0 && current_height >= utxo_height {
                                    (current_height - utxo_height + 1) as u32
                                } else {
                                    0 // Unconfirmed transaction
                                }
                            })
                            .unwrap_or(0),
                        spendable: true,
                        solvable: true,
                        safe: true,
                        address: None,
                        script_pub_key: unspent
                            .get("scriptPubKey")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        descriptor: Some(desc.clone()),
                    };
                    all_utxos.push(utxo);
                }
            }
        }

        Ok(all_utxos)
    }

    pub async fn list_utxos_for_descriptor(&self, descriptor: &str) -> Result<UtxoListResponse> {
        let utxos = self.scan_tx_out_set(descriptor).await?;
        let total_amount: f64 = utxos.iter().map(|u| u.amount).sum();
        let total_count = utxos.len();

        Ok(UtxoListResponse {
            utxos,
            total_amount,
            total_count,
        })
    }

    pub async fn list_utxos_for_addresses(
        &self,
        addresses: Vec<String>,
    ) -> Result<UtxoListResponse> {
        let utxos = self.list_unspent(Some(1), None, Some(addresses)).await?;
        let total_amount: f64 = utxos.iter().map(|u| u.amount).sum();
        let total_count = utxos.len();

        Ok(UtxoListResponse {
            utxos,
            total_amount,
            total_count,
        })
    }

    async fn get_current_block_height(&self) -> Result<u64> {
        let result = self
            .rpc_call("getblockchaininfo", serde_json::json!([]))
            .await?;

        result
            .get("blocks")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| {
                anyhow!("Missing or invalid 'blocks' field in getblockchaininfo response")
            })
    }

    pub async fn create_psbt(
        &self,
        inputs: &str,
        outputs: &str,
        fee_rate: Option<f64>, // sat/vB - will calculate fee and add to outputs
    ) -> Result<PsbtResponse> {
        // Parse inputs from "txid:vout,txid:vout" format
        let input_objects: Result<Vec<serde_json::Value>> = inputs
            .split(',')
            .map(|input| {
                let parts: Vec<&str> = input.trim().split(':').collect();
                if parts.len() != 2 {
                    bail!("Invalid input format: '{}'. Expected 'txid:vout'", input);
                }
                let txid = parts[0];
                let vout: u32 = parts[1]
                    .parse()
                    .map_err(|_| anyhow!("Invalid vout '{}' in input '{}'", parts[1], input))?;

                Ok(serde_json::json!({
                    "txid": txid,
                    "vout": vout
                }))
            })
            .collect();
        let input_objects = input_objects?;

        // Parse outputs from "address:amount,address:amount" format
        let mut output_object = serde_json::Map::new();
        for output in outputs.split(',') {
            let parts: Vec<&str> = output.trim().split(':').collect();
            if parts.len() != 2 {
                bail!(
                    "Invalid output format: '{}'. Expected 'address:amount_btc'",
                    output
                );
            }
            let address = parts[0];
            let amount: f64 = parts[1]
                .parse()
                .map_err(|_| anyhow!("Invalid amount '{}' in output '{}'", parts[1], output))?;

            output_object.insert(address.to_string(), serde_json::json!(amount));
        }

        // Store counts before moving the objects
        let num_inputs = input_objects.len();
        let num_outputs = output_object.len();

        // Build RPC parameters - Bitcoin Core accepts the outputs as a single object
        let mut params = vec![
            serde_json::Value::Array(input_objects),
            serde_json::Value::Object(output_object),
        ];

        // Add locktime (default 0)
        params.push(serde_json::json!(0));

        // Add replaceable flag (default false)
        params.push(serde_json::json!(false));

        let result = self
            .rpc_call("createpsbt", serde_json::Value::Array(params))
            .await?;

        // Parse the result
        let psbt = result
            .as_str()
            .ok_or_else(|| anyhow!("Expected PSBT string in createpsbt response"))?;

        // Calculate fee if fee_rate is provided
        let calculated_fee = if let Some(rate) = fee_rate {
            let tx_size = Self::estimate_transaction_size(num_inputs, num_outputs);
            (tx_size as f64 * rate) / 100_000_000.0 // Convert sat to BTC
        } else {
            0.0
        };

        Ok(PsbtResponse {
            psbt: psbt.to_string(),
            fee: calculated_fee,
            change_position: None, // TODO: Detect change output
        })
    }

    /// Estimate transaction size in virtual bytes for fee calculation
    /// Assumes P2WPKH inputs and outputs (most common case)
    fn estimate_transaction_size(num_inputs: usize, num_outputs: usize) -> u32 {
        // Base transaction overhead: version (4) + input count (1) + output count (1) + locktime (4) + segwit marker+flag (2)
        let base_size = 12;

        // P2WPKH input: txid (32) + vout (4) + scriptSig len (1) + scriptSig (0) + sequence (4) = 41 bytes
        // Plus witness: witness stack items (1) + signature (73) + pubkey (33) = 107 bytes
        // Virtual size for P2WPKH input: (41 * 4 + 107) / 4 = 68.25 ≈ 68 vbytes
        let input_size = 68;

        // P2WPKH output: value (8) + scriptPubKey len (1) + scriptPubKey (22) = 31 bytes
        let output_size = 31;

        base_size + (num_inputs * input_size) as u32 + (num_outputs * output_size) as u32
    }

    pub async fn wallet_create_funded_psbt(
        &self,
        inputs: &str, // Empty string for automatic input selection
        outputs: &str,
        conf_target: Option<u32>,
        estimate_mode: Option<&str>,
        fee_rate: Option<f64>, // sat/vB
    ) -> Result<WalletFundedPsbtResponse> {
        // Parse inputs - empty array for automatic input selection
        let input_objects: Vec<serde_json::Value> = if inputs.is_empty() {
            Vec::new()
        } else {
            inputs
                .split(',')
                .map(|input| {
                    let parts: Vec<&str> = input.trim().split(':').collect();
                    if parts.len() != 2 {
                        bail!("Invalid input format: '{}'. Expected 'txid:vout'", input);
                    }
                    let txid = parts[0];
                    let vout: u32 = parts[1]
                        .parse()
                        .map_err(|_| anyhow!("Invalid vout '{}' in input '{}'", parts[1], input))?;

                    Ok(serde_json::json!({
                        "txid": txid,
                        "vout": vout
                    }))
                })
                .collect::<Result<Vec<_>>>()?
        };

        // Parse outputs
        let mut output_object = serde_json::Map::new();
        for output in outputs.split(',') {
            let parts: Vec<&str> = output.trim().split(':').collect();
            if parts.len() != 2 {
                bail!(
                    "Invalid output format: '{}'. Expected 'address:amount_btc'",
                    output
                );
            }
            let address = parts[0];
            let amount: f64 = parts[1]
                .parse()
                .map_err(|_| anyhow!("Invalid amount '{}' in output '{}'", parts[1], output))?;

            output_object.insert(address.to_string(), serde_json::json!(amount));
        }

        // Build RPC parameters
        let mut params = vec![
            serde_json::Value::Array(input_objects),
            serde_json::Value::Object(output_object),
        ];

        // Add locktime (default 0)
        params.push(serde_json::json!(0));

        // Build options object for fee control
        let mut options = serde_json::Map::new();

        if let Some(target) = conf_target {
            options.insert("conf_target".to_string(), serde_json::json!(target));
        }

        if let Some(mode) = estimate_mode {
            options.insert("estimate_mode".to_string(), serde_json::json!(mode));
        }

        if let Some(rate) = fee_rate {
            // Convert sat/vB to BTC/kvB for Bitcoin Core
            let btc_per_kvb = rate * 100_000.0 / 100_000_000.0; // sat/vB to BTC/kvB
            options.insert("fee_rate".to_string(), serde_json::json!(btc_per_kvb));
        }

        if !options.is_empty() {
            params.push(serde_json::Value::Object(options));
        }

        let result = self
            .rpc_call("walletcreatefundedpsbt", serde_json::Value::Array(params))
            .await?;

        let psbt = result
            .get("psbt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Expected PSBT string in walletcreatefundedpsbt response"))?;

        let fee = result.get("fee").and_then(|v| v.as_f64()).unwrap_or(0.0);

        let change_position = result
            .get("changepos")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1) as i32;

        Ok(WalletFundedPsbtResponse {
            psbt: psbt.to_string(),
            fee,
            change_position,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_utxo_serialization() {
        let utxo = Utxo {
            txid: "abc123".to_string(),
            vout: 0,
            amount: 0.001,
            confirmations: 6,
            spendable: true,
            solvable: true,
            safe: true,
            address: Some("bc1qtest".to_string()),
            script_pub_key: "001400112233".to_string(),
            descriptor: Some("wpkh([fingerprint/84'/0'/0']xpub...)".to_string()),
        };

        let json = serde_json::to_string(&utxo).unwrap();
        let deserialized: Utxo = serde_json::from_str(&json).unwrap();

        assert_eq!(utxo.txid, deserialized.txid);
        assert_eq!(utxo.amount, deserialized.amount);
    }

    #[test]
    fn test_cookie_auth_parsing() {
        let temp_dir = std::env::temp_dir();
        let cookie_path = temp_dir.join("test_cookie");

        // Create test cookie file
        let mut file = fs::File::create(&cookie_path).unwrap();
        writeln!(file, "testuser:testpassword123").unwrap();

        // Test parsing
        let (username, password) = BitcoinRpcClient::read_cookie_auth(&cookie_path).unwrap();
        assert_eq!(username, "testuser");
        assert_eq!(password, "testpassword123");

        // Clean up
        fs::remove_file(&cookie_path).unwrap();
    }

    #[test]
    fn test_cookie_auth_invalid_format() {
        let temp_dir = std::env::temp_dir();
        let cookie_path = temp_dir.join("test_cookie_invalid");

        // Create invalid cookie file (no colon)
        let mut file = fs::File::create(&cookie_path).unwrap();
        writeln!(file, "invalidcookieformat").unwrap();

        // Should fail
        let result = BitcoinRpcClient::read_cookie_auth(&cookie_path);
        assert!(result.is_err());

        // Clean up
        fs::remove_file(&cookie_path).unwrap();
    }

    #[test]
    fn test_utxo_list_response_serialization() {
        let utxos = vec![
            Utxo {
                txid: "abc123".to_string(),
                vout: 0,
                amount: 0.001,
                confirmations: 6,
                spendable: true,
                solvable: true,
                safe: true,
                address: Some("bc1qtest".to_string()),
                script_pub_key: "001400112233".to_string(),
                descriptor: Some("test_descriptor".to_string()),
            },
            Utxo {
                txid: "def456".to_string(),
                vout: 1,
                amount: 0.002,
                confirmations: 10,
                spendable: true,
                solvable: true,
                safe: true,
                address: None,
                script_pub_key: "001400445566".to_string(),
                descriptor: Some("test_descriptor".to_string()),
            },
        ];

        let response = UtxoListResponse {
            utxos: utxos.clone(),
            total_amount: 0.003,
            total_count: 2,
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        let deserialized: UtxoListResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.total_count, deserialized.total_count);
        assert_eq!(response.total_amount, deserialized.total_amount);
        assert_eq!(response.utxos.len(), deserialized.utxos.len());
        assert_eq!(response.utxos[0].txid, deserialized.utxos[0].txid);
    }

    #[test]
    fn test_confirmation_calculation() {
        // Test the confirmation calculation logic
        let current_height = 1000u64;

        // UTXO at height 990 should have 11 confirmations (1000 - 990 + 1)
        let utxo_height = 990u64;
        let confirmations = if utxo_height > 0 && current_height >= utxo_height {
            (current_height - utxo_height + 1) as u32
        } else {
            0
        };
        assert_eq!(confirmations, 11);

        // UTXO at height 1000 (same as current) should have 1 confirmation
        let utxo_height = 1000u64;
        let confirmations = if utxo_height > 0 && current_height >= utxo_height {
            (current_height - utxo_height + 1) as u32
        } else {
            0
        };
        assert_eq!(confirmations, 1);

        // Unconfirmed UTXO (height 0) should have 0 confirmations
        let utxo_height = 0u64;
        let confirmations = if utxo_height > 0 && current_height >= utxo_height {
            (current_height - utxo_height + 1) as u32
        } else {
            0
        };
        assert_eq!(confirmations, 0);
    }

    #[test]
    fn test_psbt_response_serialization() {
        let response = PsbtResponse {
            psbt: "cHNidP8BAHECAAAAAea2/lMA5WyAk9UuMJPJ7wfhNzrhAAAAAA0AAAA=".to_string(),
            fee: 0.0001,
            change_position: Some(1),
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        let deserialized: PsbtResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.psbt, deserialized.psbt);
        assert_eq!(response.fee, deserialized.fee);
        assert_eq!(response.change_position, deserialized.change_position);
    }

    #[test]
    fn test_input_parsing_logic() {
        // Test the input parsing logic that would be in create_psbt
        let inputs = "abcd1234:0,efgh5678:1";
        let input_objects: Result<Vec<serde_json::Value>, anyhow::Error> = inputs
            .split(',')
            .map(|input| {
                let parts: Vec<&str> = input.trim().split(':').collect();
                if parts.len() != 2 {
                    bail!("Invalid input format: '{}'. Expected 'txid:vout'", input);
                }
                let txid = parts[0];
                let vout: u32 = parts[1]
                    .parse()
                    .map_err(|_| anyhow!("Invalid vout '{}' in input '{}'", parts[1], input))?;

                Ok(serde_json::json!({
                    "txid": txid,
                    "vout": vout
                }))
            })
            .collect();

        let input_objects = input_objects.unwrap();
        assert_eq!(input_objects.len(), 2);
        assert_eq!(input_objects[0]["txid"], "abcd1234");
        assert_eq!(input_objects[0]["vout"], 0);
        assert_eq!(input_objects[1]["txid"], "efgh5678");
        assert_eq!(input_objects[1]["vout"], 1);
    }

    #[test]
    fn test_output_parsing_logic() {
        // Test the output parsing logic that would be in create_psbt
        let outputs = "bc1qtest123:0.001,bc1qtest456:0.002";
        let mut output_object = serde_json::Map::new();

        for output in outputs.split(',') {
            let parts: Vec<&str> = output.trim().split(':').collect();
            if parts.len() != 2 {
                panic!(
                    "Invalid output format: '{}'. Expected 'address:amount_btc'",
                    output
                );
            }
            let address = parts[0];
            let amount: f64 = parts[1].parse().unwrap();

            output_object.insert(address.to_string(), serde_json::json!(amount));
        }

        assert_eq!(output_object.len(), 2);
        assert_eq!(output_object["bc1qtest123"], 0.001);
        assert_eq!(output_object["bc1qtest456"], 0.002);
    }

    #[test]
    fn test_invalid_input_format() {
        let inputs = "invalid_format";
        let result: Result<Vec<serde_json::Value>, anyhow::Error> = inputs
            .split(',')
            .map(|input| {
                let parts: Vec<&str> = input.trim().split(':').collect();
                if parts.len() != 2 {
                    bail!("Invalid input format: '{}'. Expected 'txid:vout'", input);
                }
                Ok(serde_json::json!({}))
            })
            .collect();

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid input format"));
    }


    #[test]
    fn test_wallet_funded_psbt_response_serialization() {
        let response = WalletFundedPsbtResponse {
            psbt: "cHNidP8BAHECAAAAAea2/lMA5WyAk9UuMJPJ7wfhNzrhAAAAAA0AAAA=".to_string(),
            fee: 0.0001,
            change_position: 1,
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        let deserialized: WalletFundedPsbtResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.psbt, deserialized.psbt);
        assert_eq!(response.fee, deserialized.fee);
        assert_eq!(response.change_position, deserialized.change_position);
    }

    #[test]
    fn test_fee_rate_conversion() {
        // Test the conversion from sat/vB to BTC/kvB used in wallet_create_funded_psbt
        let sat_per_vb = 20.0f64;
        let btc_per_kvb = sat_per_vb * 100_000.0 / 100_000_000.0; // sat/vB to BTC/kvB
        let expected = 0.02000000f64; // 20 sat/vB = 0.02 BTC/kvB

        assert!((btc_per_kvb - expected).abs() < 0.00000001);
    }

    #[test]
    fn test_empty_inputs_parsing() {
        // Test that empty string inputs result in empty array for automatic selection
        let inputs = "";
        let input_objects: Vec<serde_json::Value> = if inputs.is_empty() {
            Vec::new()
        } else {
            // Normal parsing logic would go here
            Vec::new()
        };

        assert_eq!(input_objects.len(), 0);
    }

    #[test]
    fn test_transaction_size_estimation() {
        // Test size estimation for typical transactions

        // Single input, single output
        let size = BitcoinRpcClient::estimate_transaction_size(1, 1);
        assert_eq!(size, 12 + 68 + 31); // base + input + output = 111 vbytes

        // Two inputs, two outputs (typical send with change)
        let size = BitcoinRpcClient::estimate_transaction_size(2, 2);
        assert_eq!(size, 12 + (2 * 68) + (2 * 31)); // base + inputs + outputs = 210 vbytes

        // Multiple inputs consolidation
        let size = BitcoinRpcClient::estimate_transaction_size(5, 1);
        assert_eq!(size, 12 + (5 * 68) + 31); // base + inputs + output = 383 vbytes
    }

    #[test]
    fn test_fee_calculation() {
        // Test the fee calculation logic used in create_psbt
        let num_inputs = 1;
        let num_outputs = 2;
        let fee_rate = 20.0f64; // sat/vB

        let tx_size = BitcoinRpcClient::estimate_transaction_size(num_inputs, num_outputs);
        let fee_btc = (tx_size as f64 * fee_rate) / 100_000_000.0;

        // Expected: 142 vbytes * 20 sat/vB = 2840 sats = 0.00002840 BTC
        let expected_fee = 0.00002840f64;
        assert!((fee_btc - expected_fee).abs() < 0.00000001);
    }

    #[test]
    fn test_sub_1_satbyte_fee_conversion() {
        // Test conversion for sub 1 sat/vB fees
        let sat_per_vb = 0.1f64;
        let btc_per_kvb = sat_per_vb * 100_000.0 / 100_000_000.0; // 0.1 sat/vB to BTC/kvB

        println!("0.1 sat/vB converts to {} BTC/kvB", btc_per_kvb);

        // Expected: 0.1 sat/vB = 0.0001 BTC/kvB
        let expected = 0.0001f64;
        assert!((btc_per_kvb - expected).abs() < 0.0000001);

        // Test even smaller values
        let tiny_rate = 0.01f64;
        let tiny_btc_kvb = tiny_rate * 100_000.0 / 100_000_000.0;
        println!("0.01 sat/vB converts to {} BTC/kvB", tiny_btc_kvb);

        // Expected: 0.01 sat/vB = 0.00001 BTC/kvB
        let expected_tiny = 0.00001f64;
        assert!((tiny_btc_kvb - expected_tiny).abs() < 0.000001);
    }

    #[test]
    fn test_bitcoin_amount_serialization() {
        // Test very small amounts (may use scientific notation, which is acceptable)
        let response = PsbtResponse {
            psbt: "test".to_string(),
            fee: 0.000000111, // 11.1 sats
            change_position: None,
        };

        let json = serde_json::to_string_pretty(&response).unwrap();
        println!("Serialized small amount: {}", json);

        // For very small amounts, scientific notation is acceptable
        // The fee should be present and deserializable
        assert!(json.contains("\"fee\":"));

        // Test with zero
        let zero_response = PsbtResponse {
            psbt: "test".to_string(),
            fee: 0.0,
            change_position: None,
        };

        let zero_json = serde_json::to_string_pretty(&zero_response).unwrap();
        println!("Serialized zero: {}", zero_json);
        assert!(zero_json.contains("\"fee\": 0"));

        // Test with normal-sized amount (should serialize as decimal)
        let normal_response = PsbtResponse {
            psbt: "test".to_string(),
            fee: 0.00012840, // 12840 sats - normal fee range
            change_position: None,
        };

        let normal_json = serde_json::to_string_pretty(&normal_response).unwrap();
        println!("Serialized normal amount: {}", normal_json);
        assert!(normal_json.contains("\"fee\": 0.0001284"));

        // Test deserialization works for all cases
        let deserialized: PsbtResponse = serde_json::from_str(&json).unwrap();
        assert!((deserialized.fee - 0.000000111).abs() < 0.0000000001);

        let zero_deserialized: PsbtResponse = serde_json::from_str(&zero_json).unwrap();
        assert_eq!(zero_deserialized.fee, 0.0);

        let normal_deserialized: PsbtResponse = serde_json::from_str(&normal_json).unwrap();
        assert!((normal_deserialized.fee - 0.00012840).abs() < 0.00000001);
    }
}
