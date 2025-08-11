use anyhow::Result;
use rmcp::{
    handler::server::ServerHandler,
    model::{ServerCapabilities, ServerInfo},
    schemars,
    service::ServiceExt,
    tool,
    transport::stdio,
};
use serde::Deserialize;
use std::future::Future;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

/// Configuration for the MCP server
#[derive(Debug, Clone)]
pub struct McpServerConfig {
    pub transport: Transport,
    pub host: String,
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

impl CyberkrillMcpServer {
    pub fn new(config: McpServerConfig) -> Self {
        Self {
            config,
            state: Arc::new(Mutex::new(ServerState::default())),
        }
    }

    /// Start the MCP server
    pub async fn run(self) -> Result<()> {
        // Initialize tracing
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
                    .add_directive(tracing::Level::INFO.into()),
            )
            .init();

        info!("Starting cyberkrill MCP server");

        match self.config.transport {
            Transport::Stdio => {
                info!("Starting MCP server with stdio transport");
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
    ) -> String {
        match cyberkrill_core::decode_invoice(&invoice) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    #[tool(description = "Decode an LNURL string")]
    async fn decode_lnurl(
        &self,
        DecodeLnurlRequest { lnurl }: DecodeLnurlRequest,
    ) -> String {
        match cyberkrill_core::decode_lnurl(&lnurl) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {}", e),
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
    ) -> String {
        match cyberkrill_core::generate_invoice_from_address(
            &address,
            amount_msats,
            comment.as_deref(),
        )
        .await
        {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {}", e),
        }
    }

    // Fedimint tools
    #[tool(description = "Decode a Fedimint federation invite code")]
    async fn decode_fedimint_invite(
        &self,
        DecodeFedimintInviteRequest { invite_code }: DecodeFedimintInviteRequest,
    ) -> String {
        match fedimint_lite::decode_invite(&invite_code) {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string()),
            Err(e) => format!("Error: {}", e),
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
    ) -> String {
        // We need to build the FedimintInviteOutput structure directly
        let guardians_info = guardians
            .into_iter()
            .map(|g| {
                fedimint_lite::GuardianInfo {
                    peer_id: g.peer_id as u16,
                    url: g.url,
                }
            })
            .collect();
        
        let invite = fedimint_lite::FedimintInviteOutput {
            federation_id,
            guardians: guardians_info,
            api_secret: if skip_api_secret.unwrap_or(false) { None } else { api_secret },
            encoding_format: "bech32m".to_string(),
        };
        
        match fedimint_lite::encode_fedimint_invite(&invite) {
            Ok(result) => serde_json::json!({ "invite_code": result }).to_string(),
            Err(e) => format!("Error: {}", e),
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
    ) -> String {
        let network_str = network.as_deref().unwrap_or("mainnet");
        let network = match network_str.to_lowercase().as_str() {
            "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
            "testnet" => cyberkrill_core::Network::Testnet,
            "signet" => cyberkrill_core::Network::Signet,
            "regtest" => cyberkrill_core::Network::Regtest,
            _ => return format!("Invalid network: {}", network_str),
        };

        let backend_type = backend.as_deref().unwrap_or("bitcoind");

        let result = if let Some(desc) = descriptor {
            match backend_type {
                "electrum" => {
                    if let Some(url) = backend_url {
                        match cyberkrill_core::scan_and_list_utxos_electrum(&desc, network, &url, 200).await {
                            Ok(r) => r,
                            Err(e) => return format!("Error: {}", e),
                        }
                    } else {
                        return "Error: backend_url required for electrum".to_string();
                    }
                }
                "esplora" => {
                    if let Some(url) = backend_url {
                        match cyberkrill_core::scan_and_list_utxos_esplora(&desc, network, &url, 200).await {
                            Ok(r) => r,
                            Err(e) => return format!("Error: {}", e),
                        }
                    } else {
                        return "Error: backend_url required for esplora".to_string();
                    }
                }
                _ => {
                    let dir = bitcoin_dir.as_deref().unwrap_or("~/.bitcoin");
                    let path = std::path::Path::new(dir);
                    match cyberkrill_core::scan_and_list_utxos_bitcoind(&desc, network, path).await {
                        Ok(r) => r,
                        Err(e) => return format!("Error: {}", e),
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
                Err(e) => return format!("Error creating client: {}", e),
            };

            let utxo_response = match client.list_utxos_for_addresses(addrs).await {
                Ok(r) => r,
                Err(e) => return format!("Error: {}", e),
            };
            
            // Convert UtxoListResponse to Vec<BdkUtxo> for compatibility
            utxo_response.utxos.into_iter().map(|utxo| {
                cyberkrill_core::BdkUtxo {
                    txid: utxo.txid.clone(),
                    vout: utxo.vout,
                    address: utxo.address.clone().unwrap_or_default(),
                    amount: utxo.amount_sats,
                    amount_btc: utxo.amount_sats as f64 / 100_000_000.0,
                    confirmations: utxo.confirmations,
                    is_change: false,  // We don't know from the RPC response
                    keychain: "External".to_string(),  // Default to external
                    derivation_index: None,  // Not available from RPC
                }
            }).collect()
        } else {
            return "Error: Either descriptor or addresses required".to_string();
        };

        let summary = cyberkrill_core::get_utxo_summary(result);
        serde_json::to_string_pretty(&summary).unwrap_or_else(|e| e.to_string())
    }

    #[tool(description = "Decode a PSBT (Partially Signed Bitcoin Transaction)")]
    async fn decode_psbt(
        &self,
        DecodePsbtRequest { psbt }: DecodePsbtRequest,
    ) -> String {
        use base64::Engine;

        let psbt_bytes = if psbt.starts_with("cHNidP") {
            match base64::engine::general_purpose::STANDARD.decode(&psbt) {
                Ok(b) => b,
                Err(e) => return format!("Error decoding base64: {}", e),
            }
        } else {
            match hex::decode(&psbt) {
                Ok(b) => b,
                Err(e) => return format!("Error decoding hex: {}", e),
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
                serde_json::to_string_pretty(&result).unwrap_or_else(|e| e.to_string())
            }
            Err(e) => format!("Error parsing PSBT: {}", e),
        }
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
}