use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

// Satscard imports - correct API usage
use bitcoin::{key::CompressedPublicKey, network::Network, Address};
use rust_cktap::commands::Read;
use rust_cktap::{pcsc::find_first, CkTapCard}; // Required trait import for read() method

#[derive(Debug, Serialize, Deserialize)]
pub struct SatscardAddressOutput {
    pub slot: u8,
    pub address: String,
    pub pubkey: String,
    pub derivation_path: String,
    pub is_used: bool,
    pub card_info: SatscardInfo,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SatscardInfo {
    pub proto: usize,
    pub ver: String,
    pub birth: usize,
    pub current_slot: u8,
    pub max_slot: u8,
    pub card_address: Option<String>,
}

pub async fn generate_satscard_address(slot: Option<u8>) -> Result<SatscardAddressOutput> {
    // Connect to Satscard via NFC/PCSC - this automatically gets status
    let card = find_first()
        .await
        .with_context(|| "Failed to find Satscard. Make sure PCSC daemon is running, NFC reader is connected, and Satscard is placed on the reader")?;

    let mut satscard = match card {
        CkTapCard::SatsCard(satscard) => satscard,
        _ => {
            anyhow::bail!(
                "Found CkTap card but it's not a Satscard. Make sure you're using a Satscard."
            )
        }
    };

    // Get card status information from struct fields (not a status() method)
    let current_slot = satscard.slots.0;
    let max_slot = satscard.slots.1;

    // Determine which slot to use
    let target_slot = match slot {
        Some(requested_slot) => {
            // Validate requested slot exists
            if requested_slot > max_slot {
                anyhow::bail!(
                    "Invalid slot number: {requested_slot}. Satscard has slots 0-{max_slot}."
                );
            }
            requested_slot
        }
        None => {
            // Use current active slot
            current_slot
        }
    };

    // Read the slot's public key - Satscard read() doesn't require authentication
    let read_result = satscard
        .read(None)
        .await
        .with_context(|| format!("Failed to read slot {target_slot} from Satscard"))?;

    // Get the address from the public key (Satscard uses fixed m/0 derivation)
    let pubkey_bytes = read_result
        .pubkey(None)
        .with_context(|| "Failed to get public key from read response")?
        .serialize();
    let address = pubkey_to_address(&pubkey_bytes)?;

    // Check if this slot has been used (simplified check - in practice you'd check blockchain)
    let is_used = target_slot < current_slot; // Slots before current are typically used

    // Create card info structure
    let card_info = SatscardInfo {
        proto: satscard.proto,
        ver: satscard.ver.clone(),
        birth: satscard.birth,
        current_slot,
        max_slot,
        card_address: satscard.addr.clone(),
    };

    Ok(SatscardAddressOutput {
        slot: target_slot,
        address,
        pubkey: hex::encode(pubkey_bytes),
        derivation_path: "m/0".to_string(), // Satscard always uses m/0
        is_used,
        card_info,
    })
}

fn pubkey_to_address(pubkey: &[u8]) -> Result<String> {
    // Convert public key to Bitcoin address using proper Bitcoin libraries
    if pubkey.len() != 33 {
        anyhow::bail!(
            "Invalid public key length: expected 33 bytes, got {pubkey_len}",
            pubkey_len = pubkey.len()
        );
    }

    // Convert to compressed public key for address generation
    let compressed_pubkey = CompressedPublicKey::from_slice(pubkey)
        .with_context(|| "Failed to parse compressed public key")?;

    // Generate P2WPKH (native segwit) address for mainnet
    // This corresponds to BIP-84 (m/84'/0'/0'/0/x) derivation paths
    let address = Address::p2wpkh(&compressed_pubkey, Network::Bitcoin);

    Ok(address.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pubkey_to_address() -> anyhow::Result<()> {
        // Test case from known public key
        let expected_pubkey = "02856528bfb921cfb18c9b5427ecada29a2fc72e55671b8fe131d1691b722de986";
        let expected_address = "bc1qy80agvcq084qtsdg3wayr2uzxweqmsx7xed9s5";

        // Convert hex pubkey to bytes
        let pubkey_bytes = hex::decode(expected_pubkey)?;

        // Generate address using our function
        let generated_address = pubkey_to_address(&pubkey_bytes)?;

        assert_eq!(
            generated_address, expected_address,
            "Generated address doesn't match expected output"
        );

        Ok(())
    }

    #[test]
    fn test_invalid_pubkey_length() {
        let invalid_pubkey = vec![0u8; 32]; // Wrong length
        let result = pubkey_to_address(&invalid_pubkey);
        assert!(result.is_err(), "Should fail with invalid pubkey length");
    }
}
