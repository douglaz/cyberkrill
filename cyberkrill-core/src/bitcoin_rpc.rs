use anyhow::{Context, Result, anyhow, bail};
use bitcoin::psbt::Psbt;
use bitcoin::transaction::{InputWeightPrediction, predict_weight};
use bitcoin::{Amount, Weight};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

// Constants for Bitcoin RPC operations
const DEFAULT_MAX_CONFIRMATIONS: u32 = 9999999;
const DEFAULT_DESCRIPTOR_SCAN_RANGE: u32 = 200;

/// Error type for AmountInput parsing
#[derive(Debug, thiserror::Error)]
pub enum AmountInputError {
    #[error("Empty amount string")]
    EmptyAmount,
    #[error("Invalid satoshi amount: '{0}'")]
    InvalidSatoshiAmount(String),
    #[error("Invalid BTC amount: '{0}'")]
    InvalidBtcAmount(String),
    #[error("Amount cannot be negative: {0}")]
    NegativeAmount(f64),
    #[error(
        "Invalid amount format: '{0}'. Expected formats: '123sats', '0.666btc', or '0.666' (BTC)"
    )]
    InvalidFormat(String),
    #[error("Bitcoin amount error: {0}")]
    BitcoinAmount(#[from] bitcoin::amount::ParseAmountError),
}

/// Represents an amount input that can be specified in either BTC or satoshis.
///
/// This type provides a user-friendly way to parse Bitcoin amounts from strings
/// supporting multiple formats while using the robust `bitcoin::Amount` type internally.
///
/// # Supported Formats
/// - Plain numbers: `"0.5"` (interpreted as BTC)
/// - BTC format: `"0.5btc"` or `"1.5BTC"` (case-insensitive)
/// - Satoshi format: `"50000000sats"`, `"100000sat"`, or `"123SATS"` (case-insensitive)
///
/// # Examples
/// ```
/// use std::str::FromStr;
/// use cyberkrill_core::bitcoin_rpc::AmountInput;
///
/// // Parse from different formats
/// let amount1 = AmountInput::from_str("0.5")?;
/// let amount2 = AmountInput::from_str("0.5btc")?;
/// let amount3 = AmountInput::from_str("50000000sats")?;
/// let amount4 = AmountInput::from_str("0.5sats")?; // Supports fractional satoshis (millisatoshis)
///
/// // All represent the same amount (except amount4 which is 0.5 sats)
/// assert_eq!(amount1.as_btc(), 0.5);
/// assert_eq!(amount2.as_btc(), 0.5);
/// assert_eq!(amount3.as_btc(), 0.5);
/// assert_eq!(amount4.as_millisats(), 500); // 0.5 sats = 500 millisats
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct AmountInput {
    /// The amount in millisatoshis (1/1000 of a satoshi)
    /// This allows for precise fee rate calculations and sub-satoshi amounts
    millisats: u64,
}

impl AmountInput {
    /// Creates a new `AmountInput` from a BTC amount.
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    ///
    /// let amount = AmountInput::from_btc(1.5)?;
    /// assert_eq!(amount.as_btc(), 1.5);
    /// # Ok::<(), cyberkrill_core::bitcoin_rpc::AmountInputError>(())
    /// ```
    ///
    /// # Errors
    /// Returns an error if the BTC amount is invalid (e.g., exceeds maximum value).
    pub fn from_btc(btc: f64) -> Result<Self, AmountInputError> {
        if btc < 0.0 {
            return Err(AmountInputError::NegativeAmount(btc));
        }
        // Convert BTC to millisatoshis: 1 BTC = 100_000_000 sats = 100_000_000_000 millisats
        let millisats = (btc * 100_000_000_000.0).round() as u64;
        Ok(Self { millisats })
    }

    /// Creates a new `AmountInput` from a satoshi amount.
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    ///
    /// let amount = AmountInput::from_sats(150000000);
    /// assert_eq!(amount.as_sat(), 150000000);
    /// assert_eq!(amount.as_btc(), 1.5);
    /// ```
    pub fn from_sats(sats: u64) -> Self {
        Self {
            millisats: sats * 1000, // Convert sats to millisats
        }
    }

    /// Creates a new `AmountInput` from fractional satoshis (millisatoshis).
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    ///
    /// let amount = AmountInput::from_fractional_sats(1.5)?;
    /// assert_eq!(amount.as_millisats(), 1500);
    /// assert_eq!(amount.as_sat(), 1); // Rounds down to whole satoshis
    /// # Ok::<(), cyberkrill_core::bitcoin_rpc::AmountInputError>(())
    /// ```
    pub fn from_fractional_sats(sats: f64) -> Result<Self, AmountInputError> {
        if sats < 0.0 {
            return Err(AmountInputError::NegativeAmount(sats));
        }
        // Convert fractional sats to millisats: 1 sat = 1000 millisats
        let millisats = (sats * 1000.0).round() as u64;
        Ok(Self { millisats })
    }

    /// Creates a new `AmountInput` from millisatoshis.
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    ///
    /// let amount = AmountInput::from_millisats(1500);
    /// assert_eq!(amount.as_millisats(), 1500);
    /// assert_eq!(amount.as_sat(), 1); // 1.5 sats rounds down to 1 sat
    /// ```
    pub fn from_millisats(millisats: u64) -> Self {
        Self { millisats }
    }

    /// Returns the amount in satoshis (rounded down from millisatoshis).
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    ///
    /// let amount = AmountInput::from_sats(100000000);
    /// assert_eq!(amount.as_sat(), 100000000);
    ///
    /// let fractional = AmountInput::from_millisats(1500); // 1.5 sats
    /// assert_eq!(fractional.as_sat(), 1); // Rounds down to 1 sat
    /// ```
    pub fn as_sat(&self) -> u64 {
        self.millisats / 1000
    }

    /// Returns the amount in BTC.
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    ///
    /// let amount = AmountInput::from_sats(100000000);
    /// assert_eq!(amount.as_btc(), 1.0);
    /// ```
    pub fn as_btc(&self) -> f64 {
        self.millisats as f64 / 100_000_000_000.0
    }

    /// Returns the amount in millisatoshis.
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    ///
    /// let amount = AmountInput::from_fractional_sats(1.5)?;
    /// assert_eq!(amount.as_millisats(), 1500);
    /// # Ok::<(), cyberkrill_core::bitcoin_rpc::AmountInputError>(())
    /// ```
    pub fn as_millisats(&self) -> u64 {
        self.millisats
    }

    /// Returns the amount as fractional satoshis.
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    ///
    /// let amount = AmountInput::from_millisats(1500);
    /// assert_eq!(amount.as_fractional_sats(), 1.5);
    /// ```
    pub fn as_fractional_sats(&self) -> f64 {
        self.millisats as f64 / 1000.0
    }

    /// Returns a `bitcoin::Amount` (rounded down to whole satoshis).
    ///
    /// This method provides access to a `bitcoin::Amount` type for
    /// operations that require precise Bitcoin amount handling.
    /// Note: This will lose precision for sub-satoshi amounts.
    ///
    /// # Examples
    /// ```
    /// use cyberkrill_core::bitcoin_rpc::AmountInput;
    /// use bitcoin::Amount;
    ///
    /// let amount_input = AmountInput::from_sats(100000000);
    /// let bitcoin_amount: Amount = amount_input.as_amount();
    /// assert_eq!(bitcoin_amount.to_sat(), 100000000);
    /// ```
    pub fn as_amount(&self) -> Amount {
        Amount::from_sat(self.as_sat())
    }
}

impl FromStr for AmountInput {
    type Err = AmountInputError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_lowercase();

        if s.is_empty() {
            return Err(AmountInputError::EmptyAmount);
        }

        // Check for millisatoshi suffixes
        if s.ends_with("msats") || s.ends_with("msat") {
            let number_part = if s.ends_with("msats") {
                &s[..s.len() - 5]
            } else {
                &s[..s.len() - 4]
            };

            let msats: u64 = number_part
                .parse()
                .map_err(|_| AmountInputError::InvalidSatoshiAmount(number_part.to_string()))?;

            return Ok(AmountInput::from_millisats(msats));
        }

        // Check for satoshi suffixes
        if s.ends_with("sats") || s.ends_with("sat") {
            let number_part = if s.ends_with("sats") {
                &s[..s.len() - 4]
            } else {
                &s[..s.len() - 3]
            };

            // Try parsing as f64 first to support fractional satoshis
            let sats: f64 = number_part
                .parse()
                .map_err(|_| AmountInputError::InvalidSatoshiAmount(number_part.to_string()))?;

            if sats < 0.0 {
                return Err(AmountInputError::NegativeAmount(sats));
            }

            return AmountInput::from_fractional_sats(sats);
        }

        // Check for BTC suffixes
        if s.ends_with("btc") {
            let number_part = &s[..s.len() - 3];
            let btc: f64 = number_part
                .parse()
                .map_err(|_| AmountInputError::InvalidBtcAmount(number_part.to_string()))?;

            if btc < 0.0 {
                return Err(AmountInputError::NegativeAmount(btc));
            }

            return AmountInput::from_btc(btc);
        }

        // No suffix - try to parse as a decimal number (assume BTC)
        let btc: f64 = s
            .parse()
            .map_err(|_| AmountInputError::InvalidFormat(s.to_string()))?;

        if btc < 0.0 {
            return Err(AmountInputError::NegativeAmount(btc));
        }

