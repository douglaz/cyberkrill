use anyhow::{Context, Result, bail, ensure};
use base64::Engine;
use bdk_wallet::{KeychainKind, Wallet};
use bitcoin::{Amount, FeeRate, Network, OutPoint, Psbt, ScriptBuf, Txid};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;
use tracing::{debug, warn};

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

    for desc in &descriptors {
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
                warn!("Failed to create wallet for descriptor '{desc}': {e}");
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
    use bdk_electrum::{BdkElectrumClient, electrum_client};

    let descriptors = expand_multipath_descriptor(descriptor);
    let mut all_utxos = Vec::new();

    // Create Electrum client once for all descriptors
    let client = BdkElectrumClient::new(
        electrum_client::Client::new(electrum_url).context("Failed to create Electrum client")?,
    );

    for desc in &descriptors {
        // Create wallet with only external descriptor - fail fast on invalid descriptor
        let mut wallet = Wallet::create_single(desc.clone())
            .network(network)
            .create_wallet_no_persist()
            .with_context(|| format!("Failed to create wallet for descriptor '{desc}'"))?;

        // Create a full scan request
        let request = wallet
            .start_full_scan()
            .inspect({
                move |keychain, spk_i, _| {
                    // Progress output to stderr to keep stdout clean for JSON
                    debug!("Scanning {:?} at index {}", keychain, spk_i);
                }
            })
            .build();

        // Perform the scan - fail fast on scan errors
        let update = client
            .full_scan(request, stop_gap as usize, 10, false)
            .with_context(|| format!("Failed to scan with Electrum for descriptor '{desc}'"))?;

        // Apply the update to the wallet - fail fast on update errors
        wallet
            .apply_update(update)
            .with_context(|| format!("Failed to apply update for descriptor '{desc}'"))?;

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
                    warn!("Failed to derive address from script: {e}");
                }
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
    use bdk_esplora::{EsploraExt, esplora_client};

    let descriptors = expand_multipath_descriptor(descriptor);
    let mut all_utxos = Vec::new();

    // Create Esplora client once for all descriptors
    let client = esplora_client::Builder::new(esplora_url).build_blocking();

    for desc in &descriptors {
        // Create wallet with only external descriptor - fail fast on invalid descriptor
        let mut wallet = Wallet::create_single(desc.clone())
            .network(network)
            .create_wallet_no_persist()
            .with_context(|| format!("Failed to create wallet for descriptor '{desc}'"))?;

        // Create a full scan request
        let request = wallet
            .start_full_scan()
            .inspect({
                move |keychain, spk_i, _| {
                    // Progress output to stderr to keep stdout clean for JSON
                    debug!("Scanning {:?} at index {}", keychain, spk_i);
                }
            })
            .build();

        // Perform the scan with Esplora - fail fast on scan errors
        let update = client
            .full_scan(request, stop_gap as usize, 10)
            .with_context(|| format!("Failed to scan with Esplora for descriptor '{desc}'"))?;

        // Apply the update to the wallet - fail fast on update errors
        wallet
            .apply_update(update)
            .with_context(|| format!("Failed to apply update for descriptor '{desc}'"))?;

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
                    warn!("Failed to derive address from script: {e}");
                }
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
            ensure!(
                parts.len() == 2,
                "Invalid input format: '{s}'. Expected 'txid:vout' or a descriptor"
            );
            let txid = Txid::from_str(parts[0]).context("Invalid transaction ID")?;
            let vout: u32 = parts[1].parse().context("Invalid output index")?;
            Ok(InputSpec::Utxo { txid, vout })
        }
    }
}

