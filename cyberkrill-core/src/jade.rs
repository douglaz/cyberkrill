//! Jade hardware wallet integration

use anyhow::{Context, Result, bail};
use jade_bitcoin::{JadeClient, Network as JadeNetwork};
use serde::{Deserialize, Serialize};

/// Result of Jade address generation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JadeAddressResult {
    pub address: String,
    pub path: String,
    pub network: String,
}

/// Result of Jade xpub retrieval
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JadeXpubResult {
    pub xpub: String,
    pub path: String,
    pub network: String,
}

/// Result of Jade PSBT signing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JadeSignedPsbtResult {
    pub psbt: String,
    pub psbt_hex: String,
}

/// Parse network string to Jade network enum
fn parse_network(network: &str) -> Result<JadeNetwork> {
    match network.to_lowercase().as_str() {
        "bitcoin" | "mainnet" | "main" => Ok(JadeNetwork::Bitcoin),
        "testnet" | "test" => Ok(JadeNetwork::Testnet),
        "regtest" => Ok(JadeNetwork::Regtest),
        "signet" => Ok(JadeNetwork::Signet),
        _ => bail!(
            "Invalid network: {}. Use mainnet, testnet, regtest, or signet",
            network
        ),
    }
}

/// Generate a Bitcoin address from Jade
pub async fn generate_jade_address(path: &str, network: &str) -> Result<JadeAddressResult> {
    let jade_network = parse_network(network)?;

    let mut client = JadeClient::connect()
        .await
        .context("Failed to connect to Jade device")?;

    // Always try to unlock - the unlock method will check if already unlocked
    client.unlock(jade_network)
        .await
        .context("Failed to unlock Jade device. Please ensure you enter the PIN on the device when prompted.")?;

    // Give the device a moment after unlock
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let address = client
        .get_address(path, jade_network)
        .await
        .context("Failed to get address from Jade")?;

    Ok(JadeAddressResult {
        address,
        path: path.to_string(),
        network: network.to_string(),
    })
}

/// Get extended public key from Jade
pub async fn generate_jade_xpub(path: &str, network: &str) -> Result<JadeXpubResult> {
    let jade_network = parse_network(network)?;

    let mut client = JadeClient::connect()
        .await
        .context("Failed to connect to Jade device")?;

    // Always try to unlock - the unlock method will check if already unlocked
    client.unlock(jade_network)
        .await
        .context("Failed to unlock Jade device. Please ensure you enter the PIN on the device when prompted.")?;

    // Give the device a moment after unlock
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let xpub = client
        .get_xpub(path)
        .await
        .context("Failed to get xpub from Jade")?;

    Ok(JadeXpubResult {
        xpub,
        path: path.to_string(),
        network: network.to_string(),
    })
}

/// Sign a PSBT with Jade
pub async fn sign_psbt_with_jade(psbt_input: &str, network: &str) -> Result<JadeSignedPsbtResult> {
    let jade_network = parse_network(network)?;

    // Parse PSBT from hex or base64
    let psbt_bytes = if psbt_input.chars().all(|c| c.is_ascii_hexdigit()) {
        hex::decode(psbt_input).context("Failed to decode PSBT from hex")?
    } else {
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, psbt_input)
            .context("Failed to decode PSBT from base64")?
    };

    let mut client = JadeClient::connect()
        .await
        .context("Failed to connect to Jade device")?;

    // Always try to unlock - the unlock method will check if already unlocked
    client.unlock(jade_network)
        .await
        .context("Failed to unlock Jade device. Please ensure you enter the PIN on the device when prompted.")?;

    // Give the device a moment after unlock
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let signed_psbt = client
        .sign_psbt(&psbt_bytes, jade_network)
        .await
        .context("Failed to sign PSBT with Jade")?;

    let psbt_base64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &signed_psbt);

    Ok(JadeSignedPsbtResult {
        psbt: psbt_base64,
        psbt_hex: hex::encode(&signed_psbt),
    })
}
