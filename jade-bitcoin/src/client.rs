//! High-level Jade client API

use crate::error::{Error, Result};
use crate::protocol::JadeProtocol;
use crate::serial::SerialConnection;
use crate::types::{Network, VersionInfo};
use bitcoin::bip32::DerivationPath;
use log::{debug, info};
use std::str::FromStr;

/// High-level client for Jade hardware wallet
pub struct JadeClient {
    protocol: JadeProtocol,
    current_network: Option<Network>,
}

impl JadeClient {
    /// Connect to Jade device on any available port
    pub async fn connect() -> Result<Self> {
        info!("Searching for Jade device...");
        let connection = SerialConnection::connect().await?;
        let protocol = JadeProtocol::new(connection);

        Ok(Self {
            protocol,
            current_network: None,
        })
    }

    /// Connect to Jade device on specific port
    pub async fn connect_path(path: &str) -> Result<Self> {
        info!("Connecting to Jade on {path}");
        let connection = SerialConnection::connect_path(path).await?;
        let protocol = JadeProtocol::new(connection);

        Ok(Self {
            protocol,
            current_network: None,
        })
    }

    /// List all available Jade devices
    pub fn list_devices() -> Vec<String> {
        SerialConnection::list_devices()
    }

    /// Get device version information
    pub async fn get_version_info(&mut self) -> Result<VersionInfo> {
        debug!("Getting version info");
        let result = self.protocol.get_version_info().await?;

        serde_json::from_value(result).map_err(|_| Error::InvalidResponse)
    }

    /// Unlock the device for a specific network
    pub async fn unlock(&mut self, network: Network) -> Result<()> {
        info!("Unlocking Jade for {network:?}");

        // Check if already unlocked for this network
        if self.current_network == Some(network) {
            // Try a simple operation to verify the connection is still valid
            match self.get_version_info().await {
                Ok(_) => {
                    info!("Jade already unlocked for {network:?}");
                    return Ok(());
                }
                Err(_) => {
                    // Connection might be stale, continue with unlock
                    self.current_network = None;
                }
            }
        }

        // First get version to ensure device is responsive
        let _version = self.get_version_info().await?;

        // Authenticate with the network
        self.protocol.auth_user(network).await?;
        self.current_network = Some(network);

        info!("Jade unlocked successfully");
        Ok(())
    }

    /// Check if device is already unlocked (without trying to unlock)
    pub async fn is_unlocked(&mut self) -> bool {
        info!("Checking if Jade is unlocked...");
        // Try to get version info with current auth state
        match self.protocol.get_version_info().await {
            Ok(info) => {
                debug!("Version info received: {info:?}");
                // Check if the info indicates an unlocked state
                if let Some(state) = info.get("JADE_STATE").and_then(|v| v.as_str()) {
                    let unlocked = state != "LOCKED";
                    info!("Jade state: {state} (unlocked: {unlocked})");

                    // If device is ready, set the current network to Bitcoin (mainnet) by default
                    // The device doesn't tell us which network it's on, so we assume mainnet
                    if unlocked && self.current_network.is_none() {
                        self.current_network = Some(Network::Bitcoin);
                        info!("Device is unlocked, assuming mainnet network");
                    }

                    unlocked
                } else {
                    info!("No JADE_STATE found in version info");
                    false
                }
            }
            Err(e) => {
                info!("Failed to get version info: {e:?}");
                false
            }
        }
    }

    /// Logout from the device
    pub async fn logout(&mut self) -> Result<()> {
        info!("Logging out from Jade");
        self.protocol.logout().await?;
        self.current_network = None;
        Ok(())
    }

    /// Get extended public key at derivation path
    pub async fn get_xpub(&mut self, path: &str) -> Result<String> {
        debug!("Getting xpub for path: {path}");

        // Check if device is unlocked
        if self.current_network.is_none() {
            return Err(Error::DeviceLocked);
        }

        let path_array = parse_derivation_path(path)?;
        let network = self.current_network.unwrap();
        self.protocol.get_xpub(&path_array, network).await
    }

    /// Get Bitcoin address at derivation path
    pub async fn get_address(&mut self, path: &str, network: Network) -> Result<String> {
        debug!("Getting address for path: {path} on {network:?}");

        // Check if we need to switch networks
        if let Some(current) = self.current_network {
            if current != network {
                return Err(Error::NetworkMismatch {
                    device: format!("{current:?}"),
                    requested: format!("{network:?}"),
                });
            }
        } else {
            return Err(Error::DeviceLocked);
        }

        let path_array = parse_derivation_path(path)?;

        // Determine address variant based on path
        let variant = determine_address_variant(&path_array);

        self.protocol
            .get_receive_address(network, &path_array, variant)
            .await
    }

    /// Sign a PSBT (Partially Signed Bitcoin Transaction)
    pub async fn sign_psbt(&mut self, psbt: &[u8], network: Network) -> Result<Vec<u8>> {
        debug!("Signing PSBT for {network:?}");

        // Check network
        if let Some(current) = self.current_network {
            if current != network {
                return Err(Error::NetworkMismatch {
                    device: format!("{current:?}"),
                    requested: format!("{network:?}"),
                });
            }
        } else {
            return Err(Error::DeviceLocked);
        }

        let result = self.protocol.sign_psbt(network, psbt).await?;

        // Extract the signed PSBT from response
        if let Some(psbt_str) = result.get("psbt").and_then(|v| v.as_str()) {
            // Decode base64 PSBT
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, psbt_str)
                .map_err(|_| Error::InvalidResponse)
        } else {
            Err(Error::InvalidResponse)
        }
    }

    /// Sign a message with a specific derivation path
    pub async fn sign_message(&mut self, message: &str, path: &str) -> Result<String> {
        debug!("Signing message with path: {path}");

        if self.current_network.is_none() {
            return Err(Error::DeviceLocked);
        }

        let path_array = parse_derivation_path(path)?;
        self.protocol
            .sign_message(&path_array, message, false)
            .await
    }
}

/// Parse BIP32 derivation path
fn parse_derivation_path(path: &str) -> Result<Vec<u32>> {
    // Parse using bitcoin crate's DerivationPath
    let derivation =
        DerivationPath::from_str(path).map_err(|_| Error::InvalidPath(path.to_string()))?;

    // Convert to u32 array for Jade
    let path_array: Vec<u32> = derivation
        .into_iter()
        .map(|child| {
            let index = u32::from(*child);
            // Jade expects hardened paths to have the high bit set
            if child.is_hardened() {
                index | 0x80000000
            } else {
                index
            }
        })
        .collect();

    Ok(path_array)
}

/// Determine address variant based on derivation path
fn determine_address_variant(path: &[u32]) -> Option<&'static str> {
    if path.is_empty() {
        return None;
    }

    // Check the first component (purpose) to determine address type
    // Remove hardening bit for comparison
    let purpose = path[0] & 0x7FFFFFFF;

    match purpose {
        44 => Some("pkh(k)"),      // Legacy P2PKH
        49 => Some("sh(wpkh(k))"), // Nested SegWit P2SH-P2WPKH
        84 => Some("wpkh(k)"),     // Native SegWit P2WPKH
        86 => Some("tr(k)"),       // Taproot P2TR
        _ => None,
    }
}
