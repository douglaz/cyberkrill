use anyhow::{bail, Context, Result};
use base64::Engine;
use bdk_wallet::{KeychainKind, Wallet};
use bitcoin::{Amount, FeeRate, Network, OutPoint, Txid};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

/// UTXO information returned by BDK wallet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BdkUtxo {
    /// Transaction ID
    pub txid: String,
    /// Output index
    pub vout: u32,
    /// Bitcoin address
    pub address: String,
    /// Amount in satoshis
    pub amount: u64,
    /// Amount in BTC
    pub amount_btc: f64,
    /// Number of confirmations
    pub confirmations: u32,
    /// Whether this is a change output
    pub is_change: bool,
    /// Keychain type (External or Internal)
    pub keychain: String,
    /// Derivation index
    pub derivation_index: Option<u32>,
}

/// Expand multipath descriptors (e.g., <0;1>) into individual descriptors
///
/// BDK 2.0 doesn't natively support multipath descriptors, which are commonly used
/// in wallet descriptors to specify both external (0) and internal/change (1) derivation paths.
/// This function expands descriptors like "wpkh(xpub.../<0;1>/*)" into:
/// - "wpkh(xpub.../0/*)" for external addresses
/// - "wpkh(xpub.../1/*)" for internal (change) addresses
///
/// This is necessary for compatibility with descriptors from other tools like
/// Bitcoin Core's `getdescriptorinfo` which often use this notation.
fn expand_multipath_descriptor(descriptor: &str) -> Vec<String> {
    if descriptor.contains("<") && descriptor.contains(">") {
        // Extract the multipath part and expand it
        let mut expanded = Vec::new();

        // For now, handle the simple case of <0;1>
        if descriptor.contains("<0;1>") {
            expanded.push(descriptor.replace("<0;1>", "0"));
            expanded.push(descriptor.replace("<0;1>", "1"));
        } else {
            // If it's a different multipath pattern, just return the original
            expanded.push(descriptor.to_string());
        }

        expanded
    } else {
        vec![descriptor.to_string()]
    }
}

/// List UTXOs using BDK wallet
pub fn list_utxos_bdk(descriptor: &str, network: Network) -> Result<Vec<BdkUtxo>> {
    let descriptors = expand_multipath_descriptor(descriptor);
    let mut all_utxos = Vec::new();

    for desc in descriptors {
        // Create a new in-memory wallet with only external descriptor
        match Wallet::create_single(desc.clone())
            .network(network)
            .create_wallet_no_persist()
        {
            Ok(wallet) => {
                // Get the wallet's UTXOs
                let utxos = wallet.list_unspent();

                for utxo in utxos {
                    // Get the address for this output
                    let script = &utxo.txout.script_pubkey;
                    let address = bitcoin::Address::from_script(script, network)
                        .context("Failed to derive address from script")?;

                    // Determine if this is a change output
                    let keychain = match utxo.keychain {
                        KeychainKind::External => "external",
                        KeychainKind::Internal => "internal",
                    };
                    let is_change = utxo.keychain == KeychainKind::Internal;

                    // Calculate confirmations based on chain position
                    let confirmations = match &utxo.chain_position {
                        bdk_wallet::chain::ChainPosition::Confirmed { anchor, .. } => {
                            anchor.block_id.height
                        }
                        bdk_wallet::chain::ChainPosition::Unconfirmed { .. } => 0,
                    };

                    all_utxos.push(BdkUtxo {
                        txid: utxo.outpoint.txid.to_string(),
                        vout: utxo.outpoint.vout,
                        address: address.to_string(),
                        amount: utxo.txout.value.to_sat(),
                        amount_btc: utxo.txout.value.to_btc(),
                        confirmations,
                        is_change,
                        keychain: keychain.to_string(),
                        derivation_index: None,
                    });
                }
            }
            Err(e) => {
                // If this descriptor fails, log it but continue with others
                eprintln!("Warning: Failed to create wallet for descriptor '{desc}': {e}");
            }
        }
    }

    // Sort by amount descending for consistency
    all_utxos.sort_by(|a, b| b.amount.cmp(&a.amount));

    Ok(all_utxos)
}

