use anyhow::{Context, Result, anyhow, bail};
use bitcoin::Network;
use bitcoin::bip32::{ChildNumber, DerivationPath, Xpub};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tracing::warn;
use trezor_client::client::common::handle_interaction;
use trezor_client::protos;
use trezor_client::{InputScriptType, Trezor as TrezorClient};

use crate::hardware_wallet::{AddressInfo, DeviceInfo, SignedPsbt};
use crate::slip132::parse_slip132_xpub;

/// Trezor hardware wallet implementation
pub struct TrezorWallet {
    client: TrezorClient,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrezorAddressOutput {
    pub address: String,
    pub derivation_path: String,
    pub xpub: String,
    pub network: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrezorSignOutput {
    pub psbt_base64: String,
    pub psbt_hex: String,
    pub is_complete: bool,
}

impl TrezorWallet {
    /// Connect to the first available Trezor device
    pub async fn connect() -> Result<Self> {
        // Find and connect to the first available Trezor
        let client = trezor_client::unique(false)
            .context("Failed to find Trezor device. Make sure your Trezor is connected via USB.")?;

        Ok(Self { client })
    }

    /// Initialize the device and get basic information
    pub fn init_device(&mut self) -> Result<()> {
        self.client
            .init_device(None)
            .context("Failed to initialize Trezor device")?;
        Ok(())
    }

    /// Get device information
    pub fn get_device_info(&mut self) -> Result<DeviceInfo> {
        // Initialize device if not already done
        let _ = self.init_device();

        let features = self
            .client
            .features()
            .ok_or_else(|| anyhow!("Failed to get device features"))?;

        Ok(DeviceInfo {
            device_type: "Trezor".to_string(),
            version: format!(
                "{}.{}.{}",
                features.major_version(),
                features.minor_version(),
                features.patch_version()
            ),
            initialized: features.initialized(),
            fingerprint: None, // Trezor doesn't expose master fingerprint directly
        })
    }

    /// Get a Bitcoin address at the given derivation path
    pub fn get_address(&mut self, path: &str, network: Network) -> Result<AddressInfo> {
        // Parse the derivation path
        let derivation_path = DerivationPath::from_str(path)
            .with_context(|| format!("Invalid derivation path: {path}"))?;

        // Determine script type based on path (BIP84 for native segwit by default)
        let script_type = determine_script_type(&derivation_path);

        // Get address from Trezor with user interaction handling
        let address = handle_interaction(
            self.client
                .get_address(&derivation_path, script_type, network, true)
                .context("Failed to get address from Trezor")?,
        )
        .context("User cancelled or interaction failed")?;

        // Try to get the xpub using our improved method
        let (xpub_str, pubkey_hex) = match self.get_xpub(path, network) {
            Ok(xpub) => (
                Some(xpub.to_string()),
                hex::encode(xpub.public_key.serialize()),
            ),
            Err(e) => {
                // Log the error for debugging but don't fail
                warn!("Could not extract xpub: {e}");
                (None, String::new())
            }
        };

        Ok(AddressInfo {
            address: address.to_string(),
            derivation_path: path.to_string(),
            pubkey: pubkey_hex,
            xpub: xpub_str,
        })
    }

    /// Get the raw public key response from Trezor (bypasses xpub parsing)
    fn get_public_key_raw(
        &mut self,
        path: &DerivationPath,
        script_type: InputScriptType,
        network: Network,
    ) -> Result<protos::PublicKey> {
        use trezor_client::utils;

        let mut req = protos::GetPublicKey::new();
        req.address_n = utils::convert_path(path);
        req.set_show_display(false);
        req.set_coin_name(utils::coin_name(network)?);
        req.set_script_type(script_type);

        // Call with a custom handler that just returns the raw message
        let response = self
            .client
            .call(req, Box::new(|_, m: protos::PublicKey| Ok(m)))?;

        handle_interaction(response).context("Failed to get public key from Trezor")
    }

    /// Build an Xpub from HDNodeType components
    fn build_xpub_from_node(&self, node: &protos::HDNodeType, network: Network) -> Result<Xpub> {
        use bitcoin::bip32::{ChainCode, Fingerprint};
        use bitcoin::secp256k1;

        // Extract components from HDNodeType
        let depth = node.depth() as u8;

        // Convert fingerprint (4 bytes)
        let fingerprint_bytes = node.fingerprint().to_be_bytes();
        let parent_fingerprint = Fingerprint::from(fingerprint_bytes);

        let child_number = ChildNumber::from(node.child_num());

        // Get chain code (32 bytes)
        let chain_code_bytes = node.chain_code();
        if chain_code_bytes.len() != 32 {
            bail!(
                "Invalid chain code length: {len}",
                len = chain_code_bytes.len()
            );
        }
        let mut chain_code_array = [0u8; 32];
        chain_code_array.copy_from_slice(chain_code_bytes);
        let chain_code = ChainCode::from(chain_code_array);

        // Get public key (33 bytes compressed)
        let pubkey_bytes = node.public_key();
        if pubkey_bytes.len() != 33 {
            bail!("Invalid public key length: {len}", len = pubkey_bytes.len());
        }
        let public_key = secp256k1::PublicKey::from_slice(pubkey_bytes)?;

        // Build the Xpub
        Ok(Xpub {
            network: network.into(),
            depth,
            parent_fingerprint,
            child_number,
            chain_code,
            public_key,
        })
    }

    /// Get extended public key at the given derivation path
    pub fn get_xpub(&mut self, path: &str, network: Network) -> Result<Xpub> {
        let derivation_path = DerivationPath::from_str(path)
            .with_context(|| format!("Invalid derivation path: {path}"))?;

        let script_type = determine_script_type(&derivation_path);

        // Get the raw public key response
        let pubkey_msg = self.get_public_key_raw(&derivation_path, script_type, network)?;

        // First try to parse the xpub string (handles BIP44 and SLIP-0132 formats)
        if pubkey_msg.has_xpub() {
            let xpub_str = pubkey_msg.xpub();
            if !xpub_str.is_empty() {
                // Try to parse it, handling SLIP-0132 formats
                if let Ok(xpub) = parse_slip132_xpub(xpub_str) {
                    return Ok(xpub);
                }
            }
        }

        // If that didn't work, build from the HDNodeType
        // The node field is always present in the response
        self.build_xpub_from_node(&pubkey_msg.node, network)
    }

    /// Sign a PSBT (Partially Signed Bitcoin Transaction)
    pub fn sign_psbt(&mut self, psbt_bytes: &[u8], network: Network) -> Result<SignedPsbt> {
        use base64::Engine;
        use bitcoin::psbt::Psbt;

        // Parse PSBT from bytes
        let mut psbt = Psbt::deserialize(psbt_bytes).context("Failed to deserialize PSBT")?;

        // Start the signing process
        let progress = handle_interaction(
            self.client
                .sign_tx(&psbt, network)
                .context("Failed to start transaction signing")?,
        )
        .context("User cancelled or signing failed")?;

        // Collect signatures and signed transaction parts
        let mut raw_tx = Vec::new();
        let is_complete = Self::tx_progress(&mut psbt, progress, &mut raw_tx, network)?;

        // Serialize the PSBT (potentially updated with signatures)
        let signed_psbt_bytes = psbt.serialize();
        let psbt_base64 = base64::engine::general_purpose::STANDARD.encode(&signed_psbt_bytes);

        Ok(SignedPsbt {
            psbt: signed_psbt_bytes,
            psbt_base64,
            is_complete,
        })
    }

    /// Helper function to handle the interactive signing process
    fn tx_progress(
        psbt: &mut bitcoin::psbt::Psbt,
        progress: trezor_client::SignTxProgress,
        raw_tx: &mut Vec<u8>,
        network: Network,
    ) -> Result<bool> {
        use std::io::Write;

        // Collect any serialized transaction parts
        if let Some(part) = progress.get_serialized_tx_part() {
            raw_tx.write_all(part)?;
        }

        // Continue the signing process if not finished
        if !progress.finished() {
            let next_progress = handle_interaction(
                progress
                    .ack_psbt(psbt, network)
                    .context("Failed to acknowledge PSBT to Trezor")?,
            )
            .context("User cancelled or interaction failed")?;
            Self::tx_progress(psbt, next_progress, raw_tx, network)
        } else {
            // Return whether we have a complete signed transaction
            Ok(!raw_tx.is_empty())
        }
    }

    /// Ping the device to check if it's connected
    pub fn ping(&mut self) -> Result<bool> {
        // Try to get features as a ping test
        Ok(self.client.features().is_some())
    }
}

/// Determine the appropriate script type based on the derivation path
fn determine_script_type(path: &DerivationPath) -> InputScriptType {
    use bitcoin::bip32::ChildNumber;

    // Check the purpose field (first hardened derivation)
    if let Some(purpose) = path.into_iter().next() {
        match purpose {
            ChildNumber::Hardened { index: 49 } => InputScriptType::SPENDP2SHWITNESS, // 49' - P2WPKH-nested-in-P2SH
            ChildNumber::Hardened { index: 84 } => InputScriptType::SPENDWITNESS, // 84' - P2WPKH
            ChildNumber::Hardened { index: 86 } => InputScriptType::SPENDTAPROOT, // 86' - P2TR
            ChildNumber::Hardened { index: 44 } => InputScriptType::SPENDADDRESS, // 44' - P2PKH
            _ => InputScriptType::SPENDADDRESS, // Default to P2PKH
        }
    } else {
        InputScriptType::SPENDADDRESS // Default to P2PKH
    }
}

/// Generate a Bitcoin address from Trezor
pub async fn generate_trezor_address(path: &str, network: Network) -> Result<TrezorAddressOutput> {
    let mut wallet = TrezorWallet::connect().await?;
    wallet.init_device()?;

    let address_info = wallet.get_address(path, network)?;

    Ok(TrezorAddressOutput {
        address: address_info.address,
        derivation_path: address_info.derivation_path,
        xpub: address_info.xpub.unwrap_or_default(),
        network: network.to_string(),
    })
}

/// Sign a PSBT with Trezor
pub async fn sign_psbt_with_trezor(psbt_data: &[u8], network: Network) -> Result<TrezorSignOutput> {
    let mut wallet = TrezorWallet::connect().await?;
    wallet.init_device()?;

    let signed = wallet.sign_psbt(psbt_data, network)?;

    Ok(TrezorSignOutput {
        psbt_base64: signed.psbt_base64,
        psbt_hex: hex::encode(&signed.psbt),
        is_complete: signed.is_complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trezor_address_output_serialization() -> Result<()> {
        let output = TrezorAddressOutput {
            address: "bc1qexample".to_string(),
            derivation_path: "m/84'/0'/0'/0/0".to_string(),
            xpub: "xpub...".to_string(),
            network: "bitcoin".to_string(),
        };

        let json = serde_json::to_string_pretty(&output)?;
        assert!(json.contains("\"address\": \"bc1qexample\""));
        assert!(json.contains("\"derivation_path\": \"m/84'/0'/0'/0/0\""));

        Ok(())
    }

    #[test]
    fn test_determine_script_type() -> Result<()> {
        use bitcoin::bip32::DerivationPath;
        use std::str::FromStr;

        // Test BIP84 (native segwit)
        let path = DerivationPath::from_str("m/84'/0'/0'/0/0")?;
        assert_eq!(determine_script_type(&path), InputScriptType::SPENDWITNESS);

        // Test BIP49 (nested segwit)
        let path = DerivationPath::from_str("m/49'/0'/0'/0/0")?;
        assert_eq!(
            determine_script_type(&path),
            InputScriptType::SPENDP2SHWITNESS
        );

        // Test BIP44 (legacy)
        let path = DerivationPath::from_str("m/44'/0'/0'/0/0")?;
        assert_eq!(determine_script_type(&path), InputScriptType::SPENDADDRESS);

        Ok(())
    }
}
