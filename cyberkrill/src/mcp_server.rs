use anyhow::Result;
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParam, CallToolResult, Content, ListToolsResult, PaginatedRequestParam,
        ServerCapabilities, ServerInfo, Tool,
    },
    schemars,
    service::{RequestContext, ServiceExt},
    tool,
    transport::stdio,
    ErrorData as McpError, RoleServer,
};
use serde::Deserialize;
use std::future::Future;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Configuration for the MCP server
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub transport: Transport,
    #[allow(dead_code)] // Will be used when SSE transport is implemented
    pub host: String,
    #[allow(dead_code)] // Will be used when SSE transport is implemented
    pub port: u16,
}

#[derive(Debug, Clone)]
pub enum Transport {
    Stdio,
    Sse,
}

impl Default for McpServerConfig {
    fn default() -> Self {
        Self {
            transport: Transport::Stdio,
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

/// The main MCP server for cyberkrill
#[derive(Clone)]
pub struct CyberkrillMcpServer {
    config: McpServerConfig,
    #[allow(dead_code)] // Reserved for future shared state management
    state: Arc<Mutex<ServerState>>,
}

#[derive(Default)]
struct ServerState {
    // Add any shared state here if needed
}

// Lightning Network tool requests
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DecodeInvoiceRequest {
    #[schemars(description = "The BOLT11 invoice string to decode")]
    pub invoice: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DecodeLnurlRequest {
    #[schemars(description = "The LNURL string to decode")]
    pub lnurl: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct GenerateInvoiceRequest {
    #[schemars(description = "Lightning address (e.g., user@domain.com)")]
    pub address: String,
    #[schemars(description = "Amount in millisatoshis")]
    pub amount_msats: u64,
    #[schemars(description = "Optional comment for the invoice")]
    pub comment: Option<String>,
}

// Fedimint tool requests
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DecodeFedimintInviteRequest {
    #[schemars(description = "The Fedimint invite code to decode")]
    pub invite_code: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct EncodeFedimintInviteRequest {
    #[schemars(description = "The federation ID (hex string)")]
    pub federation_id: String,
    #[schemars(description = "List of guardian nodes")]
    pub guardians: Vec<Guardian>,
    #[schemars(description = "Optional API secret")]
    pub api_secret: Option<String>,
    #[schemars(description = "Skip API secret for fedimint-cli compatibility")]
    pub skip_api_secret: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct Guardian {
    pub peer_id: u64,
    pub url: String,
}

// Bitcoin tool requests
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ListUtxosRequest {
    #[schemars(description = "Output descriptor (e.g., wpkh(...))")]
    pub descriptor: Option<String>,
    #[schemars(description = "List of Bitcoin addresses")]
    pub addresses: Option<Vec<String>>,
    #[schemars(description = "Bitcoin network (mainnet, testnet, signet, regtest)")]
    pub network: Option<String>,
    #[schemars(description = "Backend to use (bitcoind, electrum, esplora)")]
    pub backend: Option<String>,
    #[schemars(description = "Backend URL (for electrum/esplora)")]
    pub backend_url: Option<String>,
    #[schemars(description = "Bitcoin data directory (for bitcoind)")]
    pub bitcoin_dir: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DecodePsbtRequest {
    #[schemars(description = "PSBT in base64 or hex format")]
    pub psbt: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreatePsbtRequest {
    #[schemars(description = "Input specifications (txid:vout format or descriptors)")]
    pub inputs: Vec<String>,
    #[schemars(description = "Output specifications (address:amount format, comma-separated)")]
    pub outputs: String,
    #[schemars(description = "Fee rate in sat/vB")]
    pub fee_rate: Option<f64>,
    #[schemars(description = "Output descriptor for BDK backends")]
    pub descriptor: Option<String>,
    #[schemars(description = "Bitcoin network (mainnet, testnet, signet, regtest)")]
    pub network: Option<String>,
    #[schemars(description = "Backend to use (bitcoind, electrum, esplora)")]
    pub backend: Option<String>,
    #[schemars(description = "Backend URL (for electrum/esplora)")]
    pub backend_url: Option<String>,
    #[schemars(description = "Bitcoin data directory (for bitcoind)")]
    pub bitcoin_dir: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct CreateFundedPsbtRequest {
    #[schemars(description = "Output specifications (address:amount format, comma-separated)")]
    pub outputs: String,
    #[schemars(description = "Optional input specifications (txid:vout format or descriptors)")]
    pub inputs: Option<Vec<String>>,
    #[schemars(description = "Fee rate in sat/vB (overrides conf_target)")]
    pub fee_rate: Option<f64>,
    #[schemars(description = "Confirmation target in blocks")]
    pub conf_target: Option<u32>,
    #[schemars(description = "Fee estimation mode (ECONOMICAL or CONSERVATIVE)")]
    pub estimate_mode: Option<String>,
    #[schemars(description = "Output descriptor for BDK backends")]
    pub descriptor: Option<String>,
    #[schemars(description = "Bitcoin network (mainnet, testnet, signet, regtest)")]
    pub network: Option<String>,
    #[schemars(description = "Backend to use (bitcoind, electrum, esplora)")]
    pub backend: Option<String>,
    #[schemars(description = "Backend URL or RPC URL for bitcoind")]
    pub backend_url: Option<String>,
    #[schemars(description = "Bitcoin data directory for cookie auth")]
    pub bitcoin_dir: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct MoveUtxosRequest {
    #[schemars(description = "Input specifications (txid:vout format or descriptors)")]
    pub inputs: Vec<String>,
    #[schemars(description = "Destination Bitcoin address")]
    pub destination: String,
    #[schemars(description = "Fee rate in sat/vB")]
    pub fee_rate: Option<f64>,
    #[schemars(description = "Absolute fee in satoshis")]
    pub fee: Option<u64>,
    #[schemars(description = "Maximum amount to move (e.g., '0.5btc' or '50000000sats')")]
    pub max_amount: Option<String>,
    #[schemars(description = "Output descriptor for BDK backends")]
    pub descriptor: Option<String>,
    #[schemars(description = "Bitcoin network (mainnet, testnet, signet, regtest)")]
    pub network: Option<String>,
    #[schemars(description = "Backend to use (bitcoind, electrum, esplora)")]
    pub backend: Option<String>,
    #[schemars(description = "Backend URL")]
    pub backend_url: Option<String>,
    #[schemars(description = "Bitcoin data directory")]
    pub bitcoin_dir: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct DcaReportRequest {
    #[schemars(description = "Output descriptor to analyze")]
    pub descriptor: String,
    #[schemars(description = "Fiat currency for price data (USD, EUR, GBP)")]
    pub currency: Option<String>,
    #[schemars(description = "Backend to use (bitcoind, electrum, esplora)")]
    pub backend: Option<String>,
    #[schemars(description = "Backend URL")]
    pub backend_url: Option<String>,
    #[schemars(description = "Bitcoin data directory (for bitcoind)")]
    pub bitcoin_dir: Option<String>,
    #[schemars(description = "Cache directory for price data")]
    pub cache_dir: Option<String>,
}

impl CyberkrillMcpServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ServerState::default())),
        }
    }

    /// Start the MCP server
    pub async fn run(self) -> Result<()> {
        // Initialize tracing only if not in test mode (RUST_LOG != error)
        let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
        if log_level != "error" {
            tracing_subscriber::fmt()
                .with_env_filter(
                    tracing_subscriber::EnvFilter::from_default_env()
                        .add_directive(tracing::Level::INFO.into()),
                )
                .init();

            info!("Starting cyberkrill MCP server");
        }

        match self.config.transport {
            Transport::Stdio => {
                if log_level != "error" {
                    info!("Starting MCP server with stdio transport");
                }
                let service = self.serve(stdio()).await?;
                service.waiting().await?;
            }
            Transport::Sse => {
                // SSE transport would require additional implementation
                // For now, we'll focus on stdio transport
                anyhow::bail!("SSE transport not yet implemented");
            }
        }

        Ok(())
    }
}

// Implement the tool methods
impl CyberkrillMcpServer {
    // Lightning Network tools
    #[tool(description = "Decode a BOLT11 Lightning Network invoice")]
    async fn decode_invoice(
        &self,
        DecodeInvoiceRequest { invoice }: DecodeInvoiceRequest,
    ) -> CallToolResult {
        match cyberkrill_core::decode_invoice(&invoice) {
            Ok(result) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    #[tool(description = "Decode an LNURL string")]
    async fn decode_lnurl(
        &self,
        DecodeLnurlRequest { lnurl }: DecodeLnurlRequest,
    ) -> CallToolResult {
        match cyberkrill_core::decode_lnurl(&lnurl) {
            Ok(result) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    #[tool(description = "Generate a Lightning invoice from a Lightning address")]
    async fn generate_invoice(
        &self,
        GenerateInvoiceRequest {
            address,
            amount_msats,
            comment,
        }: GenerateInvoiceRequest,
    ) -> CallToolResult {
        // Convert amount_msats to AmountInput
        let amount = cyberkrill_core::AmountInput::from_millisats(amount_msats);

        match cyberkrill_core::generate_invoice_from_address(&address, &amount, comment.as_deref())
            .await
        {
            Ok(result) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    // Fedimint tools
    #[tool(description = "Decode a Fedimint federation invite code")]
    async fn decode_fedimint_invite(
        &self,
        DecodeFedimintInviteRequest { invite_code }: DecodeFedimintInviteRequest,
    ) -> CallToolResult {
        match fedimint_lite::decode_invite(&invite_code) {
            Ok(result) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    #[tool(description = "Encode a Fedimint federation invite code from JSON")]
    async fn encode_fedimint_invite(
        &self,
        EncodeFedimintInviteRequest {
            federation_id,
            guardians,
            api_secret,
            skip_api_secret,
        }: EncodeFedimintInviteRequest,
    ) -> CallToolResult {
        // We need to build the FedimintInviteOutput structure directly
        let guardians_info = guardians
            .into_iter()
            .map(|g| fedimint_lite::GuardianInfo {
                peer_id: g.peer_id as u16,
                url: g.url,
            })
            .collect();

        let invite = fedimint_lite::FedimintInviteOutput {
            federation_id,
            guardians: guardians_info,
            api_secret: if skip_api_secret.unwrap_or(false) {
                None
            } else {
                api_secret
            },
            encoding_format: "bech32m".to_string(),
        };

        match fedimint_lite::encode_fedimint_invite(&invite) {
            Ok(result) => CallToolResult::success(vec![Content::text(
                serde_json::json!({ "invite_code": result }).to_string(),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
        }
    }

    // Bitcoin tools
    #[tool(description = "List UTXOs for a Bitcoin descriptor or addresses")]
    async fn list_utxos(
        &self,
        ListUtxosRequest {
            descriptor,
            addresses,
            network,
            backend,
            backend_url,
            bitcoin_dir,
        }: ListUtxosRequest,
    ) -> CallToolResult {
        let network_str = network.as_deref().unwrap_or("mainnet");
        let network = match network_str.to_lowercase().as_str() {
            "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
            "testnet" => cyberkrill_core::Network::Testnet,
            "signet" => cyberkrill_core::Network::Signet,
            "regtest" => cyberkrill_core::Network::Regtest,
            _ => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Invalid network: {network_str}"
                ))])
            }
        };

        let backend_type = backend.as_deref().unwrap_or("bitcoind");

        let result = if let Some(desc) = descriptor {
            match backend_type {
                "electrum" => {
                    if let Some(url) = backend_url {
                        match cyberkrill_core::scan_and_list_utxos_electrum(
                            &desc, network, &url, 200,
                        )
                        .await
                        {
                            Ok(r) => r,
                            Err(e) => {
                                return CallToolResult::error(vec![Content::text(format!(
                                    "Error: {e}"
                                ))])
                            }
                        }
                    } else {
                        return CallToolResult::error(vec![Content::text(
                            "Error: backend_url required for electrum".to_string(),
                        )]);
                    }
                }
                "esplora" => {
                    if let Some(url) = backend_url {
                        match cyberkrill_core::scan_and_list_utxos_esplora(
                            &desc, network, &url, 200,
                        )
                        .await
                        {
                            Ok(r) => r,
                            Err(e) => {
                                return CallToolResult::error(vec![Content::text(format!(
                                    "Error: {e}"
                                ))])
                            }
                        }
                    } else {
                        return CallToolResult::error(vec![Content::text(
                            "Error: backend_url required for esplora".to_string(),
                        )]);
                    }
                }
                _ => {
                    let dir = bitcoin_dir.as_deref().unwrap_or("~/.bitcoin");
                    let path = std::path::Path::new(dir);
                    match cyberkrill_core::scan_and_list_utxos_bitcoind(&desc, network, path).await
                    {
                        Ok(r) => r,
                        Err(e) => {
                            return CallToolResult::error(vec![Content::text(format!(
                                "Error: {e}"
                            ))])
                        }
                    }
                }
            }
        } else if let Some(addrs) = addresses {
            let bitcoin_path = bitcoin_dir.map(|d| std::path::Path::new(&d).to_path_buf());
            let client = match cyberkrill_core::BitcoinRpcClient::new_auto(
                "http://127.0.0.1:8332".to_string(),
                bitcoin_path.as_deref(),
                None,
                None,
            ) {
                Ok(c) => c,
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Error creating client: {e}"
                    ))])
                }
            };

            let utxo_response = match client.list_utxos_for_addresses(addrs).await {
                Ok(r) => r,
                Err(e) => return CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
            };

            // Convert UtxoListResponse to Vec<BdkUtxo> for compatibility
            utxo_response
                .utxos
                .into_iter()
                .map(|utxo| {
                    cyberkrill_core::BdkUtxo {
                        txid: utxo.txid.clone(),
                        vout: utxo.vout,
                        address: utxo.address.clone().unwrap_or_default(),
                        amount: utxo.amount_sats,
                        amount_btc: utxo.amount_sats as f64 / 100_000_000.0,
                        confirmations: utxo.confirmations,
                        is_change: false, // We don't know from the RPC response
                        keychain: "External".to_string(), // Default to external
                        derivation_index: None, // Not available from RPC
                    }
                })
                .collect()
        } else {
            return CallToolResult::error(vec![Content::text(
                "Error: Either descriptor or addresses required".to_string(),
            )]);
        };

        let summary = cyberkrill_core::get_utxo_summary(result);
        CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&summary).unwrap_or_else(|e| e.to_string()),
        )])
    }

    #[tool(description = "Decode a PSBT (Partially Signed Bitcoin Transaction)")]
    async fn decode_psbt(&self, DecodePsbtRequest { psbt }: DecodePsbtRequest) -> CallToolResult {
        use base64::Engine;

        let psbt_bytes = if psbt.starts_with("cHNidP") {
            match base64::engine::general_purpose::STANDARD.decode(&psbt) {
                Ok(b) => b,
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Error decoding base64: {e}"
                    ))])
                }
            }
        } else {
            match hex::decode(&psbt) {
                Ok(b) => b,
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Error decoding hex: {e}"
                    ))])
                }
            }
        };

        match bitcoin::psbt::Psbt::deserialize(&psbt_bytes) {
            Ok(parsed_psbt) => {
                let result = serde_json::json!({
                    "unsigned_tx": format!("{:?}", parsed_psbt.unsigned_tx),
                    "version": parsed_psbt.version,
                    "inputs": parsed_psbt.inputs.len(),
                    "outputs": parsed_psbt.outputs.len(),
                });
                CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
                )])
            }
            Err(e) => {
                CallToolResult::error(vec![Content::text(format!("Error parsing PSBT: {e}"))])
            }
        }
    }

    #[tool(description = "Create PSBT with manual input/output specification")]
    async fn create_psbt(
        &self,
        CreatePsbtRequest {
            inputs,
            outputs,
            fee_rate,
            descriptor,
            network,
            backend,
            backend_url,
            bitcoin_dir,
        }: CreatePsbtRequest,
    ) -> CallToolResult {
        let network_str = network.as_deref().unwrap_or("mainnet");
        let network = match network_str.to_lowercase().as_str() {
            "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
            "testnet" => cyberkrill_core::Network::Testnet,
            "signet" => cyberkrill_core::Network::Signet,
            "regtest" => cyberkrill_core::Network::Regtest,
            _ => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Invalid network: {network_str}"
                ))])
            }
        };

        let backend_type = backend.as_deref().unwrap_or("bitcoind");
        let fee_rate_input = if let Some(rate) = fee_rate {
            match cyberkrill_core::AmountInput::from_btc(rate) {
                Ok(amt) => Some(amt),
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Invalid fee rate: {e}"
                    ))])
                }
            }
        } else {
            None
        };

        let result = if let Some(desc) = descriptor {
            // BDK path
            let backend_url_str = match backend_type {
                "electrum" => {
                    if let Some(url) = backend_url {
                        format!("electrum://{url}")
                    } else {
                        return CallToolResult::error(vec![Content::text(
                            "Error: backend_url required for electrum".to_string(),
                        )]);
                    }
                }
                "esplora" => {
                    if let Some(url) = backend_url {
                        format!("esplora://{url}")
                    } else {
                        return CallToolResult::error(vec![Content::text(
                            "Error: backend_url required for esplora".to_string(),
                        )]);
                    }
                }
                _ => {
                    let dir = backend_url.as_deref().unwrap_or("~/.bitcoin");
                    format!("bitcoind://{dir}")
                }
            };

            // Parse outputs into proper format for BDK
            let mut parsed_outputs = Vec::new();
            for output in outputs.split(',') {
                let parts: Vec<&str> = output.trim().split(':').collect();
                if parts.len() == 2 {
                    let address = parts[0].to_string();
                    match parts[1].parse::<f64>() {
                        Ok(amount_btc) => {
                            // Convert BTC to satoshis for Amount
                            let amount_sats = (amount_btc * 100_000_000.0) as u64;
                            let amount = bitcoin::Amount::from_sat(amount_sats);
                            parsed_outputs.push((address, amount));
                        }
                        Err(e) => {
                            return CallToolResult::error(vec![Content::text(format!(
                                "Invalid amount in output '{output}': {e}"
                            ))])
                        }
                    }
                } else {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Invalid output format: '{output}'. Expected 'address:amount'"
                    ))]);
                }
            }

            match cyberkrill_core::create_psbt_bdk(
                &inputs,
                &parsed_outputs,
                fee_rate_input.map(|r| r.as_sat() as f64 / 100.0),
                &desc,
                network,
                &backend_url_str,
            )
            .await
            {
                Ok(r) => CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&r).unwrap_or_else(|e| e.to_string()),
                )]),
                Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
            }
        } else {
            // Bitcoin Core RPC path
            let bitcoin_path = bitcoin_dir.map(|d| std::path::Path::new(&d).to_path_buf());
            let client = match cyberkrill_core::BitcoinRpcClient::new_auto(
                "http://127.0.0.1:8332".to_string(),
                bitcoin_path.as_deref(),
                None,
                None,
            ) {
                Ok(c) => c,
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Error creating client: {e}"
                    ))])
                }
            };

            match client.create_psbt(&inputs, &outputs, fee_rate_input).await {
                Ok(r) => CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&r).unwrap_or_else(|e| e.to_string()),
                )]),
                Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
            }
        };

        result
    }

    #[tool(description = "Create funded PSBT with automatic input selection")]
    async fn create_funded_psbt(
        &self,
        CreateFundedPsbtRequest {
            outputs,
            inputs,
            fee_rate,
            conf_target,
            estimate_mode,
            descriptor,
            network,
            backend,
            backend_url,
            bitcoin_dir,
        }: CreateFundedPsbtRequest,
    ) -> CallToolResult {
        let network_str = network.as_deref().unwrap_or("mainnet");
        let network = match network_str.to_lowercase().as_str() {
            "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
            "testnet" => cyberkrill_core::Network::Testnet,
            "signet" => cyberkrill_core::Network::Signet,
            "regtest" => cyberkrill_core::Network::Regtest,
            _ => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Invalid network: {network_str}"
                ))])
            }
        };

        let backend_type = backend.as_deref().unwrap_or("bitcoind");
        let fee_rate_input = if let Some(rate) = fee_rate {
            match cyberkrill_core::AmountInput::from_btc(rate) {
                Ok(amt) => Some(amt),
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Invalid fee rate: {e}"
                    ))])
                }
            }
        } else {
            None
        };

        let result = if let Some(desc) = descriptor {
            // BDK path
            let backend_url_str = match backend_type {
                "electrum" => {
                    if let Some(url) = backend_url {
                        format!("electrum://{url}")
                    } else {
                        return CallToolResult::error(vec![Content::text(
                            "Error: backend_url required for electrum".to_string(),
                        )]);
                    }
                }
                "esplora" => {
                    if let Some(url) = backend_url {
                        format!("esplora://{url}")
                    } else {
                        return CallToolResult::error(vec![Content::text(
                            "Error: backend_url required for esplora".to_string(),
                        )]);
                    }
                }
                _ => {
                    let dir = backend_url.as_deref().unwrap_or("~/.bitcoin");
                    format!("bitcoind://{dir}")
                }
            };

            // Parse outputs into proper format for BDK
            let mut parsed_outputs = Vec::new();
            for output in outputs.split(',') {
                let parts: Vec<&str> = output.trim().split(':').collect();
                if parts.len() == 2 {
                    let address = parts[0].to_string();
                    match parts[1].parse::<f64>() {
                        Ok(amount_btc) => {
                            // Convert BTC to satoshis for Amount
                            let amount_sats = (amount_btc * 100_000_000.0) as u64;
                            let amount = bitcoin::Amount::from_sat(amount_sats);
                            parsed_outputs.push((address, amount));
                        }
                        Err(e) => {
                            return CallToolResult::error(vec![Content::text(format!(
                                "Invalid amount in output '{output}': {e}"
                            ))])
                        }
                    }
                } else {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Invalid output format: '{output}'. Expected 'address:amount'"
                    ))]);
                }
            }

            match cyberkrill_core::create_funded_psbt_bdk(
                &parsed_outputs,
                conf_target,
                fee_rate_input.map(|r| r.as_sat() as f64 / 100.0),
                &desc,
                network,
                &backend_url_str,
            )
            .await
            {
                Ok(r) => CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&r).unwrap_or_else(|e| e.to_string()),
                )]),
                Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
            }
        } else {
            // Bitcoin Core RPC path
            let bitcoin_dir_path = bitcoin_dir.as_ref().map(std::path::Path::new);

            // Create RPC client - using cookie auth with bitcoin_dir
            let rpc_url = backend_url.unwrap_or_else(|| "http://127.0.0.1:8332".to_string());

            match cyberkrill_core::BitcoinRpcClient::new_auto(
                rpc_url,
                bitcoin_dir_path,
                None, // rpc_user - not provided via MCP
                None, // rpc_password - not provided via MCP
            ) {
                Ok(client) => {
                    // Convert inputs from Option<Vec<String>> to Vec<String> for the RPC call
                    let input_vec = inputs.unwrap_or_default();

                    // Convert fee_rate if provided
                    let fee_rate_amt = if let Some(rate) = fee_rate {
                        match cyberkrill_core::AmountInput::from_btc(rate) {
                            Ok(amt) => Some(amt),
                            Err(e) => {
                                return CallToolResult::error(vec![Content::text(format!(
                                    "Invalid fee rate: {e}"
                                ))])
                            }
                        }
                    } else {
                        None
                    };

                    match client
                        .wallet_create_funded_psbt(
                            &input_vec,
                            &outputs,
                            conf_target,
                            estimate_mode.as_deref(),
                            fee_rate_amt,
                        )
                        .await
                    {
                        Ok(r) => CallToolResult::success(vec![Content::text(
                            serde_json::to_string_pretty(&r).unwrap_or_else(|e| e.to_string()),
                        )]),
                        Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
                    }
                }
                Err(e) => CallToolResult::error(vec![Content::text(format!(
                    "Error creating Bitcoin RPC client: {e}"
                ))]),
            }
        };

        result
    }

    #[tool(description = "Consolidate/move UTXOs to a single destination")]
    async fn move_utxos(
        &self,
        MoveUtxosRequest {
            inputs,
            destination,
            fee_rate,
            fee,
            max_amount,
            descriptor,
            network,
            backend,
            backend_url,
            bitcoin_dir,
        }: MoveUtxosRequest,
    ) -> CallToolResult {
        let network_str = network.as_deref().unwrap_or("mainnet");
        let network = match network_str.to_lowercase().as_str() {
            "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
            "testnet" => cyberkrill_core::Network::Testnet,
            "signet" => cyberkrill_core::Network::Signet,
            "regtest" => cyberkrill_core::Network::Regtest,
            _ => {
                return CallToolResult::error(vec![Content::text(format!(
                    "Invalid network: {network_str}"
                ))])
            }
        };

        let backend_type = backend.as_deref().unwrap_or("bitcoind");
        let fee_rate_input = if let Some(rate) = fee_rate {
            match cyberkrill_core::AmountInput::from_btc(rate) {
                Ok(amt) => Some(amt),
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Invalid fee rate: {e}"
                    ))])
                }
            }
        } else {
            None
        };
        let fee_input = fee.map(cyberkrill_core::AmountInput::from_sats);
        let max_amount_input = if let Some(max_str) = max_amount {
            match cyberkrill_core::AmountInput::from_str(&max_str) {
                Ok(amt) => Some(amt),
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Invalid max_amount: {e}"
                    ))])
                }
            }
        } else {
            None
        };

        let result = if let Some(desc) = descriptor {
            // BDK path
            let backend_url_str = match backend_type {
                "electrum" => {
                    if let Some(url) = backend_url {
                        format!("electrum://{url}")
                    } else {
                        return CallToolResult::error(vec![Content::text(
                            "Error: backend_url required for electrum".to_string(),
                        )]);
                    }
                }
                "esplora" => {
                    if let Some(url) = backend_url {
                        format!("esplora://{url}")
                    } else {
                        return CallToolResult::error(vec![Content::text(
                            "Error: backend_url required for esplora".to_string(),
                        )]);
                    }
                }
                _ => {
                    let dir = backend_url.as_deref().unwrap_or("~/.bitcoin");
                    format!("bitcoind://{dir}")
                }
            };

            match cyberkrill_core::move_utxos_bdk(
                &inputs,
                &destination,
                fee_rate_input.map(|r| r.as_sat() as f64 / 100.0),
                fee_input.map(|f| f.as_sat()),
                max_amount_input.map(|amt| bitcoin::Amount::from_sat(amt.as_sat())),
                &desc,
                network,
                &backend_url_str,
            )
            .await
            {
                Ok(r) => CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&r).unwrap_or_else(|e| e.to_string()),
                )]),
                Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
            }
        } else {
            // Bitcoin Core RPC path
            let bitcoin_path = bitcoin_dir.map(|d| std::path::Path::new(&d).to_path_buf());
            let client = match cyberkrill_core::BitcoinRpcClient::new_auto(
                "http://127.0.0.1:8332".to_string(),
                bitcoin_path.as_deref(),
                None,
                None,
            ) {
                Ok(c) => c,
                Err(e) => {
                    return CallToolResult::error(vec![Content::text(format!(
                        "Error creating client: {e}"
                    ))])
                }
            };

            match client
                .move_utxos(
                    &inputs,
                    &destination,
                    fee_rate_input,
                    fee_input,
                    max_amount_input,
                )
                .await
            {
                Ok(r) => CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&r).unwrap_or_else(|e| e.to_string()),
                )]),
                Err(e) => CallToolResult::error(vec![Content::text(format!("Error: {e}"))]),
            }
        };

        result
    }

    #[tool(description = "Generate DCA (Dollar Cost Averaging) report for UTXOs")]
    async fn dca_report(
        &self,
        DcaReportRequest {
            descriptor,
            currency,
            backend,
            backend_url,
            bitcoin_dir,
            cache_dir,
        }: DcaReportRequest,
    ) -> CallToolResult {
        let currency_str = currency.as_deref().unwrap_or("USD");
        let backend_type = backend.as_deref().unwrap_or("bitcoind");

        let backend_enum = match backend_type {
            "electrum" => {
                if let Some(url) = backend_url {
                    cyberkrill_core::Backend::Electrum { url }
                } else {
                    return CallToolResult::error(vec![Content::text(
                        "Error: backend_url required for electrum".to_string(),
                    )]);
                }
            }
            "esplora" => {
                if let Some(url) = backend_url {
                    cyberkrill_core::Backend::Esplora { url }
                } else {
                    return CallToolResult::error(vec![Content::text(
                        "Error: backend_url required for esplora".to_string(),
                    )]);
                }
            }
            _ => {
                let dir = bitcoin_dir.as_deref().unwrap_or("~/.bitcoin");
                cyberkrill_core::Backend::BitcoinCore {
                    bitcoin_dir: std::path::PathBuf::from(dir),
                }
            }
        };

        let cache_path = cache_dir.map(|d| std::path::Path::new(&d).to_path_buf());

        match cyberkrill_core::generate_dca_report(
            &descriptor,
            backend_enum,
            currency_str,
            cache_path.as_deref(),
        )
        .await
        {
            Ok(report) => CallToolResult::success(vec![Content::text(
                serde_json::to_string_pretty(&report).unwrap_or_else(|e| e.to_string()),
            )]),
            Err(e) => CallToolResult::error(vec![Content::text(format!(
                "Error generating DCA report: {e}"
            ))]),
        }
    }
}