/// Scan blockchain for UTXOs using BDK wallet with Electrum backend
pub async fn scan_and_list_utxos_electrum(
    descriptor: &str,
    network: Network,
    electrum_url: &str,
    stop_gap: u32,
) -> Result<Vec<BdkUtxo>> {
    use bdk_electrum::{electrum_client, BdkElectrumClient};

    let descriptors = expand_multipath_descriptor(descriptor);
    let mut all_utxos = Vec::new();

    // Create Electrum client once for all descriptors
    let client = BdkElectrumClient::new(
        electrum_client::Client::new(electrum_url).context("Failed to create Electrum client")?,
    );

    for desc in descriptors {
        // Create wallet with only external descriptor
        let wallet_result = Wallet::create_single(desc.clone())
            .network(network)
            .create_wallet_no_persist();

        let mut wallet = match wallet_result {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Warning: Failed to create wallet for descriptor '{desc}': {e}");
                continue;
            }
        };

        // Create a full scan request
        let request = wallet
            .start_full_scan()
            .inspect({
                move |keychain, spk_i, _| {
                    // Progress output to stderr to keep stdout clean for JSON
                    eprint!("\rScanning {keychain:?} {spk_i}...");
                }
            })
            .build();

        // Perform the scan
        match client.full_scan(request, stop_gap as usize, 10, false) {
            Ok(update) => {
                // Apply the update to the wallet
                if let Err(e) = wallet.apply_update(update) {
                    eprintln!("Warning: Failed to apply update for descriptor '{desc}': {e}");
                    continue;
                }

                // Get current tip height for confirmation calculations
                let tip_height = wallet.latest_checkpoint().height();

                // List unspent outputs
                let utxos = wallet.list_unspent();

                for utxo in utxos {
                    // Get the address for this output
                    let script = &utxo.txout.script_pubkey;
                    match bitcoin::Address::from_script(script, network) {
                        Ok(address) => {
                            // Determine keychain type
                            let keychain = match utxo.keychain {
                                KeychainKind::External => "external",
                                KeychainKind::Internal => "internal",
                            };
                            let is_change = utxo.keychain == KeychainKind::Internal;

                            // Calculate confirmations based on chain position
                            let confirmations = match &utxo.chain_position {
                                bdk_wallet::chain::ChainPosition::Confirmed { anchor, .. } => {
                                    let block_height = anchor.block_id.height;
                                    if tip_height >= block_height {
                                        tip_height - block_height + 1
                                    } else {
                                        0
                                    }
                                }
                                bdk_wallet::chain::ChainPosition::Unconfirmed { .. } => 0,
                            };

                            all_utxos.push(BdkUtxo {
                                txid: utxo.outpoint.txid.to_string(),
                                vout: utxo.outpoint.vout,
                                address: address.to_string(),
                                amount: utxo.txout.value.to_sat(),
                                amount_btc: utxo.txout.value.to_btc(),
                                confirmations,
                                is_change,
                                keychain: keychain.to_string(),
                                derivation_index: None,
                            });
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to derive address from script: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to scan with Electrum for descriptor '{desc}': {e}");
            }
        }
    }

    // Sort by amount descending
    all_utxos.sort_by(|a, b| b.amount.cmp(&a.amount));

    Ok(all_utxos)
}

/// Scan blockchain for UTXOs using BDK wallet with Bitcoin Core RPC backend
///
/// This function bridges BDK wallet functionality with Bitcoin Core RPC. It uses
/// the existing BitcoinRpcClient to query UTXOs via `scantxoutset` and converts
/// the results to BDK-compatible structures.
///
/// # Important Implementation Details:
///
/// 1. **Address Derivation**: Bitcoin Core's `scantxoutset` doesn't return addresses,
///    only scriptPubKey hex. We derive addresses from the scriptPubKey to provide
///    a complete UTXO representation.
///
/// 2. **Why not use bdk_bitcoind_rpc?**: The bdk_bitcoind_rpc crate underwent major
///    API changes in v0.20 that are incompatible with the current BDK 2.0 release.
///    Using the existing RPC client provides a stable integration path.
///
/// 3. **Missing BDK metadata**: Some BDK-specific fields (keychain type, derivation
///    index) aren't available from Bitcoin Core and are filled with defaults.
pub async fn scan_and_list_utxos_bitcoind(
    descriptor: &str,
    _network: Network,
    bitcoin_dir: &Path,
) -> Result<Vec<BdkUtxo>> {
    use crate::BitcoinRpcClient;

    let mut all_utxos = Vec::new();

    // Use our existing Bitcoin RPC client to get UTXOs for the descriptor
    let client = BitcoinRpcClient::new_auto(
        "http://127.0.0.1:8332".to_string(),
        Some(bitcoin_dir),
        None,
        None,
    )?;

    // Use the existing list_utxos_for_descriptor method
    let utxo_result = client.list_utxos_for_descriptor(descriptor).await?;

    // Convert from RPC UTXOs to BDK UTXOs
    for utxo in utxo_result.utxos {
        // Try to derive address from scriptPubKey if not provided
        // This is a critical step because Bitcoin Core's scantxoutset doesn't include addresses
        // in the response. Without this, UTXOs would appear to be "missing" if code filters
        // by address presence (a common pattern that caused issues during development).
        let address = if let Some(addr) = utxo.address {
            addr
        } else {
            // Try to decode the scriptPubKey hex and derive address
            match hex::decode(&utxo.script_pub_key) {
                Ok(script_bytes) => {
                    let script = bitcoin::ScriptBuf::from(script_bytes);
                    match bitcoin::Address::from_script(&script, _network) {
                        Ok(addr) => addr.to_string(),
                        Err(_) => format!("script:{script}", script = utxo.script_pub_key), // Fallback to script hex
                    }
                }
                Err(_) => format!("script:{script}", script = utxo.script_pub_key), // Fallback to script hex
            }
        };

        all_utxos.push(BdkUtxo {
            txid: utxo.txid,
            vout: utxo.vout,
            address,
            amount: utxo.amount_sats,
            amount_btc: bitcoin::Amount::from_sat(utxo.amount_sats).to_btc(),
            confirmations: utxo.confirmations,
            is_change: false,                // We don't have this info from RPC
            keychain: "unknown".to_string(), // We don't have this info from RPC
            derivation_index: None,
        });
    }

    Ok(all_utxos)
}

/// Scan blockchain for UTXOs using BDK wallet with Esplora backend
pub async fn scan_and_list_utxos_esplora(
    descriptor: &str,
    network: Network,
    esplora_url: &str,
    stop_gap: u32,
) -> Result<Vec<BdkUtxo>> {
    use bdk_esplora::{esplora_client, EsploraExt};

    let descriptors = expand_multipath_descriptor(descriptor);
    let mut all_utxos = Vec::new();

    // Create Esplora client once for all descriptors
    let client = esplora_client::Builder::new(esplora_url).build_blocking();

    for desc in descriptors {
        // Create wallet with only external descriptor
        let wallet_result = Wallet::create_single(desc.clone())
            .network(network)
            .create_wallet_no_persist();

        let mut wallet = match wallet_result {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Warning: Failed to create wallet for descriptor '{desc}': {e}");
                continue;
            }
        };

        // Create a full scan request
        let request = wallet
            .start_full_scan()
            .inspect({
                move |keychain, spk_i, _| {
                    // Progress output to stderr to keep stdout clean for JSON
                    eprint!("\rScanning {keychain:?} {spk_i}...");
                }
            })
            .build();

        // Perform the scan with Esplora
        match client.full_scan(request, stop_gap as usize, 10) {
            Ok(update) => {
                // Apply the update to the wallet
                if let Err(e) = wallet.apply_update(update) {
                    eprintln!("Warning: Failed to apply update for descriptor '{desc}': {e}");
                    continue;
                }

                // Get current tip height for confirmation calculations
                let tip_height = wallet.latest_checkpoint().height();

                // List unspent outputs
                let utxos = wallet.list_unspent();

                for utxo in utxos {
                    // Get the address for this output
                    let script = &utxo.txout.script_pubkey;
                    match bitcoin::Address::from_script(script, network) {
                        Ok(address) => {
                            // Determine keychain type
                            let keychain = match utxo.keychain {
                                KeychainKind::External => "external",
                                KeychainKind::Internal => "internal",
                            };
                            let is_change = utxo.keychain == KeychainKind::Internal;

                            // Calculate confirmations based on chain position
                            let confirmations = match &utxo.chain_position {
                                bdk_wallet::chain::ChainPosition::Confirmed { anchor, .. } => {
                                    let block_height = anchor.block_id.height;
                                    if tip_height >= block_height {
                                        tip_height - block_height + 1
                                    } else {
                                        0
                                    }
                                }
                                bdk_wallet::chain::ChainPosition::Unconfirmed { .. } => 0,
                            };

                            all_utxos.push(BdkUtxo {
                                txid: utxo.outpoint.txid.to_string(),
                                vout: utxo.outpoint.vout,
                                address: address.to_string(),
                                amount: utxo.txout.value.to_sat(),
                                amount_btc: utxo.txout.value.to_btc(),
                                confirmations,
                                is_change,
                                keychain: keychain.to_string(),
                                derivation_index: None,
                            });
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to derive address from script: {e}");
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to scan with Esplora for descriptor '{desc}': {e}");
            }
        }
    }

    // Sort by amount descending
    all_utxos.sort_by(|a, b| b.amount.cmp(&a.amount));

    Ok(all_utxos)
}

/// Summary of UTXOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BdkUtxoSummary {
    /// Total count of UTXOs
    pub total_count: usize,
    /// Total amount in satoshis
    pub total_amount: u64,
    /// Total amount in BTC
    pub total_amount_btc: f64,
    /// Count by confirmation status
    pub confirmed_count: usize,
    pub unconfirmed_count: usize,
    /// List of UTXOs
    pub utxos: Vec<BdkUtxo>,
}

/// Get UTXO summary from a list of UTXOs
pub fn get_utxo_summary(utxos: Vec<BdkUtxo>) -> BdkUtxoSummary {
    let total_amount: u64 = utxos.iter().map(|u| u.amount).sum();
    let confirmed_count = utxos.iter().filter(|u| u.confirmations > 0).count();
    let unconfirmed_count = utxos.iter().filter(|u| u.confirmations == 0).count();

    BdkUtxoSummary {
        total_count: utxos.len(),
        total_amount,
        total_amount_btc: bitcoin::Amount::from_sat(total_amount).to_btc(),
        confirmed_count,
        unconfirmed_count,
        utxos,
    }
}

/// Response structure for PSBT creation
#[derive(Debug, Serialize, Deserialize)]
pub struct BdkPsbtResponse {
    /// Base64-encoded PSBT
    pub psbt: String,
    /// Fee amount in satoshis
    pub fee_sats: u64,
    /// Change output position (if any)
    pub change_position: Option<u32>,
}

/// Input structure that can be either a UTXO (txid:vout) or a descriptor
#[derive(Debug, Clone)]
pub enum InputSpec {
    /// Specific UTXO: txid:vout
    Utxo { txid: Txid, vout: u32 },
    /// Descriptor to expand into UTXOs
    Descriptor(String),
}

impl FromStr for InputSpec {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Check if this looks like a descriptor (contains parentheses or brackets)
        if s.contains('(') || s.contains('[') {
            Ok(InputSpec::Descriptor(s.to_string()))
        } else {
            // Try to parse as txid:vout
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 2 {
                bail!(
                    "Invalid input format: '{}'. Expected 'txid:vout' or a descriptor",
                    s
                );
            }
            let txid = Txid::from_str(parts[0]).context("Invalid transaction ID")?;
            let vout: u32 = parts[1].parse().context("Invalid output index")?;
            Ok(InputSpec::Utxo { txid, vout })
        }
    }
}

/// Create a PSBT with manual input/output specification using BDK
pub async fn create_psbt_bdk(
    inputs: &[String],
    outputs: &[(String, Amount)],
    fee_rate: Option<f64>, // sat/vB
    descriptor: &str,
    network: Network,
    backend: &str,
) -> Result<BdkPsbtResponse> {
    // Create wallet and sync with backend
    let mut wallet = Wallet::create_single(descriptor.to_string())
        .network(network)
        .create_wallet_no_persist()?;

    // Sync wallet with blockchain and get UTXOs
    let utxos = if backend.starts_with("electrum://") {
        let url = backend.strip_prefix("electrum://").unwrap();
        use bdk_electrum::{electrum_client, BdkElectrumClient};

        let client = BdkElectrumClient::new(
            electrum_client::Client::new(url).context("Failed to create Electrum client")?,
        );

        let request = wallet.start_full_scan().build();
        let update = client.full_scan(request, 200, 10, false)?;
        wallet.apply_update(update)?;

        // Now get the UTXOs from the synced wallet
        let wallet_utxos = wallet.list_unspent();
        let mut utxos = Vec::new();
        for utxo in wallet_utxos {
            utxos.push(BdkUtxo {
                txid: utxo.outpoint.txid.to_string(),
                vout: utxo.outpoint.vout,
                address: bitcoin::Address::from_script(&utxo.txout.script_pubkey, network)?
                    .to_string(),
                amount: utxo.txout.value.to_sat(),
                amount_btc: utxo.txout.value.to_btc(),
                confirmations: 0, // Not needed for this use case
                is_change: utxo.keychain == KeychainKind::Internal,
                keychain: match utxo.keychain {
                    KeychainKind::External => "external",
                    KeychainKind::Internal => "internal",
                }
                .to_string(),
                derivation_index: None,
            });
        }
        utxos
    } else if backend.starts_with("esplora://") {
        let url = backend.strip_prefix("esplora://").unwrap();
        use bdk_esplora::{esplora_client, EsploraExt};

        let client = esplora_client::Builder::new(url).build_blocking();
        let request = wallet.start_full_scan().build();
        let update = client.full_scan(request, 200, 10)?;
        wallet.apply_update(update)?;

        // Now get the UTXOs from the synced wallet
        let wallet_utxos = wallet.list_unspent();
        let mut utxos = Vec::new();
        for utxo in wallet_utxos {
            utxos.push(BdkUtxo {
                txid: utxo.outpoint.txid.to_string(),
                vout: utxo.outpoint.vout,
                address: bitcoin::Address::from_script(&utxo.txout.script_pubkey, network)?
                    .to_string(),
                amount: utxo.txout.value.to_sat(),
                amount_btc: utxo.txout.value.to_btc(),
                confirmations: 0, // Not needed for this use case
                is_change: utxo.keychain == KeychainKind::Internal,
                keychain: match utxo.keychain {
                    KeychainKind::External => "external",
                    KeychainKind::Internal => "internal",
                }
                .to_string(),
                derivation_index: None,
            });
        }
        utxos
    } else if backend.starts_with("bitcoind://") {
        let path_str = backend.strip_prefix("bitcoind://").unwrap();
        let path = Path::new(path_str);
        scan_and_list_utxos_bitcoind(descriptor, network, path).await?
    } else {
        bail!(
            "Unsupported backend: {}. Expected electrum://, esplora://, or bitcoind://",
            backend
        )
    };

    // Parse inputs
    let mut input_specs = Vec::new();
    for input in inputs {
        input_specs.push(InputSpec::from_str(input)?);
    }

    // Expand descriptors to UTXOs
    let mut selected_utxos = Vec::new();
    for spec in input_specs {
        match spec {
            InputSpec::Utxo { txid, vout } => {
                // Find this specific UTXO in our wallet
                let utxo = utxos
                    .iter()
                    .find(|u| u.txid == txid.to_string() && u.vout == vout)
                    .ok_or_else(|| anyhow::anyhow!("UTXO {}:{} not found in wallet", txid, vout))?;
                selected_utxos.push(utxo.clone());
            }
            InputSpec::Descriptor(_desc) => {
                // For descriptors, we need to scan and add all UTXOs
                // This is a simplified version - in reality we'd want to expand the descriptor
                bail!("Descriptor input expansion not yet implemented for BDK PSBT creation");
            }
        }
    }

    // Build transaction
    let mut tx_builder = wallet.build_tx();

    // Add inputs
    for utxo in &selected_utxos {
        let outpoint = OutPoint {
            txid: Txid::from_str(&utxo.txid)?,
            vout: utxo.vout,
        };
        tx_builder.add_utxo(outpoint)?;
    }

    // Add outputs
    for (address, amount) in outputs {
        let script = bitcoin::Address::from_str(address)?
            .require_network(network)?
            .script_pubkey();
        tx_builder.add_recipient(script, *amount);
    }

    // Set fee rate if provided
    if let Some(rate) = fee_rate {
        // BDK expects fee rate in sat/vB
        tx_builder.fee_rate(FeeRate::from_sat_per_vb(rate as u64).expect("Valid fee rate"));
    }

    // Manually select UTXOs (disable coin selection)
    tx_builder.manually_selected_only();

    // Finish building
    let psbt = tx_builder.finish()?;

    // Calculate fee
    let fee = psbt.fee()?;

    // Find change position if any
    let change_position = None; // TODO: Detect change output

    // Serialize PSBT to base64
    let psbt_bytes = psbt.serialize();
    let psbt_base64 = base64::engine::general_purpose::STANDARD.encode(&psbt_bytes);

    Ok(BdkPsbtResponse {
        psbt: psbt_base64,
        fee_sats: fee.to_sat(),
        change_position,
    })
}

/// Create a funded PSBT with automatic input selection using BDK
pub async fn create_funded_psbt_bdk(
    outputs: &[(String, Amount)],
    conf_target: Option<u32>,
    fee_rate: Option<f64>, // sat/vB
    descriptor: &str,
    network: Network,
    backend: &str,
) -> Result<BdkPsbtResponse> {
    // Create wallet and sync with backend
    let mut wallet = Wallet::create_single(descriptor.to_string())
        .network(network)
        .create_wallet_no_persist()?;

    // Sync wallet with blockchain
    if backend.starts_with("electrum://") {
        let url = backend.strip_prefix("electrum://").unwrap();
        use bdk_electrum::{electrum_client, BdkElectrumClient};

        let client = BdkElectrumClient::new(
            electrum_client::Client::new(url).context("Failed to create Electrum client")?,
        );

        let request = wallet.start_full_scan().build();
        let update = client.full_scan(request, 200, 10, false)?;
        wallet.apply_update(update)?;
    } else if backend.starts_with("esplora://") {
        let url = backend.strip_prefix("esplora://").unwrap();
        use bdk_esplora::{esplora_client, EsploraExt};

        let client = esplora_client::Builder::new(url).build_blocking();
        let request = wallet.start_full_scan().build();
        let update = client.full_scan(request, 200, 10)?;
        wallet.apply_update(update)?;
    } else if backend.starts_with("bitcoind://") {
        // For Bitcoin Core, we'll use the existing RPC approach
        // since BDK's bitcoind integration is limited
        let path_str = backend.strip_prefix("bitcoind://").unwrap();
        let path = Path::new(path_str);
        let _utxos = scan_and_list_utxos_bitcoind(descriptor, network, path).await?;
        // Note: Bitcoin Core backend doesn't provide the same update mechanism
        // so the wallet won't be fully aware of pending transactions
    } else {
        bail!(
            "Unsupported backend: {}. Expected electrum://, esplora://, or bitcoind://",
            backend
        )
    }

    // Build transaction
    let mut tx_builder = wallet.build_tx();

    // Add outputs
    for (address, amount) in outputs {
        let script = bitcoin::Address::from_str(address)?
            .require_network(network)?
            .script_pubkey();
        tx_builder.add_recipient(script, *amount);
    }

    // Set fee rate
    if let Some(rate) = fee_rate {
        // BDK expects fee rate in sat/vB
        tx_builder.fee_rate(FeeRate::from_sat_per_vb(rate as u64).expect("Valid fee rate"));
    } else if let Some(_target) = conf_target {
        // TODO: Implement fee estimation based on confirmation target
        // For now, use a default fee rate
        tx_builder.fee_rate(FeeRate::from_sat_per_vb(10).expect("Valid fee rate"));
    }

    // Enable RBF is not available in BDK 2.0
    // tx_builder.enable_rbf();

    // Finish building
    let psbt = tx_builder.finish()?;

    // Calculate fee
    let fee = psbt.fee()?;

    // Find change position
    let change_position = None; // TODO: Detect change output position

    // Serialize PSBT to base64
    let psbt_bytes = psbt.serialize();
    let psbt_base64 = base64::engine::general_purpose::STANDARD.encode(&psbt_bytes);

    Ok(BdkPsbtResponse {
        psbt: psbt_base64,
        fee_sats: fee.to_sat(),
        change_position,
    })
}

/// Move/consolidate UTXOs to a single destination using BDK
#[allow(clippy::too_many_arguments)]
pub async fn move_utxos_bdk(
    inputs: &[String],
    destination: &str,
    fee_rate: Option<f64>,
    fee_sats: Option<u64>,
    max_amount: Option<Amount>,
    descriptor: &str,
    network: Network,
    backend: &str,
) -> Result<BdkPsbtResponse> {
    // Create wallet and sync with backend
    let mut wallet = Wallet::create_single(descriptor.to_string())
        .network(network)
        .create_wallet_no_persist()?;

    // Sync wallet with blockchain and get UTXOs
    let utxos = if backend.starts_with("electrum://") {
        let url = backend.strip_prefix("electrum://").unwrap();
        use bdk_electrum::{electrum_client, BdkElectrumClient};

        let client = BdkElectrumClient::new(
            electrum_client::Client::new(url).context("Failed to create Electrum client")?,
        );

        let request = wallet.start_full_scan().build();
        let update = client.full_scan(request, 200, 10, false)?;
        wallet.apply_update(update)?;

        // Now get the UTXOs from the synced wallet
        let wallet_utxos = wallet.list_unspent();
        let mut utxos = Vec::new();
        for utxo in wallet_utxos {
            utxos.push(BdkUtxo {
                txid: utxo.outpoint.txid.to_string(),
                vout: utxo.outpoint.vout,
                address: bitcoin::Address::from_script(&utxo.txout.script_pubkey, network)?
                    .to_string(),
                amount: utxo.txout.value.to_sat(),
                amount_btc: utxo.txout.value.to_btc(),
                confirmations: 0, // Not needed for this use case
                is_change: utxo.keychain == KeychainKind::Internal,
                keychain: match utxo.keychain {
                    KeychainKind::External => "external",
                    KeychainKind::Internal => "internal",
                }
                .to_string(),
                derivation_index: None,
            });
        }
        utxos
    } else if backend.starts_with("esplora://") {
        let url = backend.strip_prefix("esplora://").unwrap();
        use bdk_esplora::{esplora_client, EsploraExt};

        let client = esplora_client::Builder::new(url).build_blocking();
        let request = wallet.start_full_scan().build();
        let update = client.full_scan(request, 200, 10)?;
        wallet.apply_update(update)?;

        // Now get the UTXOs from the synced wallet
        let wallet_utxos = wallet.list_unspent();
        let mut utxos = Vec::new();
        for utxo in wallet_utxos {
            utxos.push(BdkUtxo {
                txid: utxo.outpoint.txid.to_string(),
                vout: utxo.outpoint.vout,
                address: bitcoin::Address::from_script(&utxo.txout.script_pubkey, network)?
                    .to_string(),
                amount: utxo.txout.value.to_sat(),
                amount_btc: utxo.txout.value.to_btc(),
                confirmations: 0, // Not needed for this use case
                is_change: utxo.keychain == KeychainKind::Internal,
                keychain: match utxo.keychain {
                    KeychainKind::External => "external",
                    KeychainKind::Internal => "internal",
                }
                .to_string(),
                derivation_index: None,
            });
        }
        utxos
    } else if backend.starts_with("bitcoind://") {
        let path_str = backend.strip_prefix("bitcoind://").unwrap();
        let path = Path::new(path_str);
        scan_and_list_utxos_bitcoind(descriptor, network, path).await?
    } else {
        bail!(
            "Unsupported backend: {}. Expected electrum://, esplora://, or bitcoind://",
            backend
        )
    };

    // Parse inputs
    let mut input_specs = Vec::new();
    for input in inputs {
        input_specs.push(InputSpec::from_str(input)?);
    }

    // Expand descriptors to UTXOs
    let mut selected_utxos = Vec::new();
    for spec in input_specs {
        match spec {
            InputSpec::Utxo { txid, vout } => {
                // Find this specific UTXO
                let utxo = utxos
                    .iter()
                    .find(|u| u.txid == txid.to_string() && u.vout == vout)
                    .ok_or_else(|| anyhow::anyhow!("UTXO {}:{} not found", txid, vout))?;
                selected_utxos.push(utxo.clone());
            }
            InputSpec::Descriptor(_desc) => {
                // For descriptors, add all UTXOs from that descriptor
                // This is simplified - we'd need to filter by descriptor
                selected_utxos.extend(utxos.clone());
            }
        }
    }

    // Apply max amount selection if specified
    if let Some(max_amt) = max_amount {
        // Sort by amount descending
        selected_utxos.sort_by(|a, b| b.amount.cmp(&a.amount));

        let mut total = 0u64;
        let mut final_selection = Vec::new();

        for utxo in selected_utxos {
            if total >= max_amt.to_sat() {
                break;
            }
            total += utxo.amount;
            final_selection.push(utxo);
        }

        selected_utxos = final_selection;
    }

    // Build transaction
    let mut tx_builder = wallet.build_tx();

    // Add all selected inputs
    for utxo in &selected_utxos {
        let outpoint = OutPoint {
            txid: Txid::from_str(&utxo.txid)?,
            vout: utxo.vout,
        };
        tx_builder.add_utxo(outpoint)?;
    }

    // Calculate total input value
    let total_input: u64 = selected_utxos.iter().map(|u| u.amount).sum();

    // Determine fee
    let fee = if let Some(sats) = fee_sats {
        sats
    } else if let Some(rate) = fee_rate {
        // Estimate fee based on transaction size
        // Rough estimate: 10 + 41*inputs + 32*outputs
        let estimated_vbytes = 10 + 41 * selected_utxos.len() + 32;
        (estimated_vbytes as f64 * rate) as u64
    } else {
        bail!("Must specify either fee_rate or fee_sats");
    };

    // Add single output (total - fee)
    let output_amount = total_input.saturating_sub(fee);
    if output_amount == 0 {
        bail!("Output amount would be zero after fees");
    }

    let dest_script = bitcoin::Address::from_str(destination)?
        .require_network(network)?
        .script_pubkey();
    tx_builder.add_recipient(dest_script, Amount::from_sat(output_amount));

    // Manually select UTXOs (disable coin selection)
    tx_builder.manually_selected_only();

    // Finish building
    let psbt = tx_builder.finish()?;

    // Serialize PSBT to base64
    let psbt_bytes = psbt.serialize();
    let psbt_base64 = base64::engine::general_purpose::STANDARD.encode(&psbt_bytes);

    Ok(BdkPsbtResponse {
        psbt: psbt_base64,
        fee_sats: fee,
        change_position: None, // No change in consolidation
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bdk_wallet_creation() -> Result<()> {
        // Test descriptor for a simple wallet
        let descriptor = "wpkh([c258d2e4/84h/1h/0h]tpubDDYkZojQFQjht8Tm4jsS3iuEmKjTiEGjG6KnuFNKKJb5A6ZUCUZKdvLdSDWofKi4ToRCwb9poe1XdqfUnP4jaJjCB2Zwv11ZLgSbnZSNecE/0/*)";

        let wallet = Wallet::create_single(descriptor)
            .network(Network::Testnet)
            .create_wallet_no_persist()?;

        // Verify wallet was created successfully
        assert_eq!(wallet.network(), Network::Testnet);

        Ok(())
    }
}
