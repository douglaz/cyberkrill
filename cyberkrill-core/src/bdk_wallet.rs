use anyhow::{Context, Result};
use bdk_wallet::{KeychainKind, Wallet};
use bitcoin::Network;
use serde::{Deserialize, Serialize};
use std::path::Path;

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
                        Err(_) => format!("script:{}", utxo.script_pub_key), // Fallback to script hex
                    }
                }
                Err(_) => format!("script:{}", utxo.script_pub_key), // Fallback to script hex
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