// Helper function to create a Tool with proper types
fn create_tool(name: &'static str, description: &'static str, schema: serde_json::Value) -> Tool {
    Tool {
        name: name.into(),
        description: Some(description.into()),
        input_schema: Arc::new(schema.as_object().unwrap().clone()),
        output_schema: None,
        annotations: None,
    }
}

// Implement ServerHandler trait
impl ServerHandler for CyberkrillMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        let tools = vec![
            create_tool(
                "decode_invoice",
                "Decode a BOLT11 Lightning Network invoice",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "invoice": {
                            "type": "string",
                            "description": "The BOLT11 invoice string to decode"
                        }
                    },
                    "required": ["invoice"]
                }),
            ),
            create_tool(
                "decode_lnurl",
                "Decode an LNURL string",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "lnurl": {
                            "type": "string",
                            "description": "The LNURL string to decode"
                        }
                    },
                    "required": ["lnurl"]
                }),
            ),
            create_tool(
                "generate_invoice",
                "Generate a Lightning invoice from a Lightning address",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "address": {
                            "type": "string",
                            "description": "Lightning address (e.g., user@domain.com)"
                        },
                        "amount_msats": {
                            "type": "integer",
                            "description": "Amount in millisatoshis"
                        },
                        "comment": {
                            "type": "string",
                            "description": "Optional comment for the invoice"
                        }
                    },
                    "required": ["address", "amount_msats"]
                }),
            ),
            create_tool(
                "decode_fedimint_invite",
                "Decode a Fedimint federation invite code",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "invite_code": {
                            "type": "string",
                            "description": "The Fedimint invite code to decode"
                        }
                    },
                    "required": ["invite_code"]
                }),
            ),
            create_tool(
                "encode_fedimint_invite",
                "Encode a Fedimint federation invite code from JSON",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "federation_id": {
                            "type": "string",
                            "description": "The federation ID (hex string)"
                        },
                        "guardians": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "peer_id": { "type": "integer" },
                                    "url": { "type": "string" }
                                },
                                "required": ["peer_id", "url"]
                            },
                            "description": "List of guardian nodes"
                        },
                        "api_secret": {
                            "type": "string",
                            "description": "Optional API secret"
                        },
                        "skip_api_secret": {
                            "type": "boolean",
                            "description": "Skip API secret for fedimint-cli compatibility"
                        }
                    },
                    "required": ["federation_id", "guardians"]
                }),
            ),
            create_tool(
                "list_utxos",
                "List Bitcoin UTXOs for addresses or descriptors",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "descriptor": {
                            "type": "string",
                            "description": "Output descriptor (e.g., wpkh(...))"
                        },
                        "addresses": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "List of Bitcoin addresses"
                        },
                        "network": {
                            "type": "string",
                            "description": "Bitcoin network (mainnet, testnet, signet, regtest)"
                        },
                        "backend": {
                            "type": "string",
                            "description": "Backend to use (bitcoind, electrum, esplora)"
                        },
                        "backend_url": {
                            "type": "string",
                            "description": "Backend URL (for electrum/esplora)"
                        },
                        "bitcoin_dir": {
                            "type": "string",
                            "description": "Bitcoin data directory (for bitcoind)"
                        }
                    }
                }),
            ),
            create_tool(
                "decode_psbt",
                "Decode a PSBT (Partially Signed Bitcoin Transaction)",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "psbt": {
                            "type": "string",
                            "description": "PSBT in base64 or hex format"
                        }
                    },
                    "required": ["psbt"]
                }),
            ),
            create_tool(
                "create_psbt",
                "Create PSBT with manual input/output specification",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "inputs": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Input specifications (txid:vout format or descriptors)"
                        },
                        "outputs": {
                            "type": "string",
                            "description": "Output specifications (address:amount format, comma-separated)"
                        },
                        "fee_rate": {
                            "type": "number",
                            "description": "Fee rate in sat/vB"
                        },
                        "descriptor": {
                            "type": "string",
                            "description": "Output descriptor for BDK backends"
                        },
                        "network": {
                            "type": "string",
                            "description": "Bitcoin network (mainnet, testnet, signet, regtest)"
                        },
                        "backend": {
                            "type": "string",
                            "description": "Backend to use (bitcoind, electrum, esplora)"
                        },
                        "backend_url": {
                            "type": "string",
                            "description": "Backend URL (for electrum/esplora)"
                        },
                        "bitcoin_dir": {
                            "type": "string",
                            "description": "Bitcoin data directory (for bitcoind)"
                        }
                    },
                    "required": ["inputs", "outputs"]
                }),
            ),
            create_tool(
                "create_funded_psbt",
                "Create funded PSBT with automatic input selection",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "outputs": {
                            "type": "string",
                            "description": "Output specifications (address:amount format, comma-separated)"
                        },
                        "inputs": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Optional input specifications (txid:vout format or descriptors)"
                        },
                        "fee_rate": {
                            "type": "number",
                            "description": "Fee rate in sat/vB (overrides conf_target)"
                        },
                        "conf_target": {
                            "type": "integer",
                            "description": "Confirmation target in blocks"
                        },
                        "estimate_mode": {
                            "type": "string",
                            "description": "Fee estimation mode (ECONOMICAL or CONSERVATIVE)"
                        },
                        "descriptor": {
                            "type": "string",
                            "description": "Output descriptor for BDK backends"
                        },
                        "network": {
                            "type": "string",
                            "description": "Bitcoin network (mainnet, testnet, signet, regtest)"
                        },
                        "backend": {
                            "type": "string",
                            "description": "Backend to use (bitcoind, electrum, esplora)"
                        },
                        "backend_url": {
                            "type": "string",
                            "description": "Backend URL"
                        },
                        "bitcoin_dir": {
                            "type": "string",
                            "description": "Bitcoin data directory"
                        }
                    },
                    "required": ["outputs"]
                }),
            ),
            create_tool(
                "move_utxos",
                "Move/consolidate UTXOs to a destination address",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "inputs": {
                            "type": "array",
                            "items": { "type": "string" },
                            "description": "Input specifications (txid:vout format or descriptors)"
                        },
                        "destination": {
                            "type": "string",
                            "description": "Destination Bitcoin address"
                        },
                        "fee_rate": {
                            "type": "number",
                            "description": "Fee rate in sat/vB"
                        },
                        "fee": {
                            "type": "integer",
                            "description": "Absolute fee in satoshis"
                        },
                        "max_amount": {
                            "type": "string",
                            "description": "Maximum amount to move (e.g., '0.5btc' or '50000000sats')"
                        },
                        "descriptor": {
                            "type": "string",
                            "description": "Output descriptor for BDK backends"
                        },
                        "network": {
                            "type": "string",
                            "description": "Bitcoin network (mainnet, testnet, signet, regtest)"
                        },
                        "backend": {
                            "type": "string",
                            "description": "Backend to use (bitcoind, electrum, esplora)"
                        },
                        "backend_url": {
                            "type": "string",
                            "description": "Backend URL"
                        },
                        "bitcoin_dir": {
                            "type": "string",
                            "description": "Bitcoin data directory"
                        }
                    },
                    "required": ["inputs", "destination"]
                }),
            ),
            create_tool(
                "dca_report",
                "Generate Dollar Cost Averaging report for UTXOs",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "descriptor": {
                            "type": "string",
                            "description": "Output descriptor to analyze"
                        },
                        "currency": {
                            "type": "string",
                            "description": "Fiat currency for price data (USD, EUR, GBP)"
                        },
                        "backend": {
                            "type": "string",
                            "description": "Backend to use (bitcoind, electrum, esplora)"
                        },
                        "backend_url": {
                            "type": "string",
                            "description": "Backend URL"
                        },
                        "bitcoin_dir": {
                            "type": "string",
                            "description": "Bitcoin data directory (for bitcoind)"
                        },
                        "cache_dir": {
                            "type": "string",
                            "description": "Cache directory for price data"
                        }
                    },
                    "required": ["descriptor"]
                }),
            ),
        ];

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args = request.arguments.unwrap_or_default();

        match request.name.as_ref() {
            "decode_invoice" => {
                let invoice = args
                    .get("invoice")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_request("invoice parameter required", None))?;
                Ok(self
                    .decode_invoice(DecodeInvoiceRequest {
                        invoice: invoice.to_string(),
                    })
                    .await)
            }
            "decode_lnurl" => {
                let lnurl = args
                    .get("lnurl")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_request("lnurl parameter required", None))?;
                Ok(self
                    .decode_lnurl(DecodeLnurlRequest {
                        lnurl: lnurl.to_string(),
                    })
                    .await)
            }
            "generate_invoice" => {
                let address = args
                    .get("address")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_request("address parameter required", None))?;
                let amount_msats = args
                    .get("amount_msats")
                    .and_then(|v| v.as_u64())
                    .ok_or_else(|| {
                        McpError::invalid_request("amount_msats parameter required", None)
                    })?;
                let comment = args
                    .get("comment")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(self
                    .generate_invoice(GenerateInvoiceRequest {
                        address: address.to_string(),
                        amount_msats,
                        comment,
                    })
                    .await)
            }
            "decode_fedimint_invite" => {
                let invite_code = args
                    .get("invite_code")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_request("invite_code parameter required", None)
                    })?;
                Ok(self
                    .decode_fedimint_invite(DecodeFedimintInviteRequest {
                        invite_code: invite_code.to_string(),
                    })
                    .await)
            }
            "encode_fedimint_invite" => {
                let federation_id = args
                    .get("federation_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_request("federation_id parameter required", None)
                    })?;
                let guardians_json = args.get("guardians").ok_or_else(|| {
                    McpError::invalid_request("guardians parameter required", None)
                })?;
                let guardians: Vec<Guardian> = serde_json::from_value(guardians_json.clone())
                    .map_err(|e| {
                        McpError::invalid_request(format!("Invalid guardians format: {e}"), None)
                    })?;
                let api_secret = args
                    .get("api_secret")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let skip_api_secret = args.get("skip_api_secret").and_then(|v| v.as_bool());
                Ok(self
                    .encode_fedimint_invite(EncodeFedimintInviteRequest {
                        federation_id: federation_id.to_string(),
                        guardians,
                        api_secret,
                        skip_api_secret,
                    })
                    .await)
            }
            "list_utxos" => {
                let descriptor = args
                    .get("descriptor")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let addresses = args.get("addresses").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
                let network = args
                    .get("network")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend = args
                    .get("backend")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend_url = args
                    .get("backend_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let bitcoin_dir = args
                    .get("bitcoin_dir")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(self
                    .list_utxos(ListUtxosRequest {
                        descriptor,
                        addresses,
                        network,
                        backend,
                        backend_url,
                        bitcoin_dir,
                    })
                    .await)
            }
            "decode_psbt" => {
                let psbt = args
                    .get("psbt")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_request("psbt parameter required", None))?;
                Ok(self
                    .decode_psbt(DecodePsbtRequest {
                        psbt: psbt.to_string(),
                    })
                    .await)
            }
            "create_psbt" => {
                let inputs_json = args
                    .get("inputs")
                    .ok_or_else(|| McpError::invalid_request("inputs parameter required", None))?;
                let inputs: Vec<String> =
                    serde_json::from_value(inputs_json.clone()).map_err(|e| {
                        McpError::invalid_request(format!("Invalid inputs format: {e}"), None)
                    })?;
                let outputs = args
                    .get("outputs")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_request("outputs parameter required", None))?;
                let fee_rate = args.get("fee_rate").and_then(|v| v.as_f64());
                let descriptor = args
                    .get("descriptor")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let network = args
                    .get("network")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend = args
                    .get("backend")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend_url = args
                    .get("backend_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let bitcoin_dir = args
                    .get("bitcoin_dir")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(self
                    .create_psbt(CreatePsbtRequest {
                        inputs,
                        outputs: outputs.to_string(),
                        fee_rate,
                        descriptor,
                        network,
                        backend,
                        backend_url,
                        bitcoin_dir,
                    })
                    .await)
            }
            "create_funded_psbt" => {
                let outputs = args
                    .get("outputs")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| McpError::invalid_request("outputs parameter required", None))?;
                let inputs = args.get("inputs").and_then(|v| v.as_array()).map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
                let fee_rate = args.get("fee_rate").and_then(|v| v.as_f64());
                let conf_target = args
                    .get("conf_target")
                    .and_then(|v| v.as_u64())
                    .map(|v| v as u32);
                let estimate_mode = args
                    .get("estimate_mode")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let descriptor = args
                    .get("descriptor")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let network = args
                    .get("network")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend = args
                    .get("backend")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend_url = args
                    .get("backend_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let bitcoin_dir = args
                    .get("bitcoin_dir")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(self
                    .create_funded_psbt(CreateFundedPsbtRequest {
                        outputs: outputs.to_string(),
                        inputs,
                        fee_rate,
                        conf_target,
                        estimate_mode,
                        descriptor,
                        network,
                        backend,
                        backend_url,
                        bitcoin_dir,
                    })
                    .await)
            }
            "move_utxos" => {
                let inputs_json = args
                    .get("inputs")
                    .ok_or_else(|| McpError::invalid_request("inputs parameter required", None))?;
                let inputs: Vec<String> =
                    serde_json::from_value(inputs_json.clone()).map_err(|e| {
                        McpError::invalid_request(format!("Invalid inputs format: {e}"), None)
                    })?;
                let destination = args
                    .get("destination")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_request("destination parameter required", None)
                    })?;
                let fee_rate = args.get("fee_rate").and_then(|v| v.as_f64());
                let fee = args.get("fee").and_then(|v| v.as_u64());
                let max_amount = args
                    .get("max_amount")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let descriptor = args
                    .get("descriptor")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let network = args
                    .get("network")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend = args
                    .get("backend")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend_url = args
                    .get("backend_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let bitcoin_dir = args
                    .get("bitcoin_dir")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(self
                    .move_utxos(MoveUtxosRequest {
                        inputs,
                        destination: destination.to_string(),
                        fee_rate,
                        fee,
                        max_amount,
                        descriptor,
                        network,
                        backend,
                        backend_url,
                        bitcoin_dir,
                    })
                    .await)
            }
            "dca_report" => {
                let descriptor =
                    args.get("descriptor")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            McpError::invalid_request("descriptor parameter required", None)
                        })?;
                let currency = args
                    .get("currency")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend = args
                    .get("backend")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let backend_url = args
                    .get("backend_url")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let bitcoin_dir = args
                    .get("bitcoin_dir")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let cache_dir = args
                    .get("cache_dir")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                Ok(self
                    .dca_report(DcaReportRequest {
                        descriptor: descriptor.to_string(),
                        currency,
                        backend,
                        backend_url,
                        bitcoin_dir,
                        cache_dir,
                    })
                    .await)
            }
            _ => Err(McpError::invalid_request(
                format!("Tool '{}' not found", request.name),
                None,
            )),
        }
    }
}
