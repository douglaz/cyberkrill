use anyhow::{Context, Result, anyhow, ensure};
use bitcoin::bip32::Xpub;
use coldcard::{
    Api, Coldcard as ColdcardDevice, SignMode,
    protocol::{AddressFormat, DerivationPath},
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::hardware_wallet::{AddressInfo, DeviceInfo, SignedPsbt};

/// Convert our u32 derivation path to Coldcard's DerivationPath type
fn convert_to_coldcard_path(path: &[u32]) -> Result<DerivationPath> {
    // Convert to BIP32 string format
    let mut path_str = "m".to_string();
    for &component in path {
        path_str.push('/');
        if component >= 0x80000000 {
            // Hardened path
            path_str.push_str(&(component - 0x80000000).to_string());
            path_str.push('h'); // Coldcard uses 'h' for hardened
        } else {
            path_str.push_str(&component.to_string());
        }
    }

    // Create Coldcard DerivationPath from string
    DerivationPath::new(&path_str)
        .map_err(|e| anyhow!("Invalid derivation path for Coldcard: {e:?}"))
}

/// Coldcard hardware wallet implementation
/// Note: This is not thread-safe due to HID device limitations
pub struct ColdcardWallet {
    device: ColdcardDevice,
    master_fingerprint: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColdcardAddressOutput {
    pub address: String,
    pub derivation_path: String,
    pub xpub: String,
    pub device_fingerprint: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ColdcardSignOutput {
    pub psbt_base64: String,
    pub psbt_hex: String,
    pub is_complete: bool,
}

impl ColdcardWallet {
    /// Connect to the first available Coldcard device
    pub async fn connect() -> Result<Self> {
        let mut api = Api::new().context("Failed to initialize Coldcard API")?;

        let serials = api.detect().context(
            "Failed to detect Coldcard devices. Make sure your Coldcard is connected via USB.",
        )?;

        ensure!(
            !serials.is_empty(),
            "No Coldcard devices found. Please connect your Coldcard via USB."
        );

        // Connect to the first detected device
        let (device, xpub_info) = api
            .open(&serials[0], None)
            .context("Failed to open Coldcard device. Make sure it's unlocked.")?;

        let master_fingerprint = xpub_info.map(|info| {
            // Convert [u8; 4] to hex string
            hex::encode(info.fingerprint)
        });

        Ok(Self {
            device,
            master_fingerprint,
        })
    }

    /// Connect to a specific Coldcard by serial number
    pub async fn connect_serial(serial: &str) -> Result<Self> {
        let api = Api::new().context("Failed to initialize Coldcard API")?;

        let (device, xpub_info) = api
            .open(serial, None)
            .with_context(|| format!("Failed to open Coldcard with serial: {serial}"))?;

        let master_fingerprint = xpub_info.map(|info| {
            // Convert [u8; 4] to hex string
            hex::encode(info.fingerprint)
        });

        Ok(Self {
            device,
            master_fingerprint,
        })
    }

    /// List all connected Coldcard devices
    pub async fn list_devices() -> Result<Vec<String>> {
        let mut api = Api::new().context("Failed to initialize Coldcard API")?;

        let serials = api.detect().context("Failed to detect Coldcard devices")?;

        // SerialNumber doesn't implement Display, use Debug formatting
        Ok(serials
            .into_iter()
            .map(|serial| {
                // Coldcard serial numbers are hex strings, Debug format works
                format!("{serial:?}")
            })
            .collect())
    }
}

impl ColdcardWallet {
    pub fn get_device_info(&mut self) -> Result<DeviceInfo> {
        let version = self
            .device
            .version()
            .context("Failed to get Coldcard version")?;

        Ok(DeviceInfo {
            device_type: "Coldcard".to_string(),
            version,
            initialized: true, // Coldcard is always initialized if we can connect
            fingerprint: self.master_fingerprint.clone(),
        })
    }

    pub fn get_address(&mut self, path: &str) -> Result<AddressInfo> {
        // Parse the derivation path
        let derivation_path = crate::hardware_wallet::parse_derivation_path(path)?;

        // Convert to Coldcard's DerivationPath type
        let coldcard_path = convert_to_coldcard_path(&derivation_path)?;

        // Always use P2WPKH (native segwit) format
        let addr_fmt = AddressFormat::P2WPKH;

        // Get the address from Coldcard
        let address_str = self
            .device
            .address(coldcard_path, addr_fmt)
            .with_context(|| format!("Failed to get address at path: {path}"))?;

        // Get the xpub at this path
        let xpub = self.get_xpub(path)?;

        Ok(AddressInfo {
            address: address_str,
            derivation_path: path.to_string(),
            pubkey: hex::encode(xpub.public_key.serialize()),
            xpub: Some(xpub.to_string()),
        })
    }

    pub fn get_xpub(&mut self, path: &str) -> Result<Xpub> {
        let derivation_path = crate::hardware_wallet::parse_derivation_path(path)?;

        // Convert to Coldcard's DerivationPath type
        let coldcard_path = if derivation_path.is_empty() {
            None
        } else {
            Some(convert_to_coldcard_path(&derivation_path)?)
        };

        let xpub_str = self
            .device
            .xpub(coldcard_path)
            .with_context(|| format!("Failed to get xpub at path: {path}"))?;

        Xpub::from_str(&xpub_str).context("Failed to parse xpub from Coldcard")
    }

    pub fn sign_psbt(&mut self, psbt: &[u8]) -> Result<SignedPsbt> {
        use base64::Engine;

        // Sign the PSBT - note: sign_psbt doesn't return the signed PSBT directly
        self.device
            .sign_psbt(psbt, SignMode::Finalize)
            .context("Failed to sign PSBT with Coldcard")?;

        // Retrieve the signed transaction
        let signed_psbt_opt = self
            .device
            .get_signed_tx()
            .context("Failed to retrieve signed PSBT from Coldcard")?;

        // Check if we got a signed PSBT
        let signed_psbt =
            signed_psbt_opt.ok_or_else(|| anyhow!("No signed PSBT available from Coldcard"))?;

        // The signed PSBT is returned as bytes, encode to base64
        let psbt_base64 = base64::engine::general_purpose::STANDARD.encode(&signed_psbt);

        Ok(SignedPsbt {
            psbt: signed_psbt,
            psbt_base64,
            is_complete: false, // Coldcard doesn't tell us if it's complete
        })
    }

    pub fn ping(&mut self) -> Result<bool> {
        // Try to get version as a ping test
        self.device.version().map(|_| true).or(Ok(false))
    }
}

/// Generate a Bitcoin address from Coldcard
/// Note: The address network depends on the Coldcard's internal settings
pub async fn generate_coldcard_address(path: &str) -> Result<ColdcardAddressOutput> {
    let mut wallet = ColdcardWallet::connect().await?;
    let info = wallet.get_device_info()?;
    let address_info = wallet.get_address(path)?;

    Ok(ColdcardAddressOutput {
        address: address_info.address,
        derivation_path: address_info.derivation_path,
        xpub: address_info.xpub.unwrap_or_default(),
        device_fingerprint: info.fingerprint.unwrap_or_default(),
    })
}

/// Sign a PSBT with Coldcard
pub async fn sign_psbt_with_coldcard(psbt_data: &[u8]) -> Result<ColdcardSignOutput> {
    let mut wallet = ColdcardWallet::connect().await?;
    let signed = wallet.sign_psbt(psbt_data)?;

    Ok(ColdcardSignOutput {
        psbt_base64: signed.psbt_base64,
        psbt_hex: hex::encode(&signed.psbt),
        is_complete: signed.is_complete,
    })
}

/// Export a PSBT to Coldcard's SD card (air-gapped operation)
pub async fn export_psbt_to_coldcard(psbt_data: &[u8], filename: &str) -> Result<String> {
    use base64::Engine;

    let mut wallet = ColdcardWallet::connect().await?;

    // Coldcard expects base64-encoded PSBT
    let _psbt_base64 = base64::engine::general_purpose::STANDARD.encode(psbt_data);

    // Note: Coldcard doesn't have a direct method to save PSBT to SD card via API
    // The user needs to manually save it on the device
    // For now, we'll just prepare it for signing
    wallet
        .device
        .sign_psbt(psbt_data, SignMode::Finalize)
        .context("Failed to prepare PSBT for Coldcard")?;

    Ok(format!(
        "PSBT has been sent to Coldcard. Please save it to SD card as '{filename}' using the device menu."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coldcard_address_output_serialization() -> Result<()> {
        let output = ColdcardAddressOutput {
            address: "bc1qexample".to_string(),
            derivation_path: "m/84'/0'/0'/0/0".to_string(),
            xpub: "xpub...".to_string(),
            device_fingerprint: "12345678".to_string(),
        };

        let json = serde_json::to_string_pretty(&output)?;
        assert!(json.contains("\"address\": \"bc1qexample\""));
        assert!(json.contains("\"derivation_path\": \"m/84'/0'/0'/0/0\""));

        Ok(())
    }

    #[test]
    fn test_coldcard_sign_output_serialization() -> Result<()> {
        let output = ColdcardSignOutput {
            psbt_base64: "cHNidP8B...".to_string(),
            psbt_hex: "70736274ff01...".to_string(),
            is_complete: false,
        };

        let json = serde_json::to_string_pretty(&output)?;
        assert!(json.contains("\"psbt_base64\": \"cHNidP8B...\""));
        assert!(json.contains("\"is_complete\": false"));

        Ok(())
    }
}