/// Converts a fee rate in sat/vB to a BDK [`FeeRate`] without dropping the fraction.
///
/// `FeeRate` counts sat per 1000 weight units (1 sat/vB = 250 sat/kwu). The rate is rounded up,
/// so the result is never below the requested one. A rate that would come out as zero is an
/// error rather than a zero-fee transaction, and so is one too large for `FeeRate`.
fn fee_rate_from_sat_per_vb(rate: f64) -> Result<FeeRate> {
    let sat_per_kwu = (rate * 250.0).ceil();
    ensure!(
        sat_per_kwu > 0.0,
        "Fee rate must be above zero, got {rate} sat/vB"
    );
    // `as u64` would saturate infinity and anything past u64::MAX sat/kwu to u64::MAX
    ensure!(
        sat_per_kwu < u64::MAX as f64,
        "Fee rate is too large, got {rate} sat/vB"
    );
    Ok(FeeRate::from_sat_per_kwu(sat_per_kwu as u64))
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
        use bdk_electrum::{BdkElectrumClient, electrum_client};

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
        use bdk_esplora::{EsploraExt, esplora_client};

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
                    .ok_or_else(|| anyhow::anyhow!("UTXO {txid}:{vout} not found in wallet"))?;
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
        use bdk_electrum::{BdkElectrumClient, electrum_client};

        let client = BdkElectrumClient::new(
            electrum_client::Client::new(url).context("Failed to create Electrum client")?,
        );

        let request = wallet.start_full_scan().build();
        let update = client.full_scan(request, 200, 10, false)?;
        wallet.apply_update(update)?;
    } else if backend.starts_with("esplora://") {
        let url = backend.strip_prefix("esplora://").unwrap();
        use bdk_esplora::{EsploraExt, esplora_client};

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
        use bdk_electrum::{BdkElectrumClient, electrum_client};

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
        use bdk_esplora::{EsploraExt, esplora_client};

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
                    .ok_or_else(|| anyhow::anyhow!("UTXO {txid}:{vout} not found"))?;
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

    let outpoints = selected_utxos
        .iter()
        .map(|utxo| {
            Ok(OutPoint {
                txid: Txid::from_str(&utxo.txid)?,
                vout: utxo.vout,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let dest_script = bitcoin::Address::from_str(destination)?
        .require_network(network)?
        .script_pubkey();

    let psbt = build_move_psbt(&mut wallet, &outpoints, dest_script, fee_rate, fee_sats)?;
    let fee = psbt.fee()?;

    // Serialize PSBT to base64
    let psbt_bytes = psbt.serialize();
    let psbt_base64 = base64::engine::general_purpose::STANDARD.encode(&psbt_bytes);

    Ok(BdkPsbtResponse {
        psbt: psbt_base64,
        fee_sats: fee.to_sat(),
        change_position: None, // No change in consolidation
    })
}

/// Builds a PSBT that spends exactly `outpoints` to a single `destination` output.
///
/// BDK sets the output to the inputs minus the fee, paying `fee_sats` if given or else
/// `fee_rate` (sat/vB) on its own weight estimate, so no change output is added.
fn build_move_psbt(
    wallet: &mut Wallet,
    outpoints: &[OutPoint],
    destination: ScriptBuf,
    fee_rate: Option<f64>,
    fee_sats: Option<u64>,
) -> Result<Psbt> {
    let mut tx_builder = wallet.build_tx();
    tx_builder
        .add_utxos(outpoints)?
        .manually_selected_only()
        .drain_to(destination);
    match (fee_sats, fee_rate) {
        (Some(sats), _) => {
            tx_builder.fee_absolute(Amount::from_sat(sats));
        }
        (None, Some(rate)) => {
            tx_builder.fee_rate(fee_rate_from_sat_per_vb(rate)?);
        }
        (None, None) => bail!("Must specify either fee_rate or fee_sats"),
    }
    Ok(tx_builder.finish()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bitcoin_rpc::AmountInput;
    use bitcoin::WPubkeyHash;
    use bitcoin::hashes::Hash;

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

    /// Watch-only regtest wallet (wpkh of the generator point G) holding made-up unconfirmed
    /// UTXOs of the given amounts.
    fn funded_wallet(amounts: &[u64]) -> Result<(Wallet, Vec<OutPoint>)> {
        let mut wallet = Wallet::create_single(
            "wpkh(0279BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798)",
        )
        .network(Network::Regtest)
        .create_wallet_no_persist()?;
        let script_pubkey = wallet
            .peek_address(KeychainKind::External, 0)
            .script_pubkey();
        let mut outpoints = Vec::new();
        for (seed, sats) in (1u8..).zip(amounts) {
            let tx = bitcoin::Transaction {
                version: bitcoin::transaction::Version::TWO,
                lock_time: bitcoin::absolute::LockTime::ZERO,
                input: vec![bitcoin::TxIn {
                    previous_output: OutPoint::new(Txid::from_byte_array([seed; 32]), 0),
                    ..Default::default()
                }],
                output: vec![bitcoin::TxOut {
                    value: Amount::from_sat(*sats),
                    script_pubkey: script_pubkey.clone(),
                }],
            };
            outpoints.push(OutPoint::new(tx.compute_txid(), 0));
            wallet.apply_unconfirmed_txs([(tx, 0)]);
        }
        Ok((wallet, outpoints))
    }

    fn destination() -> ScriptBuf {
        ScriptBuf::new_p2wpkh(&WPubkeyHash::from_byte_array([0x11; 20]))
    }

    #[test]
    fn test_move_psbt_pays_fee_rate_without_change() -> Result<()> {
        for (inputs, input, fee) in [
            (1, "20sats", 2190),
            (3, "20sats", 4910),
            (1, "1sat", 110),
            (1, "0.5sats", 56),
            (1, "0.1sats", 13),
        ] {
            let (mut wallet, outpoints) = funded_wallet(&vec![100_000; inputs])?;
            let rate = AmountInput::from_str(input)?.as_fractional_sats();
            let psbt = build_move_psbt(&mut wallet, &outpoints, destination(), Some(rate), None)?;
            let outputs = &psbt.unsigned_tx.output;
            assert_eq!(
                outputs.len(),
                1,
                "{inputs} x {input}: one output, no change"
            );
            assert_eq!(outputs[0].script_pubkey, destination());
            assert_eq!(psbt.fee()?.to_sat(), fee, "{inputs} x {input}");
            assert_eq!(outputs[0].value.to_sat() + fee, 100_000 * inputs as u64);
        }

        Ok(())
    }

    #[test]
    fn test_move_psbt_pays_absolute_fee_without_change() -> Result<()> {
        let (mut wallet, outpoints) = funded_wallet(&[100_000])?;
        let psbt = build_move_psbt(&mut wallet, &outpoints, destination(), None, Some(1660))?;
        let outputs = &psbt.unsigned_tx.output;
        assert_eq!(outputs.len(), 1, "one output, no change");
        assert_eq!(outputs[0].value.to_sat(), 98_340);
        assert_eq!(psbt.fee()?.to_sat(), 1660);

        Ok(())
    }

    #[test]
    fn test_move_psbt_rejects_too_large_fee_rate() -> Result<()> {
        for rate in [f64::INFINITY, 1e17] {
            let (mut wallet, outpoints) = funded_wallet(&[100_000])?;
            let error = build_move_psbt(&mut wallet, &outpoints, destination(), Some(rate), None)
                .expect_err("a rate past FeeRate's range");
            assert!(error.to_string().contains("too large"), "{rate}: {error}");
        }

        Ok(())
    }
}
