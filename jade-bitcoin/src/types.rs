//! Common types used throughout jade-bitcoin

use serde::{Deserialize, Serialize};

/// Bitcoin network type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    #[serde(rename = "mainnet")]
    Bitcoin,
    #[serde(rename = "testnet")]
    Testnet,
    #[serde(rename = "regtest")]
    Regtest,
    #[serde(rename = "signet")]
    Signet,
}

impl Network {
    /// Convert to string for Jade protocol
    pub fn as_jade_str(&self) -> &str {
        match self {
            Network::Bitcoin => "mainnet",
            Network::Testnet => "testnet",
            Network::Regtest => "localtest",
            Network::Signet => "testnet", // Jade treats signet as testnet
        }
    }

    /// Convert to bitcoin crate network
    pub fn to_bitcoin_network(&self) -> bitcoin::Network {
        match self {
            Network::Bitcoin => bitcoin::Network::Bitcoin,
            Network::Testnet => bitcoin::Network::Testnet,
            Network::Regtest => bitcoin::Network::Regtest,
            Network::Signet => bitcoin::Network::Signet,
        }
    }
}

/// Device version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    #[serde(rename = "JADE_VERSION")]
    pub jade_version: String,
    #[serde(rename = "JADE_OTA_MAX_CHUNK")]
    pub jade_ota_max_chunk: Option<u32>,
    #[serde(rename = "JADE_CONFIG")]
    pub jade_config: String,
    #[serde(rename = "BOARD_TYPE")]
    pub board_type: String,
    #[serde(rename = "JADE_FEATURES")]
    pub jade_features: String,
    #[serde(rename = "IDF_VERSION")]
    pub idf_version: String,
    #[serde(rename = "CHIP_FEATURES")]
    pub chip_features: String,
    #[serde(rename = "EFUSEMAC")]
    pub efusemac: String,
    #[serde(rename = "BATTERY_STATUS")]
    pub battery_status: Option<u32>,
    #[serde(rename = "JADE_STATE")]
    pub jade_state: String,
    #[serde(rename = "JADE_NETWORKS")]
    pub jade_networks: String,
    #[serde(rename = "JADE_HAS_PIN")]
    pub jade_has_pin: bool,
}

/// Device identifiers for auto-detection
pub const JADE_USB_IDS: &[(u16, u16)] = &[
    (0x10c4, 0xea60), // CP210x UART Bridge
    (0x1a86, 0x55d4), // QinHeng CH9102F
    (0x0403, 0x6001), // FTDI FT232
];

/// Default serial port settings
pub const SERIAL_BAUD_RATE: u32 = 115200;
pub const SERIAL_TIMEOUT_MS: u64 = 120000; // 120 seconds for PIN server auth