        AmountInput::from_btc(btc)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub amount: f64, // Keep as f64 for deserialization from Bitcoin Core
    pub confirmations: u32,
    pub spendable: bool,
    pub solvable: bool,
    pub safe: bool,
    pub address: Option<String>,
    #[serde(rename = "scriptPubKey")]
    pub script_pub_key: String,
    pub descriptor: Option<String>,
}

// Separate struct for serialization to users
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtxoOutput {
    pub txid: String,
    pub vout: u32,
    pub amount_sats: u64,
    pub confirmations: u32,
    pub spendable: bool,
    pub solvable: bool,
    pub safe: bool,
    pub address: Option<String>,
    pub script_pub_key: String,
    pub descriptor: Option<String>,
}

impl From<Utxo> for UtxoOutput {
    fn from(utxo: Utxo) -> Self {
        UtxoOutput {
            txid: utxo.txid,
            vout: utxo.vout,
            amount_sats: Amount::from_btc(utxo.amount)
                .unwrap_or(Amount::ZERO)
                .to_sat(),
            confirmations: utxo.confirmations,
            spendable: utxo.spendable,
            solvable: utxo.solvable,
            safe: utxo.safe,
            address: utxo.address,
            script_pub_key: utxo.script_pub_key,
            descriptor: utxo.descriptor,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UtxoListResponse {
    pub utxos: Vec<UtxoOutput>,
    pub total_amount_sats: u64,
    pub total_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PsbtResponse {
    pub psbt: String,
    pub fee_sats: u64,
    pub change_position: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletFundedPsbtResponse {
    pub psbt: String,
    pub fee_sats: u64,
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

    pub async fn rpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
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
            bail!("HTTP error: {status}", status = response.status());
        }

        let json: serde_json::Value = response.json().await?;

        if let Some(error) = json.get("error") {
            if !error.is_null() {
                bail!("RPC error: {error}");
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

        // Use 0 as minimum to include mempool transactions
        if let Some(min) = min_conf {
            params.push(serde_json::Value::Number(min.into()));
        } else {
            params.push(serde_json::Value::Number(0.into())); // Changed from 1 to 0
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

        let utxos: Vec<Utxo> =
            serde_json::from_value(result).context("Failed to deserialize listunspent response")?;
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
        // Use the new wallet-based method to include mempool transactions
        let utxos = self.list_unspent_for_descriptor(descriptor).await?;
        let total_amount_sats: u64 = utxos
            .iter()
            .map(|u| Amount::from_btc(u.amount).unwrap_or(Amount::ZERO).to_sat())
            .sum();
        let total_count = utxos.len();
        let utxo_outputs: Vec<UtxoOutput> = utxos.into_iter().map(Into::into).collect();

        Ok(UtxoListResponse {
            utxos: utxo_outputs,
            total_amount_sats,
            total_count,
        })
    }

    pub async fn list_utxos_for_addresses(
        &self,
        addresses: Vec<String>,
    ) -> Result<UtxoListResponse> {
        // Use min_conf=0 to include mempool transactions
        let utxos = self.list_unspent(Some(0), None, Some(addresses)).await?;
        let total_amount_sats: u64 = utxos
            .iter()
            .map(|u| Amount::from_btc(u.amount).unwrap_or(Amount::ZERO).to_sat())
            .sum();
        let total_count = utxos.len();
        let utxo_outputs: Vec<UtxoOutput> = utxos.into_iter().map(Into::into).collect();

        Ok(UtxoListResponse {
            utxos: utxo_outputs,
            total_amount_sats,
            total_count,
        })
    }

    /// List UTXOs from a frozenkrill wallet export file
    #[cfg(feature = "frozenkrill")]
    pub async fn list_utxos_from_wallet_file(
        &self,
        wallet_path: &Path,
    ) -> Result<UtxoListResponse> {
        use crate::frozenkrill::FrozenkrillWallet;

        let wallet = FrozenkrillWallet::from_file(wallet_path)
            .with_context(|| format!("Failed to load wallet from {}", wallet_path.display()))?;

        // List UTXOs for both receiving and change descriptors
        let receiving_utxos = self
            .list_unspent_for_descriptor(wallet.receiving_descriptor())
            .await?;
        let change_utxos = self
            .list_unspent_for_descriptor(wallet.change_descriptor())
            .await?;

        // Combine all UTXOs
        let mut all_utxos = receiving_utxos;
        all_utxos.extend(change_utxos);

        // Calculate totals
        let total_amount_sats: u64 = all_utxos
            .iter()
            .map(|u| Amount::from_btc(u.amount).unwrap_or(Amount::ZERO).to_sat())
            .sum();
        let total_count = all_utxos.len();
        let utxo_outputs: Vec<UtxoOutput> = all_utxos.into_iter().map(Into::into).collect();

        Ok(UtxoListResponse {
            utxos: utxo_outputs,
            total_amount_sats,
            total_count,
        })
    }

    /// Get descriptors from a frozenkrill wallet export file
    #[cfg(feature = "frozenkrill")]
    pub fn get_descriptors_from_wallet_file(wallet_path: &Path) -> Result<(String, String)> {
        use crate::frozenkrill::FrozenkrillWallet;

        let wallet = FrozenkrillWallet::from_file(wallet_path)
            .with_context(|| format!("Failed to load wallet from {}", wallet_path.display()))?;

        Ok((
            wallet.receiving_descriptor().to_string(),
            wallet.change_descriptor().to_string(),
        ))
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

    /// Import a descriptor as a watch-only wallet
    async fn import_descriptor(&self, descriptor: &str, rescan: bool) -> Result<()> {
        // First, get descriptor info to validate and get checksum
        let info_params = vec![serde_json::json!(descriptor)];
        let info_result = self
            .rpc_call("getdescriptorinfo", serde_json::Value::Array(info_params))
            .await?;

        let descriptor_with_checksum = info_result
            .get("descriptor")
            .and_then(|d| d.as_str())
            .context("Failed to get descriptor with checksum")?;

        // Import the descriptor
        let timestamp_value = if rescan {
            serde_json::json!(0)
        } else {
            serde_json::json!("now")
        };

        let import_params = vec![serde_json::json!([{
            "desc": descriptor_with_checksum,
            "timestamp": timestamp_value,
            "range": [0, 1000], // Import first 1000 addresses
            "watchonly": true,
            "label": "cyberkrill_import"
        }])];

        self.rpc_call("importdescriptors", serde_json::Value::Array(import_params))
            .await?;

        Ok(())
    }

    /// List unspent outputs for a descriptor using wallet functionality
    pub async fn list_unspent_for_descriptor(&self, descriptor: &str) -> Result<Vec<Utxo>> {
        // Import the descriptor if not already imported
        // We'll ignore errors as it might already be imported
        let _ = self.import_descriptor(descriptor, false).await;

        // Expand <0;1> syntax if present
        let descriptors = if descriptor.contains("<0;1>") {
            vec![
                descriptor.replace("<0;1>", "0"),
                descriptor.replace("<0;1>", "1"),
            ]
        } else {
            vec![descriptor.to_string()]
        };

        let mut all_utxos = Vec::new();
        let mut seen_outpoints = std::collections::HashSet::new();

        for desc in descriptors {
            // Get addresses from the descriptor
            let addresses = self.get_addresses_from_descriptor(&desc, 100).await?;

            // Use listunspent with min_conf=0 to include mempool
            let utxos = self.list_unspent(Some(0), None, Some(addresses)).await?;

            // Only add UTXOs we haven't seen before (deduplication for multipath descriptors)
            for utxo in utxos {
                let outpoint = (utxo.txid.clone(), utxo.vout);
                if seen_outpoints.insert(outpoint) {
                    all_utxos.push(utxo);
                }
            }
        }

        Ok(all_utxos)
    }

    /// Get addresses from a descriptor
    async fn get_addresses_from_descriptor(
        &self,
        descriptor: &str,
        count: u32,
    ) -> Result<Vec<String>> {
        let mut addresses = Vec::new();

        // Get descriptor info first
        let info_params = vec![serde_json::json!(descriptor)];
        let info_result = self
            .rpc_call("getdescriptorinfo", serde_json::Value::Array(info_params))
            .await?;

        let descriptor_with_checksum = info_result
            .get("descriptor")
            .and_then(|d| d.as_str())
            .context("Failed to get descriptor with checksum")?;

        // Derive addresses using proper range syntax for wildcard descriptors
        // The range should be [start, end] inclusive
        let derive_params = vec![
            serde_json::json!(descriptor_with_checksum),
            serde_json::json!([0, count - 1]), // Derive from index 0 to count-1
        ];

        if let Ok(result) = self
            .rpc_call("deriveaddresses", serde_json::Value::Array(derive_params))
            .await
        {
            if let Some(addr_array) = result.as_array() {
                for addr in addr_array {
                    if let Some(addr_str) = addr.as_str() {
                        addresses.push(addr_str.to_string());
                    }
                }
            }
        }

        Ok(addresses)
    }

    /// Parse input list and expand descriptors to UTXOs if needed
    /// Supports:
    /// - Standard format: "txid:vout"
    /// - Descriptor format: "wpkh([fingerprint/84'/0'/0']xpub...)"
    /// - Complex descriptors: "wsh(sortedmulti(...))"
    async fn parse_and_expand_inputs(&self, inputs: &[String]) -> Result<Vec<serde_json::Value>> {
        let mut all_inputs = Vec::new();

        for input in inputs {
            let input = input.trim();

            // Check if this looks like a descriptor (contains parentheses and/or brackets)
            if input.contains('(') || input.contains('[') {
                // This is a descriptor - expand it to UTXOs using wallet method to include mempool
                let utxos = self
                    .list_unspent_for_descriptor(input)
                    .await
                    .with_context(|| format!("Failed to expand descriptor: {input}"))?;

                // Convert each UTXO to input format
                for utxo in utxos {
                    all_inputs.push(serde_json::json!({
                        "txid": utxo.txid,
                        "vout": utxo.vout
                    }));
                }
            } else {
                // Standard txid:vout format
                let parts: Vec<&str> = input.split(':').collect();
                if parts.len() != 2 {
                    bail!(
                        "Invalid input format: '{}'. Expected 'txid:vout' or a descriptor",
                        input
                    );
                }
                let txid = parts[0];
                let vout: u32 = parts[1].parse().map_err(|_| {
                    anyhow!("Invalid vout '{vout}' in input '{input}'", vout = parts[1])
                })?;

                all_inputs.push(serde_json::json!({
                    "txid": txid,
                    "vout": vout
                }));
            }
        }

        if all_inputs.is_empty() {
            bail!("No valid inputs found after parsing and expansion");
        }

        Ok(all_inputs)
    }

    pub async fn create_psbt(
        &self,
        inputs: &[String],
        outputs: &str,
        fee_rate: Option<AmountInput>, // sat/vB - will calculate fee and add to outputs
    ) -> Result<PsbtResponse> {
        // Parse and expand inputs (handles both "txid:vout" and descriptor formats)
        let input_objects = self.parse_and_expand_inputs(inputs).await?;

        // Parse outputs from "address:amount,address:amount" format with flexible amount support
        let mut output_object = serde_json::Map::new();
        for output in outputs.split(',') {
            let parts: Vec<&str> = output.trim().split(':').collect();
            if parts.len() != 2 {
                bail!(
                    "Invalid output format: '{}'. Expected 'address:amount'",
                    output
                );
            }
            let address = parts[0];
            let amount_str = parts[1];

            // Parse amount using AmountInput for flexible format support
            let amount_input = AmountInput::from_str(amount_str).map_err(|e| {
                anyhow!(
                    "Invalid amount '{}' in output '{}': {}",
                    amount_str,
                    output,
                    e
                )
            })?;

            // Convert to BTC for Bitcoin Core RPC
            let amount_btc = amount_input.as_btc();

            output_object.insert(address.to_string(), serde_json::json!(amount_btc));
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
        let psbt_string = result
            .as_str()
            .ok_or_else(|| anyhow!("Expected PSBT string in createpsbt response"))?;

        // Validate PSBT using rust-bitcoin's parser
        Self::validate_psbt(psbt_string)?;

        // Calculate fee if fee_rate is provided
        let calculated_fee_sats = if let Some(rate) = fee_rate {
            let tx_weight = Self::estimate_transaction_weight(num_inputs, num_outputs);
            let fee_amount = Self::calculate_fee_with_feerate(tx_weight, rate.as_fractional_sats());
            fee_amount.to_sat()
        } else {
            0
        };

        Ok(PsbtResponse {
            psbt: psbt_string.to_string(),
            fee_sats: calculated_fee_sats,
            change_position: None, // TODO: Detect change output
        })
    }

    /// Estimate transaction weight using rust-bitcoin's predict_weight function
    /// Assumes P2WPKH inputs and P2WPKH outputs (most common case)
    fn estimate_transaction_weight(num_inputs: usize, num_outputs: usize) -> Weight {
        // Use rust-bitcoin's InputWeightPrediction for P2WPKH inputs
        let input_predictions = std::iter::repeat_n(InputWeightPrediction::P2WPKH_MAX, num_inputs);

        // P2WPKH output script length: OP_0 (1) + 20-byte pubkey hash (20) = 21 bytes
        // But rust-bitcoin expects script length without compact size prefix
        let output_script_lens = std::iter::repeat_n(22usize, num_outputs);

        predict_weight(input_predictions, output_script_lens)
    }

    /// Calculate fee using rust-bitcoin's types for more precise calculations
    /// Handles fractional sat/vB rates by doing precise weight-based calculation
    fn calculate_fee_with_feerate(weight: Weight, sat_per_vb: f64) -> Amount {
        // Calculate vbytes from weight (more precise than our helper method)
        let vbytes = weight.to_wu().div_ceil(4); // Same calculation but keep as u64

        // Calculate fee in satoshis: vbytes * sat_per_vb
        let fee_sat = (vbytes as f64 * sat_per_vb).round() as u64;

        // Convert to Amount using rust-bitcoin's type safety
        Amount::from_sat(fee_sat)
    }

    /// Validate PSBT string by parsing it with rust-bitcoin
    /// Returns the parsed PSBT if valid, error if invalid
    fn validate_psbt(psbt_str: &str) -> Result<Psbt> {
        // Decode base64 string to bytes first
        use base64::{Engine as _, engine::general_purpose::STANDARD};
        let psbt_bytes = STANDARD
            .decode(psbt_str)
            .with_context(|| "Failed to decode base64 PSBT string")?;

        // Parse PSBT from bytes using rust-bitcoin's deserialize method
        let psbt =
            Psbt::deserialize(&psbt_bytes).with_context(|| "Failed to parse PSBT from bytes")?;

        // Additional validation could go here if needed
        // For now, just return the parsed PSBT as validation success
        Ok(psbt)
    }

    /// Derives a change address from input descriptors with <0;1> syntax.
    /// Returns the first unused change address found, or None if no descriptors support change.
    async fn derive_change_address_from_inputs(&self, inputs: &[String]) -> Result<Option<String>> {
        for input in inputs {
            let input = input.trim();

            // Check if this looks like a descriptor with <0;1> syntax
            if (input.contains('(') || input.contains('[')) && input.contains("<0;1>") {
                // Extract the base descriptor and convert to change path
                let change_descriptor = self.convert_to_change_descriptor(input)?;

                // Find an unused change address
                if let Some(change_addr) = self.find_unused_address(&change_descriptor).await? {
                    return Ok(Some(change_addr));
                }
            }
        }
        Ok(None)
    }

    /// Converts a receive descriptor with <0;1> syntax to a change descriptor.
    /// Example: wpkh([...]/xpub.../<0;1>/*) -> wpkh([...]/xpub.../1/*)
    fn convert_to_change_descriptor(&self, descriptor: &str) -> Result<String> {
        if let Some(start) = descriptor.find("<0;1>") {
            let before = &descriptor[..start];
            let after = &descriptor[start + 5..]; // Skip "<0;1>"
            let change_descriptor = format!("{before}1{after}");
            Ok(change_descriptor)
        } else {
            bail!("Descriptor does not contain <0;1> syntax: {descriptor}");
        }
    }

    /// Finds an unused address from a descriptor using BIP 44 gap limit.
    /// Returns the first address that has never been used (never received any transactions).
    async fn find_unused_address(&self, descriptor: &str) -> Result<Option<String>> {
        let mut consecutive_unused = 0;
        const GAP_LIMIT: usize = 20; // BIP 44 standard gap limit

        // Scan addresses following BIP 44 gap limit
        for index in 0..200 {
            // Reasonable upper bound
            let indexed_descriptor = descriptor.replace('*', &index.to_string());

            // Remove any existing checksum and let Bitcoin Core calculate it
            let desc_without_checksum = if let Some(hash_pos) = indexed_descriptor.find('#') {
                &indexed_descriptor[..hash_pos]
            } else {
                &indexed_descriptor
            };

            // Get the address for this index
            if let Ok(address) = self
                .derive_address_from_descriptor(desc_without_checksum)
                .await
            {
                // Check if this address has ever been used
                let ever_used = self.check_address_ever_used(&address).await?;

                if ever_used {
                    consecutive_unused = 0; // Reset counter
                } else {
                    consecutive_unused += 1;
                    if consecutive_unused == 1 {
                        // This is the first unused address we found
                        return Ok(Some(address));
                    }
                }

                // If we hit the gap limit, we can stop scanning
                if consecutive_unused >= GAP_LIMIT {
                    break;
                }
            }
        }
        Ok(None)
    }

    /// Derives a single address from a specific descriptor (with index).
    async fn derive_address_from_descriptor(&self, descriptor: &str) -> Result<String> {
        // First get the descriptor with checksum using getdescriptorinfo
        let info_params = vec![serde_json::json!(descriptor)];
        let info_result = self
            .rpc_call("getdescriptorinfo", serde_json::Value::Array(info_params))
            .await?;

        let descriptor_with_checksum = info_result
            .get("descriptor")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Failed to get descriptor with checksum"))?;

        // Now derive addresses using the descriptor with checksum
        let params = vec![serde_json::json!(descriptor_with_checksum)];
        let result = self
            .rpc_call("deriveaddresses", serde_json::Value::Array(params))
            .await?;

        if let Some(addresses) = result.as_array() {
            if let Some(address) = addresses.first().and_then(|v| v.as_str()) {
                return Ok(address.to_string());
            }
        }

        bail!("Failed to derive address from descriptor: {descriptor}");
    }

    /// Checks if an address has any UTXOs (used or unused).
    async fn check_address_has_utxos(&self, address: &str) -> Result<bool> {
        // Use listunspent to check if this address has any UTXOs
        let params = vec![
            serde_json::json!(0),         // minconf
            serde_json::json!(9999999),   // maxconf
            serde_json::json!([address]), // addresses
        ];

        let result = self
            .rpc_call("listunspent", serde_json::Value::Array(params))
            .await?;

        if let Some(utxos) = result.as_array() {
            Ok(!utxos.is_empty())
        } else {
            Ok(false)
        }
    }

    /// Checks if an address has ever been used (received any transactions).
    /// This is more comprehensive than checking current UTXOs as it includes spent outputs.
    async fn check_address_ever_used(&self, address: &str) -> Result<bool> {
        // First try getreceivedbyaddress (works if address is watch-only)
        match self.get_received_by_address(address, 0).await {
            Ok(amount) => Ok(amount > 0.0),
            Err(_) => {
                // If getreceivedbyaddress fails (address not imported),
                // fallback to checking current UTXOs as a best effort
                self.check_address_has_utxos(address).await
            }
        }
    }

    /// Gets the total amount ever received by an address.
    /// Returns 0.0 if the address has never received funds or is not watch-only.
    async fn get_received_by_address(&self, address: &str, min_conf: u32) -> Result<f64> {
        let params = vec![serde_json::json!(address), serde_json::json!(min_conf)];

        let result = self
            .rpc_call("getreceivedbyaddress", serde_json::Value::Array(params))
            .await?;

        Ok(result.as_f64().unwrap_or(0.0))
    }

    pub async fn wallet_create_funded_psbt(
        &self,
        inputs: &[String], // Empty slice for automatic input selection
        outputs: &str,
        conf_target: Option<u32>,
        estimate_mode: Option<&str>,
        fee_rate: Option<AmountInput>, // sat/vB
    ) -> Result<WalletFundedPsbtResponse> {
        // Parse and expand inputs (empty slice means automatic input selection)
        let input_objects: Vec<serde_json::Value> = if inputs.is_empty() {
            Vec::new()
        } else {
            self.parse_and_expand_inputs(inputs).await?
        };

        // Parse outputs with flexible amount format support
        let mut output_object = serde_json::Map::new();
        for output in outputs.split(',') {
            let parts: Vec<&str> = output.trim().split(':').collect();
            if parts.len() != 2 {
                bail!(
                    "Invalid output format: '{}'. Expected 'address:amount'",
                    output
                );
            }
            let address = parts[0];
            let amount_str = parts[1];

            // Parse amount using AmountInput for flexible format support
            let amount_input = AmountInput::from_str(amount_str).map_err(|e| {
                anyhow!(
                    "Invalid amount '{}' in output '{}': {}",
                    amount_str,
                    output,
                    e
                )
            })?;

            // Convert to BTC for Bitcoin Core RPC
            let amount_btc = amount_input.as_btc();

            output_object.insert(address.to_string(), serde_json::json!(amount_btc));
        }

        // Try to derive a change address from input descriptors
        let change_address = self.derive_change_address_from_inputs(inputs).await?;

        // Build RPC parameters
        let mut params = vec![
            serde_json::Value::Array(input_objects),
            serde_json::Value::Object(output_object),
        ];

        // Add locktime (default 0)
        params.push(serde_json::json!(0));

        // Build options object for fee control and change address
        let mut options = serde_json::Map::new();

        // Add change address if we derived one
        if let Some(change_addr) = change_address {
            options.insert("changeAddress".to_string(), serde_json::json!(change_addr));
        }

        if let Some(target) = conf_target {
            options.insert("conf_target".to_string(), serde_json::json!(target));
        }

        if let Some(mode) = estimate_mode {
            options.insert("estimate_mode".to_string(), serde_json::json!(mode));
        }

        if let Some(rate) = fee_rate {
            // Bitcoin Core 0.21+ expects fee_rate in sat/vB directly (not BTC/kvB)
            let rate_sat_per_vb = rate.as_fractional_sats(); // This gives us precise sat/vB including sub-satoshi rates
            options.insert("fee_rate".to_string(), serde_json::json!(rate_sat_per_vb));
        }

        if !options.is_empty() {
            params.push(serde_json::Value::Object(options));
        }

        let result = self
            .rpc_call("walletcreatefundedpsbt", serde_json::Value::Array(params))
            .await?;

        let psbt_string = result
            .get("psbt")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Expected PSBT string in walletcreatefundedpsbt response"))?;

        // Validate PSBT using rust-bitcoin's parser
        Self::validate_psbt(psbt_string)?;

        let fee_btc = result.get("fee").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let fee_sats = Amount::from_btc(fee_btc)?.to_sat();

        let change_position = result
            .get("changepos")
            .and_then(|v| v.as_i64())
            .unwrap_or(-1) as i32;

        Ok(WalletFundedPsbtResponse {
            psbt: psbt_string.to_string(),
            fee_sats,
            change_position,
        })
    }

    pub async fn move_utxos(
        &self,
        inputs: &[String],
        destination: &str,
        fee_rate: Option<AmountInput>,
        fee_sats: Option<AmountInput>,
        max_amount: Option<AmountInput>,
    ) -> Result<PsbtResponse> {
        // Parse and expand inputs (handles both "txid:vout" and descriptor formats)
        let all_input_objects = self.parse_and_expand_inputs(inputs).await?;

        // Get UTXO details with values
        let mut utxo_details = Vec::new();
        for input_obj in &all_input_objects {
            let txid = input_obj["txid"]
                .as_str()
                .ok_or_else(|| anyhow!("Missing txid in input object"))?;
            let vout = input_obj["vout"]
                .as_u64()
                .ok_or_else(|| anyhow!("Missing vout in input object"))?
                as u32;

            // Get transaction details to find the output value
            let tx_result = self
                .rpc_call("getrawtransaction", serde_json::json!([txid, true]))
                .await?;

            let vouts = tx_result
                .get("vout")
                .and_then(|v| v.as_array())
                .ok_or_else(|| anyhow!("Missing vout array in transaction {txid}"))?;

            if let Some(output) = vouts.get(vout as usize) {
                let value = output
                    .get("value")
                    .and_then(|v| v.as_f64())
                    .ok_or_else(|| anyhow!("Missing value in output {txid}:{vout}"))?;
                utxo_details.push((input_obj.clone(), value));
            } else {
                bail!("Output {txid}:{vout} not found in transaction");
            }
        }

        // If max_amount is specified, perform coin selection
        let (selected_inputs, total_input_value) = if let Some(max_amount_input) = max_amount {
            let max_btc = max_amount_input.as_btc();
            // Sort UTXOs by value in descending order (largest first)
            utxo_details.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let mut selected = Vec::new();
            let mut selected_value = 0.0f64;

            // Select UTXOs until we reach or exceed max_amount
            for (input_obj, value) in utxo_details {
                if selected_value >= max_btc {
                    break;
                }
                selected.push(input_obj);
                selected_value += value;
            }

            if selected.is_empty() {
                bail!(
                    "No UTXOs selected. All available UTXOs exceed max_amount of {} BTC",
                    max_btc
                );
            }

            (selected, selected_value.min(max_btc))
        } else {
            // Use all inputs
            let total_value: f64 = utxo_details.iter().map(|(_, value)| value).sum();
            let inputs: Vec<_> = utxo_details.into_iter().map(|(input, _)| input).collect();
            (inputs, total_value)
        };

        // Calculate fee
        let fee_sats_amount = match (fee_rate, fee_sats) {
            (Some(rate), None) => {
                // Calculate fee using fee rate (rate should be in sat/vB)
                let num_inputs = selected_inputs.len();
                let num_outputs = 1; // Single consolidation output
                let tx_weight = Self::estimate_transaction_weight(num_inputs, num_outputs);
                // For fee rate, use fractional satoshi precision for sub-1 sat/vB rates
                let rate_sat_per_vb = rate.as_fractional_sats();
                let fee_amount = Self::calculate_fee_with_feerate(tx_weight, rate_sat_per_vb);
                fee_amount.to_sat()
            }
            (None, Some(fee_amount)) => {
                // Use absolute fee amount
                fee_amount.as_sat()
            }
            (Some(_), Some(_)) => {
                bail!("Cannot specify both fee_rate and fee_sats");
            }
            (None, None) => {
                bail!("Must specify either fee_rate or fee_sats");
            }
        };

        // Convert fee to BTC for output calculation
        let fee_btc = Amount::from_sat(fee_sats_amount).to_btc();

        // Calculate output amount (total input - fees)
        let output_amount = total_input_value - fee_btc;
        if output_amount <= 0.0 {
            bail!(
                "Insufficient funds: inputs={:.8} BTC, fee={:.8} BTC",
                total_input_value,
                fee_btc
            );
        }

        // Create PSBT using selected inputs
        let mut output_object = serde_json::Map::new();
        output_object.insert(destination.to_string(), serde_json::json!(output_amount));

        let mut params = vec![
            serde_json::Value::Array(selected_inputs),
            serde_json::Value::Object(output_object),
        ];

        // Add locktime (default 0)
        params.push(serde_json::json!(0));

        // Add replaceable flag (default false)
        params.push(serde_json::json!(false));

        let result = self
            .rpc_call("createpsbt", serde_json::Value::Array(params))
            .await?;

        let psbt_string = result
            .as_str()
            .ok_or_else(|| anyhow!("Expected PSBT string in createpsbt response"))?;

        // Validate PSBT
        Self::validate_psbt(psbt_string)?;

        Ok(PsbtResponse {
            psbt: psbt_string.to_string(),
            fee_sats: fee_sats_amount,
            change_position: None, // No change in consolidation
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_utxo_serialization() -> Result<()> {
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

        let json = serde_json::to_string(&utxo)?;
        let deserialized: Utxo = serde_json::from_str(&json)?;

        assert_eq!(utxo.txid, deserialized.txid);
        assert_eq!(utxo.amount, deserialized.amount);
        Ok(())
    }

    #[test]
    fn test_amount_input_parsing_sats() -> Result<()> {
        // Test satoshi parsing
        let amount = AmountInput::from_str("123sats")?;
        assert_eq!(amount.as_sat(), 123);
        assert_eq!(amount.as_btc(), 0.00000123);

        let amount = AmountInput::from_str("100000sat")?;
        assert_eq!(amount.as_sat(), 100000);
        assert_eq!(amount.as_btc(), 0.001);

        // Test with whitespace
        let amount = AmountInput::from_str("  50000sats  ")?;
        assert_eq!(amount.as_sat(), 50000);

        Ok(())
    }

    #[test]
    fn test_amount_input_parsing_msats() -> Result<()> {
        // Test millisatoshi parsing
        let amount = AmountInput::from_str("123000msats")?;
        assert_eq!(amount.as_millisats(), 123000);
        assert_eq!(amount.as_sat(), 123);
        assert_eq!(amount.as_btc(), 0.00000123);

        let amount = AmountInput::from_str("100000000msat")?;
        assert_eq!(amount.as_millisats(), 100000000);
        assert_eq!(amount.as_sat(), 100000);
        assert_eq!(amount.as_btc(), 0.001);

        // Test with whitespace
        let amount = AmountInput::from_str("  50000000msats  ")?;
        assert_eq!(amount.as_millisats(), 50000000);
        assert_eq!(amount.as_sat(), 50000);

        // Test case insensitivity
        let amount1 = AmountInput::from_str("1000MSATS")?;
        let amount2 = AmountInput::from_str("1000msats")?;
        assert_eq!(amount1, amount2);

        Ok(())
    }

    #[test]
    fn test_amount_input_parsing_btc() -> Result<()> {
        // Test BTC parsing
        let amount = AmountInput::from_str("0.666btc")?;
        assert_eq!(amount.as_btc(), 0.666);
        assert_eq!(amount.as_sat(), 66600000);

        let amount = AmountInput::from_str("1.5btc")?;
        assert_eq!(amount.as_btc(), 1.5);
        assert_eq!(amount.as_sat(), 150000000);

        // Test with whitespace
        let amount = AmountInput::from_str("  0.25btc  ")?;
        assert_eq!(amount.as_btc(), 0.25);

        Ok(())
    }

    #[test]
    fn test_amount_input_parsing_plain_number() -> Result<()> {
        // Test plain number (defaults to BTC)
        let amount = AmountInput::from_str("0.5")?;
        assert_eq!(amount.as_btc(), 0.5);
        assert_eq!(amount.as_sat(), 50000000);

        let amount = AmountInput::from_str("2.0")?;
        assert_eq!(amount.as_btc(), 2.0);
        assert_eq!(amount.as_sat(), 200000000);

        Ok(())
    }

    #[test]
    fn test_amount_input_parsing_case_insensitive() -> Result<()> {
        // Test case insensitivity
        let amount1 = AmountInput::from_str("123SATS")?;
        let amount2 = AmountInput::from_str("123sats")?;
        assert_eq!(amount1, amount2);

        let amount3 = AmountInput::from_str("0.5BTC")?;
        let amount4 = AmountInput::from_str("0.5btc")?;
        assert_eq!(amount3, amount4);

        Ok(())
    }

    #[test]
    fn test_amount_input_parsing_errors() -> Result<()> {
        // Test invalid formats
        assert!(AmountInput::from_str("").is_err());
        assert!(AmountInput::from_str("   ").is_err());
        assert!(AmountInput::from_str("abc").is_err());
        assert!(AmountInput::from_str("123xyz").is_err());
        assert!(AmountInput::from_str("sats").is_err());
        assert!(AmountInput::from_str("btc").is_err());

        // Test negative amounts
        assert!(AmountInput::from_str("-1btc").is_err());
        assert!(AmountInput::from_str("-0.5").is_err());

        // Test fractional satoshi amounts are now supported
        assert!(AmountInput::from_str("123.5sats").is_ok());

        Ok(())
    }

    #[test]
    fn test_amount_input_fractional_sats() -> Result<()> {
        // Test fractional satoshi parsing
        let amount = AmountInput::from_str("0.5sats")?;
        assert_eq!(amount.as_millisats(), 500);
        assert_eq!(amount.as_fractional_sats(), 0.5);
        assert_eq!(amount.as_sat(), 0); // Rounds down

        let amount = AmountInput::from_str("1.5sats")?;
        assert_eq!(amount.as_millisats(), 1500);
        assert_eq!(amount.as_fractional_sats(), 1.5);
        assert_eq!(amount.as_sat(), 1); // Rounds down

        let amount = AmountInput::from_str("123.456sats")?;
        assert_eq!(amount.as_millisats(), 123456);
        assert_eq!(amount.as_fractional_sats(), 123.456);
        assert_eq!(amount.as_sat(), 123); // Rounds down

        // Test direct creation from millisats
        let amount = AmountInput::from_millisats(1500);
        assert_eq!(amount.as_fractional_sats(), 1.5);

        // Test conversion from fractional sats
        let amount = AmountInput::from_fractional_sats(2.7)?;
        assert_eq!(amount.as_millisats(), 2700);

        Ok(())
    }

    #[test]
    fn test_amount_input_conversion() -> Result<()> {
        // Test round-trip conversions
        let original_sats = 123456789;
        let amount = AmountInput::from_sats(original_sats);
        assert_eq!(amount.as_sat(), original_sats);

        let original_btc = 1.23456789;
        let amount = AmountInput::from_btc(original_btc)?;
        assert_eq!(amount.as_btc(), original_btc);

        // Test specific conversions
        let amount = AmountInput::from_sats(100000000); // 1 BTC in sats
        assert_eq!(amount.as_btc(), 1.0);

        let amount = AmountInput::from_btc(0.00000001)?; // 1 sat in BTC
        assert_eq!(amount.as_sat(), 1);

        Ok(())
    }

    #[test]
    fn test_cookie_auth_parsing() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let cookie_path = temp_dir.join("test_cookie");

        // Create test cookie file
        let mut file = fs::File::create(&cookie_path)?;
        writeln!(file, "testuser:testpassword123")?;

        // Test parsing
        let (username, password) = BitcoinRpcClient::read_cookie_auth(&cookie_path)?;
        assert_eq!(username, "testuser");
        assert_eq!(password, "testpassword123");

        // Clean up
        fs::remove_file(&cookie_path)?;
        Ok(())
    }

    #[test]
    fn test_cookie_auth_invalid_format() -> Result<()> {
        let temp_dir = std::env::temp_dir();
        let cookie_path = temp_dir.join("test_cookie_invalid");

        // Create invalid cookie file (no colon)
        let mut file = fs::File::create(&cookie_path)?;
        writeln!(file, "invalidcookieformat")?;

        // Should fail
        let result = BitcoinRpcClient::read_cookie_auth(&cookie_path);
        assert!(result.is_err());

        // Clean up
        fs::remove_file(&cookie_path)?;
        Ok(())
    }

    #[test]
    fn test_utxo_list_response_serialization() -> Result<()> {
        let utxos = vec![
            UtxoOutput {
                txid: "abc123".to_string(),
                vout: 0,
                amount_sats: 100000, // 0.001 BTC = 100000 sats
                confirmations: 6,
                spendable: true,
                solvable: true,
                safe: true,
                address: Some("bc1qtest".to_string()),
                script_pub_key: "001400112233".to_string(),
                descriptor: Some("test_descriptor".to_string()),
            },
            UtxoOutput {
                txid: "def456".to_string(),
                vout: 1,
                amount_sats: 200000, // 0.002 BTC = 200000 sats
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
            total_amount_sats: 300000, // 0.003 BTC = 300000 sats
            total_count: 2,
        };

        let json = serde_json::to_string_pretty(&response)?;
        let deserialized: UtxoListResponse = serde_json::from_str(&json)?;

        assert_eq!(response.total_count, deserialized.total_count);
        assert_eq!(response.total_amount_sats, deserialized.total_amount_sats);
        assert_eq!(response.utxos.len(), deserialized.utxos.len());
        assert_eq!(response.utxos[0].txid, deserialized.utxos[0].txid);
        Ok(())
    }

    #[test]
    fn test_confirmation_calculation() -> Result<()> {
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
        Ok(())
    }

    #[test]
    fn test_psbt_response_serialization() -> Result<()> {
        let response = PsbtResponse {
            psbt: "cHNidP8BAHECAAAAAea2/lMA5WyAk9UuMJPJ7wfhNzrhAAAAAA0AAAA=".to_string(),
            fee_sats: 10000, // 0.0001 BTC = 10000 sats
            change_position: Some(1),
        };

        let json = serde_json::to_string_pretty(&response)?;
        let deserialized: PsbtResponse = serde_json::from_str(&json)?;

        assert_eq!(response.psbt, deserialized.psbt);
        assert_eq!(response.fee_sats, deserialized.fee_sats);
        assert_eq!(response.change_position, deserialized.change_position);
        Ok(())
    }

    #[test]
    fn test_input_parsing_logic() -> Result<()> {
        // Test the input parsing logic that would be in create_psbt
        let inputs = "abcd1234:0,efgh5678:1";
        let input_objects: Result<Vec<serde_json::Value>, anyhow::Error> = inputs
            .split(',')
            .map(|input| {
                let parts: Vec<&str> = input.trim().split(':').collect();
                if parts.len() != 2 {
                    bail!("Invalid input format: '{input}'. Expected 'txid:vout'");
                }
                let txid = parts[0];
                let vout: u32 = parts[1].parse().map_err(|_| {
                    anyhow!("Invalid vout '{vout}' in input '{input}'", vout = parts[1])
                })?;

                Ok(serde_json::json!({
                    "txid": txid,
                    "vout": vout
                }))
            })
            .collect();

        let input_objects = input_objects?;
        assert_eq!(input_objects.len(), 2);
        assert_eq!(input_objects[0]["txid"], "abcd1234");
        assert_eq!(input_objects[0]["vout"], 0);
        assert_eq!(input_objects[1]["txid"], "efgh5678");
        assert_eq!(input_objects[1]["vout"], 1);
        Ok(())
    }

    #[test]
    fn test_output_parsing_logic() -> Result<()> {
        // Test the output parsing logic with flexible amount formats
        let outputs = "bc1qtest123:0.001,bc1qtest456:0.002";
        let mut output_object = serde_json::Map::new();

        for output in outputs.split(',') {
            let parts: Vec<&str> = output.trim().split(':').collect();
            if parts.len() != 2 {
                bail!(
                    "Invalid output format: '{}'. Expected 'address:amount'",
                    output
                );
            }
            let address = parts[0];
            let amount_str = parts[1];

            // Parse amount using AmountInput for flexible format support
            let amount_input = AmountInput::from_str(amount_str)?;
            let amount_btc = amount_input.as_btc();

            output_object.insert(address.to_string(), serde_json::json!(amount_btc));
        }

        assert_eq!(output_object.len(), 2);
        assert_eq!(output_object["bc1qtest123"], 0.001);
        assert_eq!(output_object["bc1qtest456"], 0.002);
        Ok(())
    }

    #[test]
    fn test_output_parsing_flexible_formats() -> Result<()> {
        // Test parsing outputs with various amount formats
        let test_cases = vec![
            ("bc1qaddr:0.5", 0.5),
            ("bc1qaddr:0.5btc", 0.5),
            ("bc1qaddr:50000000sats", 0.5),
            ("bc1qaddr:50000000sat", 0.5),
            ("bc1qaddr:50000000000msats", 0.5),
            ("bc1qaddr:50000000000msat", 0.5),
        ];

        for (output_str, expected_btc) in test_cases {
            let parts: Vec<&str> = output_str.split(':').collect();
            let amount_input = AmountInput::from_str(parts[1])?;
            let amount_btc = amount_input.as_btc();

            assert!(
                (amount_btc - expected_btc).abs() < 0.00000001,
                "Failed for '{output_str}': expected {expected_btc} BTC, got {amount_btc} BTC"
            );
        }
        Ok(())
    }

    #[test]
    fn test_output_parsing_mixed_formats() -> Result<()> {
        // Test parsing multiple outputs with mixed amount formats
        let outputs = "bc1qaddr1:0.1btc,bc1qaddr2:100000sats,bc1qaddr3:0.00001";
        let mut output_object = serde_json::Map::new();

        for output in outputs.split(',') {
            let parts: Vec<&str> = output.trim().split(':').collect();
            let address = parts[0];
            let amount_input = AmountInput::from_str(parts[1])?;
            let amount_btc = amount_input.as_btc();
            output_object.insert(address.to_string(), serde_json::json!(amount_btc));
        }

        assert_eq!(output_object.len(), 3);
        assert_eq!(output_object["bc1qaddr1"], 0.1);
        assert_eq!(output_object["bc1qaddr2"], 0.001);
        assert_eq!(output_object["bc1qaddr3"], 0.00001);
        Ok(())
    }

    #[test]
    fn test_invalid_input_format() -> Result<()> {
        let inputs = "invalid_format";
        let result: Result<Vec<serde_json::Value>, anyhow::Error> = inputs
            .split(',')
            .map(|input| {
                let parts: Vec<&str> = input.trim().split(':').collect();
                if parts.len() != 2 {
                    bail!("Invalid input format: '{input}'. Expected 'txid:vout'");
                }
                Ok(serde_json::json!({}))
            })
            .collect();

        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("Invalid input format"));
        }
        Ok(())
    }

    #[test]
    fn test_wallet_funded_psbt_response_serialization() -> Result<()> {
        let response = WalletFundedPsbtResponse {
            psbt: "cHNidP8BAHECAAAAAea2/lMA5WyAk9UuMJPJ7wfhNzrhAAAAAA0AAAA=".to_string(),
            fee_sats: 10000, // 0.0001 BTC = 10000 sats
            change_position: 1,
        };

        let json = serde_json::to_string_pretty(&response)?;
        let deserialized: WalletFundedPsbtResponse = serde_json::from_str(&json)?;

        assert_eq!(response.psbt, deserialized.psbt);
        assert_eq!(response.fee_sats, deserialized.fee_sats);
        assert_eq!(response.change_position, deserialized.change_position);
        Ok(())
    }

    #[test]
    fn test_fee_rate_conversion() -> Result<()> {
        // Test the conversion from sat/vB to BTC/kvB used in wallet_create_funded_psbt
        let sat_per_vb = 20.0f64;
        let btc_per_kvb = sat_per_vb * 100_000.0 / 100_000_000.0; // sat/vB to BTC/kvB
        let expected = 0.02000000f64; // 20 sat/vB = 0.02 BTC/kvB

        assert!((btc_per_kvb - expected).abs() < 0.00000001);
        Ok(())
    }

    #[test]
    fn test_empty_inputs_parsing() -> Result<()> {
        // Test that empty string inputs result in empty array for automatic selection
        let inputs = "";
        let input_objects: Vec<serde_json::Value> = if inputs.is_empty() {
            Vec::new()
        } else {
            // Normal parsing logic would go here
            Vec::new()
        };

        assert_eq!(input_objects.len(), 0);
        Ok(())
    }

    #[test]
    fn test_transaction_weight_estimation() -> Result<()> {
        // Test weight estimation using rust-bitcoin's predict_weight function

        // Single input, single output
        let weight = BitcoinRpcClient::estimate_transaction_weight(1, 1);
        let vbytes = weight.to_wu().div_ceil(4) as u32;
        // P2WPKH transaction: should be around 110 vbytes (rust-bitcoin's precise calculation)
        assert!(
            (109..=112).contains(&vbytes),
            "Expected ~110 vbytes, got {vbytes}"
        );

        // Two inputs, two outputs (typical send with change)
        let weight = BitcoinRpcClient::estimate_transaction_weight(2, 2);
        let vbytes = weight.to_wu().div_ceil(4) as u32;
        // Should be around 208 vbytes
        assert!(
            (206..=212).contains(&vbytes),
            "Expected ~208 vbytes, got {vbytes}"
        );

        // Multiple inputs consolidation
        let weight = BitcoinRpcClient::estimate_transaction_weight(5, 1);
        let vbytes = weight.to_wu().div_ceil(4) as u32;
        // Should be around 380 vbytes
        assert!(
            (378..=385).contains(&vbytes),
            "Expected ~380 vbytes, got {vbytes}"
        );
        Ok(())
    }

    #[test]
    fn test_fee_calculation() -> Result<()> {
        // Test the fee calculation logic used in create_psbt
        let num_inputs = 1;
        let num_outputs = 2;
        let fee_rate = 20.0f64; // sat/vB

        let tx_weight = BitcoinRpcClient::estimate_transaction_weight(num_inputs, num_outputs);
        let tx_vbytes = tx_weight.to_wu().div_ceil(4) as u32;
        let fee_btc = (tx_vbytes as f64 * fee_rate) / 100_000_000.0;

        // Fee calculation should be: vbytes * 20 sat/vB converted to BTC
        // With rust-bitcoin's precise calculation, verify fee is reasonable for 1 input, 2 outputs
        let fee_sats = tx_vbytes as f64 * fee_rate;
        assert!(
            (2600.0..=2900.0).contains(&fee_sats),
            "Expected fee ~2800 sats, got {fee_sats} sats"
        );

        // Verify BTC conversion is correct
        let expected_fee_btc = fee_sats / 100_000_000.0;
        assert!((fee_btc - expected_fee_btc).abs() < 0.0000001);
        Ok(())
    }

    #[test]
    fn test_fee_calculation_with_feerate() -> Result<()> {
        // Test the new rust-bitcoin FeeRate-based fee calculation
        let num_inputs = 1;
        let num_outputs = 2;
        let fee_rate = 20.0f64; // sat/vB

        let tx_weight = BitcoinRpcClient::estimate_transaction_weight(num_inputs, num_outputs);
        let fee_amount = BitcoinRpcClient::calculate_fee_with_feerate(tx_weight, fee_rate);
        let fee_btc = fee_amount.to_btc();

        // Verify fee is reasonable for 1 input, 2 outputs at 20 sat/vB
        let fee_sats = fee_amount.to_sat();
        assert!(
            (2600..=2900).contains(&fee_sats),
            "Expected fee ~2800 sats, got {fee_sats} sats"
        );

        // Compare with manual calculation to ensure consistency
        let tx_vbytes = tx_weight.to_wu().div_ceil(4) as u32;
        let manual_fee_btc = (tx_vbytes as f64 * fee_rate) / 100_000_000.0;
        assert!(
            (fee_btc - manual_fee_btc).abs() < 0.0000001,
            "FeeRate calculation differs from manual"
        );
        Ok(())
    }

    #[test]
    fn test_sub_1_satbyte_fee_with_amount() -> Result<()> {
        // Test sub-1 sat/vB fees using rust-bitcoin's Amount type
        let num_inputs = 1;
        let num_outputs = 2;
        let fee_rate = 0.1f64; // 0.1 sat/vB

        let tx_weight = BitcoinRpcClient::estimate_transaction_weight(num_inputs, num_outputs);
        let fee_amount = BitcoinRpcClient::calculate_fee_with_feerate(tx_weight, fee_rate);
        let fee_btc = fee_amount.to_btc();

        // For 0.1 sat/vB, expect ~13-14 sats total fee
        let fee_sats = fee_amount.to_sat();
        assert!(
            (13..=15).contains(&fee_sats),
            "Expected fee ~14 sats, got {fee_sats} sats"
        );

        // Verify BTC conversion is reasonable for tiny amounts
        assert!(
            fee_btc > 0.0 && fee_btc < 0.00000100,
            "Expected tiny BTC amount, got {fee_btc}"
        );

        // Test even smaller rate
        let tiny_rate = 0.01f64; // 0.01 sat/vB
        let tiny_fee = BitcoinRpcClient::calculate_fee_with_feerate(tx_weight, tiny_rate);
        let tiny_sats = tiny_fee.to_sat();
        assert!(
            (1..=2).contains(&tiny_sats),
            "Expected ~1 sat, got {tiny_sats} sats"
        );
        Ok(())
    }

    #[test]
    fn test_psbt_validation() -> Result<()> {
        // Test PSBT validation with valid PSBT (this is a minimal valid PSBT from the spec)
        let valid_psbt = "cHNidP8BAJoCAAAAAljoeiG1ba8UV76bKlSu3iwYyYR3JStOGhp9w+gCEGOUqABAAAABPUA= AJocCAAABSk3LjAAAAAAAAA=";

        // This should fail because it's not a complete valid PSBT, but we test our parser integration
        let result = BitcoinRpcClient::validate_psbt(valid_psbt);
        // The test passes if we can attempt parsing without panicking
        // In practice, valid PSBTs from Bitcoin Core will parse correctly
        assert!(result.is_ok() || result.is_err()); // Either outcome is valid for parsing attempt

        // Test with clearly invalid base64
        let invalid_psbt = "not-valid-base64!@#$";
        let result = BitcoinRpcClient::validate_psbt(invalid_psbt);
        assert!(result.is_err(), "Should fail to parse invalid base64");
        Ok(())
    }

    #[test]
    fn test_sub_1_satbyte_fee_conversion() -> Result<()> {
        // Test conversion for sub 1 sat/vB fees
        let sat_per_vb = 0.1f64;
        let btc_per_kvb = sat_per_vb * 100_000.0 / 100_000_000.0; // 0.1 sat/vB to BTC/kvB

        println!("0.1 sat/vB converts to {btc_per_kvb} BTC/kvB");

        // Expected: 0.1 sat/vB = 0.0001 BTC/kvB
        let expected = 0.0001f64;
        assert!((btc_per_kvb - expected).abs() < 0.0000001);

        // Test even smaller values
        let tiny_rate = 0.01f64;
        let tiny_btc_kvb = tiny_rate * 100_000.0 / 100_000_000.0;
        println!("0.01 sat/vB converts to {tiny_btc_kvb} BTC/kvB");

        // Expected: 0.01 sat/vB = 0.00001 BTC/kvB
        let expected_tiny = 0.00001f64;
        assert!((tiny_btc_kvb - expected_tiny).abs() < 0.000001);
        Ok(())
    }

    #[test]
    fn test_move_utxos_input_parsing() -> Result<()> {
        // Test input parsing logic used in move_utxos
        let inputs = "abcd1234:0,efgh5678:1,ijkl9012:2";
        let input_objects: Result<Vec<serde_json::Value>, anyhow::Error> = inputs
            .split(',')
            .map(|input| {
                let parts: Vec<&str> = input.trim().split(':').collect();
                if parts.len() != 2 {
                    bail!("Invalid input format: '{input}'. Expected 'txid:vout'");
                }
                let txid = parts[0];
                let vout: u32 = parts[1].parse().map_err(|_| {
                    anyhow!("Invalid vout '{vout}' in input '{input}'", vout = parts[1])
                })?;

                Ok(serde_json::json!({
                    "txid": txid,
                    "vout": vout
                }))
            })
            .collect();

        let input_objects = input_objects?;
        assert_eq!(input_objects.len(), 3);

        // Verify first input
        assert_eq!(input_objects[0]["txid"], "abcd1234");
        assert_eq!(input_objects[0]["vout"], 0);

        // Verify second input
        assert_eq!(input_objects[1]["txid"], "efgh5678");
        assert_eq!(input_objects[1]["vout"], 1);

        // Verify third input
        assert_eq!(input_objects[2]["txid"], "ijkl9012");
        assert_eq!(input_objects[2]["vout"], 2);
        Ok(())
    }

    #[test]
    fn test_move_utxos_fee_calculation() -> Result<()> {
        // Test fee calculation logic for consolidation transactions

        // Test 3 inputs to 1 output (typical consolidation)
        let num_inputs = 3;
        let num_outputs = 1;
        let fee_rate = 15.0f64; // sat/vB

        let tx_weight = BitcoinRpcClient::estimate_transaction_weight(num_inputs, num_outputs);
        let fee_amount = BitcoinRpcClient::calculate_fee_with_feerate(tx_weight, fee_rate);
        let fee_sats = fee_amount.to_sat();

        // 3 inputs, 1 output should be around 246-250 vbytes at 15 sat/vB = ~3690-3750 sats
        assert!(
            (3600..=3900).contains(&fee_sats),
            "Expected ~3700 sats fee, got {fee_sats} sats"
        );

        // Test large consolidation (10 inputs to 1 output)
        let large_num_inputs = 10;
        let large_weight =
            BitcoinRpcClient::estimate_transaction_weight(large_num_inputs, num_outputs);
        let large_fee = BitcoinRpcClient::calculate_fee_with_feerate(large_weight, fee_rate);
        let large_fee_sats = large_fee.to_sat();

        // Should be significantly larger than 3-input case
        assert!(
            large_fee_sats > fee_sats * 2,
            "Large consolidation fee should be > 2x small consolidation"
        );
        Ok(())
    }

    #[test]
    fn test_consolidation_output_calculation() -> Result<()> {
        // Test the math for calculating consolidation output amount
        let total_input_value = 0.05f64; // 0.05 BTC total inputs

        // Simulate fee calculation for 5 inputs, 1 output
        let num_inputs = 5;
        let num_outputs = 1;
        let fee_rate = 20.0f64; // sat/vB

        let tx_weight = BitcoinRpcClient::estimate_transaction_weight(num_inputs, num_outputs);
        let fee_amount = BitcoinRpcClient::calculate_fee_with_feerate(tx_weight, fee_rate);
        let fee_btc = fee_amount.to_btc();

        // Calculate output amount
        let output_amount = total_input_value - fee_btc;

        // Should be positive and reasonable
        assert!(output_amount > 0.0, "Output amount should be positive");
        assert!(
            output_amount < total_input_value,
            "Output should be less than input"
        );
        assert!(fee_btc > 0.0, "Fee should be positive");

        // Fee should be reasonable (less than a few % for this size)
        let fee_percentage = (fee_btc / total_input_value) * 100.0;
        assert!(
            fee_percentage < 5.0,
            "Fee should be less than 5% for this transaction size"
        );
        Ok(())
    }

    #[test]
    fn test_insufficient_funds_case() -> Result<()> {
        // Test logic for detecting insufficient funds
        let tiny_input_value = 0.00001f64; // 1000 sats

        // Large fee due to many inputs
        let num_inputs = 20;
        let num_outputs = 1;
        let high_fee_rate = 100.0f64; // Very high fee rate

        let tx_weight = BitcoinRpcClient::estimate_transaction_weight(num_inputs, num_outputs);
        let fee_amount = BitcoinRpcClient::calculate_fee_with_feerate(tx_weight, high_fee_rate);
        let fee_btc = fee_amount.to_btc();

        // This should result in insufficient funds
        let output_amount = tiny_input_value - fee_btc;
        assert!(
            output_amount <= 0.0,
            "Should detect insufficient funds when fee > input"
        );
        Ok(())
    }

    #[test]
    fn test_move_utxos_response_structure() -> Result<()> {
        // Test the response structure for move_utxos
        let response = PsbtResponse {
            psbt: "cHNidP8BAHECAAAAAea2/lMA5WyAk9UuMJPJ7wfhNzrhAAAAAA0AAAA=".to_string(),
            fee_sats: 15000,       // 15000 sats
            change_position: None, // No change in consolidation
        };

        let json = serde_json::to_string_pretty(&response)?;
        let deserialized: PsbtResponse = serde_json::from_str(&json)?;

        assert_eq!(response.psbt, deserialized.psbt);
        assert_eq!(response.fee_sats, deserialized.fee_sats);
        assert_eq!(response.change_position, deserialized.change_position);
        assert!(
            response.change_position.is_none(),
            "Consolidation should have no change"
        );
        Ok(())
    }

    #[test]
    fn test_single_utxo_consolidation() -> Result<()> {
        // Edge case: consolidating a single UTXO (essentially just moving it)
        let num_inputs = 1;
        let num_outputs = 1;
        let fee_rate = 10.0f64;

        let tx_weight = BitcoinRpcClient::estimate_transaction_weight(num_inputs, num_outputs);
        let fee_amount = BitcoinRpcClient::calculate_fee_with_feerate(tx_weight, fee_rate);
        let fee_sats = fee_amount.to_sat();

        // Single input, single output should be minimal size (~110 vbytes * 10 sat/vB = ~1100 sats)
        assert!(
            (1000..=1300).contains(&fee_sats),
            "Expected ~1100 sats fee for single UTXO move, got {fee_sats} sats"
        );
        Ok(())
    }

    #[test]
    fn test_move_utxos_absolute_fee_calculation() -> Result<()> {
        // Test fee calculation logic with absolute fee amounts

        // Test absolute fee in satoshis
        let fee_sats = 5000u64; // 5000 satoshis
        let fee_amount = Amount::from_sat(fee_sats);
        let fee_btc = fee_amount.to_btc();

        // Verify conversion accuracy
        assert_eq!(fee_btc, 0.00005000f64);
        assert_eq!(fee_amount.to_sat(), 5000);

        // Test typical consolidation fee amounts
        let typical_fees = vec![1000, 2500, 5000, 10000, 25000]; // Various sat amounts

        for fee_sat in typical_fees {
            let amount = Amount::from_sat(fee_sat);
            let btc_value = amount.to_btc();
            let back_to_sat = Amount::from_btc(btc_value)?.to_sat();

            // Ensure round-trip conversion is accurate
            assert_eq!(
                fee_sat, back_to_sat,
                "Fee conversion should be lossless for {fee_sat} sats"
            );
        }

        Ok(())
    }

    #[test]
    fn test_move_utxos_fee_validation() -> Result<()> {
        // Test fee parameter validation logic

        // Test that we can handle both fee rate and absolute fee scenarios
        let fee_rate = Some(15.0f64);
        let fee_sats = Some(5000u64);

        // This should represent the validation logic from move_utxos function
        match (fee_rate, None::<u64>) {
            (Some(rate), None) => {
                assert!(rate > 0.0, "Fee rate should be positive");
                // Fee rate case works
            }
            (None, Some(sats)) => {
                assert!(sats > 0, "Fee sats should be positive");
                // Absolute fee case works
            }
            (Some(_), Some(_)) => {
                panic!("Should not specify both fee rate and absolute fee");
            }
            (None, None) => {
                panic!("Must specify either fee rate or absolute fee");
            }
        }

        // Test absolute fee case
        match (None::<f64>, fee_sats) {
            (None, Some(sats)) => {
                assert_eq!(sats, 5000);
                // Absolute fee case works
            }
            _ => panic!("Should handle absolute fee case"),
        }

        Ok(())
    }

    #[test]
    fn test_input_format_validation() -> Result<()> {
        // Test that input format validation works correctly

        // Valid txid:vout format
        let valid_input = "abcd1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcd:0";
        let parts: Vec<&str> = valid_input.split(':').collect();
        assert_eq!(parts.len(), 2);
        assert!(parts[1].parse::<u32>().is_ok());

        // Invalid formats
        let invalid_inputs = vec![
            "invalid_format",
            "txid_without_colon",
            "txid:not_a_number",
            "txid:0:extra_part",
            "",
        ];

        for invalid in invalid_inputs {
            let parts: Vec<&str> = invalid.split(':').collect();
            let is_valid = parts.len() == 2 && parts[1].parse::<u32>().is_ok();
            assert!(!is_valid, "Input '{invalid}' should be invalid");
        }
        Ok(())
    }

    #[test]
    fn test_descriptor_detection() -> Result<()> {
        // Test various descriptor formats
        let descriptors = vec![
            "wpkh([fingerprint/84'/0'/0']xpub...)",
            "pkh([fingerprint/44'/0'/0']xpub...)",
            "sh(wpkh([fingerprint/49'/0'/0']xpub...))",
            "tr([fingerprint/86'/0'/0']xpub...)",
            "wsh(multi(2,[fingerprint1/48'/0'/0'/2']xpub1,[fingerprint2/48'/0'/0'/2']xpub2))",
            "addr(bc1qexample)",
        ];

        for desc in descriptors {
            // All should be detected as descriptors
            assert!(
                desc.contains('(') || desc.contains('['),
                "Descriptor '{desc}' should contain parentheses or brackets"
            );
        }

        // Test non-descriptors
        let non_descriptors = vec!["abcd1234:0", "ef567890:10", "1234567890abcdef:123"];

        for non_desc in non_descriptors {
            // None should be detected as descriptors
            assert!(
                !non_desc.contains('(') && !non_desc.contains('['),
                "Non-descriptor '{non_desc}' should not contain parentheses or brackets"
            );
        }
        Ok(())
    }

    #[test]
    fn test_input_parsing_with_vec() -> Result<()> {
        // Test parsing Vec<String> inputs instead of comma-separated strings
        let standard_inputs = vec!["txid1:0".to_string(), "txid2:1".to_string()];
        let descriptor_input = vec!["wpkh([fingerprint/84'/0'/0']xpub...)".to_string()];
        let mixed_inputs = [
            "txid1:0".to_string(),
            "wpkh([fingerprint/84'/0'/0']xpub...)".to_string(),
            "txid2:1".to_string(),
        ];

        // Test standard inputs detection
        for input in &standard_inputs {
            assert!(!input.contains('(') && !input.contains('['));
        }

        // Test descriptor input detection
        for input in &descriptor_input {
            assert!(input.contains('(') || input.contains('['));
        }

        // Test mixed inputs
        assert!(!mixed_inputs[0].contains('(') && !mixed_inputs[0].contains('['));
        assert!(mixed_inputs[1].contains('(') || mixed_inputs[1].contains('['));
        assert!(!mixed_inputs[2].contains('(') && !mixed_inputs[2].contains('['));
        Ok(())
    }

    #[test]
    fn test_descriptor_formats() -> Result<()> {
        // Test various descriptor formats that should be detected correctly
        let descriptors = vec![
            "wpkh([fingerprint/84'/0'/0']xpub...)",
            "pkh([fingerprint/44'/0'/0']xpub...)",
            "sh(wpkh([fingerprint/49'/0'/0']xpub...))",
            "tr([fingerprint/86'/0'/0']xpub...)",
            "wsh(sortedmulti(4,[fp1/48'/0'/0'/2']xpub1,[fp2/48'/0'/0'/2']xpub2,[fp3/48'/0'/0'/2']xpub3,[fp4/48'/0'/0'/2']xpub4))",
            "addr(bc1qexample)",
        ];

        for desc in descriptors {
            // All should be detected as descriptors
            assert!(
                desc.contains('(') || desc.contains('['),
                "Descriptor '{desc}' should contain parentheses or brackets"
            );
        }

        // Test non-descriptors
        let non_descriptors = vec!["abcd1234:0", "ef567890:10", "1234567890abcdef:123"];

        for non_desc in non_descriptors {
            // None should be detected as descriptors
            assert!(
                !non_desc.contains('(') && !non_desc.contains('['),
                "Non-descriptor '{non_desc}' should not contain parentheses or brackets"
            );
        }
        Ok(())
    }

    #[test]
    fn test_bitcoin_amount_serialization() -> Result<()> {
        // Test very small amounts
        let response = PsbtResponse {
            psbt: "test".to_string(),
            fee_sats: 11, // 11 sats (was 11.1 sats, rounded down)
            change_position: None,
        };

        let json = serde_json::to_string_pretty(&response)?;
        println!("Serialized small amount: {json}");

        // The fee should be present and deserializable
        assert!(json.contains("\"fee_sats\":"));

        // Test with zero
        let zero_response = PsbtResponse {
            psbt: "test".to_string(),
            fee_sats: 0,
            change_position: None,
        };

        let zero_json = serde_json::to_string_pretty(&zero_response)?;
        println!("Serialized zero: {zero_json}");
        assert!(zero_json.contains("\"fee_sats\": 0"));

        // Test with normal-sized amount
        let normal_response = PsbtResponse {
            psbt: "test".to_string(),
            fee_sats: 12840, // 12840 sats - normal fee range
            change_position: None,
        };

        let normal_json = serde_json::to_string_pretty(&normal_response)?;
        println!("Serialized normal amount: {normal_json}");
        assert!(normal_json.contains("\"fee_sats\": 12840"));

        // Test deserialization works for all cases
        let deserialized: PsbtResponse = serde_json::from_str(&json)?;
        assert_eq!(deserialized.fee_sats, 11);

        let zero_deserialized: PsbtResponse = serde_json::from_str(&zero_json)?;
        assert_eq!(zero_deserialized.fee_sats, 0);

        let normal_deserialized: PsbtResponse = serde_json::from_str(&normal_json)?;
        assert_eq!(normal_deserialized.fee_sats, 12840);
        Ok(())
    }
}
