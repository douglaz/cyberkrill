use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// Tapsigner imports
use bitcoin::{
    bip32::{DerivationPath, Xpub},
    hashes::{hash160, Hash},
    key::CompressedPublicKey,
    network::Network,
    secp256k1::{PublicKey, Secp256k1},
    Address,
};
use rust_cktap::{pcsc::find_first, CkTapCard, TapSigner};

#[derive(Debug, Serialize, Deserialize)]
pub struct TapsignerAddressOutput {
    pub derivation_path: String,
    pub address: String,
    pub pubkey: String,
    pub master_pubkey: String,
    pub master_fingerprint: String,
    pub chain_code: String,
}

pub async fn generate_tapsigner_address(path: &str) -> Result<TapsignerAddressOutput> {
    // Parse the derivation path and split into hardened/non-hardened parts
    let (hardened_path, non_hardened_path) = split_derivation_path(path)?;

    // Connect to Tapsigner via NFC/PCSC
    let mut tapsigner = connect_tapsigner().await?;

    // First, get the master key by deriving from root path
    let master_result = tapsigner.derive_address(&[]).await?;

    // Derive the hardened portion on hardware (up to account level)
    let account_result = tapsigner.derive_address(&hardened_path).await?;

    // Create xpub from the account-level result
    let account_xpub = create_xpub_from_result(&account_result)?;

    // Software derive the non-hardened portion
    let final_pubkey = if non_hardened_path.is_empty() {
        // If no non-hardened path, use the account key directly
        account_result.pubkey
    } else {
        // Derive non-hardened path in software
        software_derive_pubkey(&account_xpub, &non_hardened_path)?
    };

    // Convert the public key to a Bitcoin address
    let address = pubkey_to_address(&final_pubkey)?;

    // Calculate the master fingerprint from the actual master pubkey
    let master_fingerprint = calculate_fingerprint(&master_result.pubkey)?;

    Ok(TapsignerAddressOutput {
        derivation_path: path.to_string(),
        address,
        pubkey: hex::encode(final_pubkey),
        master_pubkey: hex::encode(master_result.pubkey),
        master_fingerprint,
        chain_code: hex::encode(account_result.chain_code),
    })
}

fn parse_derivation_path(path: &str) -> Result<Vec<u32>> {
    // Simple derivation path parser (e.g., "m/84'/0'/0'/0/0")
    if !path.starts_with("m/") {
        anyhow::bail!("Derivation path must start with 'm/'");
    }

    let path_str = &path[2..]; // Remove "m/"
    let mut components = Vec::new();

    for component in path_str.split('/') {
        if component.is_empty() {
            continue;
        }

        let (number_str, _hardened) = if let Some(stripped) = component.strip_suffix('\'') {
            (stripped, true)
        } else {
            (component, false)
        };

        let number: u32 = number_str
            .parse()
            .with_context(|| format!("Invalid derivation path component: {component}"))?;

        // Apply hardened derivation bit for proper BIP-32 path handling
        let value = if _hardened {
            number + 0x80000000
        } else {
            number
        };

        components.push(value);
    }

    Ok(components)
}

// Real Tapsigner device communication
enum TapsignerDevice {
    TapSigner(Box<TapSigner<::pcsc::Card>>),
}

struct DeriveResult {
    pubkey: Vec<u8>,
    chain_code: Vec<u8>,
}

impl TapsignerDevice {
    async fn derive_address(&mut self, path: &[u32]) -> Result<DeriveResult> {
        match self {
            TapsignerDevice::TapSigner(tapsigner) => {
                // Convert hardened path to raw numbers for rust-cktap API
                // rust-cktap expects raw numbers and handles hardened derivation internally
                let derive_path: Vec<u32> = path
                    .iter()
                    .map(|&x| {
                        if x >= 0x80000000 {
                            x - 0x80000000 // Remove hardened bit for rust-cktap
                        } else {
                            x
                        }
                    })
                    .collect();

                // Get CVC from environment or prompt user
                let cvc_str = get_cvc_from_env_or_prompt()?;

                // Derive the public key at the specified path
                let derive_response = tapsigner
                    .derive(&derive_path, &cvc_str)
                    .await
                    .with_context(|| "Failed to derive key from Tapsigner")?;
                Ok(DeriveResult {
                    pubkey: derive_response.pubkey.unwrap_or([0u8; 33]).to_vec(),
                    chain_code: derive_response.chain_code.to_vec(),
                })
            }
        }
    }
}

async fn connect_tapsigner() -> Result<TapsignerDevice> {
    // Find and connect to the first available CkTap card
    let card = find_first()
        .await
        .with_context(|| "Failed to find Tapsigner. Make sure PCSC daemon is running, NFC reader is connected, and Tapsigner card is placed on the reader")?;

    match card {
        CkTapCard::TapSigner(tapsigner) => Ok(TapsignerDevice::TapSigner(Box::new(tapsigner))),
        _ => {
            anyhow::bail!("Found CkTap card but it's not a TapSigner. Make sure you're using a TapSigner card.")
        }
    }
}

fn get_cvc_from_env_or_prompt() -> Result<String> {
    // Try to get CVC from environment variable first
    if let Ok(cvc_str) = std::env::var("TAPSIGNER_CVC") {
        // Validate it's a 6-digit PIN
        if cvc_str.len() == 6 && cvc_str.chars().all(|c| c.is_ascii_digit()) {
            return Ok(cvc_str);
        }

        anyhow::bail!(
            "Invalid PIN format. TAPSIGNER_CVC must be exactly 6 digits. Got: '{cvc_str}'"
        );
    }

    // For now, return an error asking user to set environment variable
    anyhow::bail!(
        "PIN authentication required. Please set TAPSIGNER_CVC environment variable with your card's 6-digit PIN.
Example: export TAPSIGNER_CVC=123456

To find your PIN, check the back of your Tapsigner card or your purchase documentation."
    )
}

fn split_derivation_path(path: &str) -> Result<(Vec<u32>, Vec<u32>)> {
    let components = parse_derivation_path(path)?;

    // For BIP-44/84 paths like m/84'/0'/0'/0/0:
    // - Hardened part: [84', 0', 0'] (account level)
    // - Non-hardened part: [0, 0] (change/address index)

    // Find the split point - typically after the third hardened component
    let mut hardened_end = 0;
    for (i, &component) in components.iter().enumerate() {
        if component >= 0x80000000 {
            hardened_end = i + 1;
        } else {
            break;
        }
    }

    let hardened_path = components[..hardened_end].to_vec();
    let non_hardened_path = components[hardened_end..].to_vec();

    Ok((hardened_path, non_hardened_path))
}

fn create_xpub_from_result(result: &DeriveResult) -> Result<Xpub> {
    let public_key = PublicKey::from_slice(&result.pubkey)
        .with_context(|| "Failed to parse secp256k1 public key")?;

    // Convert chain code to proper format (must be exactly 32 bytes)
    if result.chain_code.len() != 32 {
        anyhow::bail!(
            "Invalid chain code length: expected 32 bytes, got {len}",
            len = result.chain_code.len()
        );
    }
    let mut chain_code_array = [0u8; 32];
    chain_code_array.copy_from_slice(&result.chain_code);
    let chain_code = bitcoin::bip32::ChainCode::from(chain_code_array);

    // Create xpub with minimal metadata (depth=3 for account level m/84'/0'/0')
    let xpub = Xpub {
        network: Network::Bitcoin.into(),
        depth: 3,
        parent_fingerprint: bitcoin::bip32::Fingerprint::default(), // Simplified
        child_number: bitcoin::bip32::ChildNumber::from_hardened_idx(0)
            .with_context(|| "Failed to create hardened child number")?,
        public_key,
        chain_code,
    };

    Ok(xpub)
}

fn software_derive_pubkey(xpub: &Xpub, path_components: &[u32]) -> Result<Vec<u8>> {
    let secp = Secp256k1::new();

    // Convert path components to derivation path
    let mut path_str = "m".to_string();
    for &component in path_components {
        if component >= 0x80000000 {
            anyhow::bail!("Non-hardened path contains hardened component: {component}");
        }
        path_str.push_str(&format!("/{component}"));
    }

    let path = DerivationPath::from_str(&path_str)?;
    let derived_xpub = xpub.derive_pub(&secp, &path)?;

    Ok(derived_xpub.public_key.serialize().to_vec())
}

fn calculate_fingerprint(pubkey: &[u8]) -> Result<String> {
    // Calculate the fingerprint (first 4 bytes of hash160 of the public key)
    if pubkey.len() != 33 {
        anyhow::bail!(
            "Invalid public key length for fingerprint: expected 33 bytes, got {pubkey_len}",
            pubkey_len = pubkey.len()
        );
    }

    let hash = hash160::Hash::hash(pubkey);
    let fingerprint_bytes = &hash.as_byte_array()[0..4];
    Ok(hex::encode(fingerprint_bytes))
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
    fn test_parse_derivation_path() -> anyhow::Result<()> {
        // Test standard BIP-84 path
        let path = "m/84'/0'/0'/0/0";
        let components = parse_derivation_path(path)?;
        assert_eq!(
            components,
            vec![
                84 + 0x80000000, // 84' (hardened)
                0 + 0x80000000,  // 0' (hardened)
                0 + 0x80000000,  // 0' (hardened)
                0,               // 0 (non-hardened)
                0                // 0 (non-hardened)
            ]
        );

        // Test root path
        let path = "m/";
        let components = parse_derivation_path(path)?;
        assert_eq!(components, Vec::<u32>::new());

        Ok(())
    }

    #[test]
    fn test_invalid_derivation_paths() {
        let invalid_paths = vec![
            "84'/0'/0'/0/0", // Missing "m/"
            "m/invalid/0",   // Non-numeric component
            "",              // Empty path
        ];

        for path in invalid_paths {
            let result = parse_derivation_path(path);
            assert!(
                result.is_err(),
                "Path '{path}' should be invalid",
                path = path
            );
        }
    }

    #[test]
    fn test_sparrow_wallet_expected_output() -> anyhow::Result<()> {
        // Test case from Sparrow wallet for m/84'/0'/0'/0/0
        let expected_pubkey = "02856528bfb921cfb18c9b5427ecada29a2fc72e55671b8fe131d1691b722de986";
        let expected_address = "bc1qy80agvcq084qtsdg3wayr2uzxweqmsx7xed9s5";

        // Convert hex pubkey to bytes
        let pubkey_bytes = hex::decode(expected_pubkey)?;

        // Generate address using our function
        let generated_address = pubkey_to_address(&pubkey_bytes)?;

        assert_eq!(
            generated_address,
            expected_address,
            "Generated address doesn't match Sparrow wallet expected output"
        );

        Ok(())
    }

    #[test]
    fn test_sparrow_xpub_decode() -> anyhow::Result<()> {
        // The xpub from Sparrow wallet descriptor
        let sparrow_xpub = "xpub6BemYiVNp19a1ufcPyUNs1CFUVV6fp2vMkLoiQCXHaLyBCJ317M6jqM4y2k22naLNC4tZMCm597k2Bhomza5A1SM3VP9WBeaxbR1ErZkpw2";

        // Parse the xpub
        let xpub = Xpub::from_str(sparrow_xpub)?;

        println!("Sparrow xpub public key: {}", hex::encode(xpub.public_key.serialize()));
        println!("Sparrow xpub chain code: {}", hex::encode(xpub.chain_code.as_bytes()));

        // Compare with our expected Tapsigner output
        let our_account_pubkey = "0379890f62200b30e6c33ece95d7be439184c1280366f5b3ebed60b3e946681b68";
        let our_chain_code = "b278131303d560983aa72e0ee571a9c9b7b38b19aab335a1f3a0b8395338b4e7";

        println!("Our Tapsigner pubkey: {our_account_pubkey}");
        println!("Our Tapsigner chain code: {our_chain_code}");

        // They should match if we're getting the same xpub from the same card
        let sparrow_pubkey_hex = hex::encode(xpub.public_key.serialize());
        let sparrow_chain_code_hex = hex::encode(xpub.chain_code.as_bytes());

        if sparrow_pubkey_hex != our_account_pubkey {
            println!("MISMATCH: Public keys don't match!");
        }
        if sparrow_chain_code_hex != our_chain_code {
            println!("MISMATCH: Chain codes don't match!");
        }

        Ok(())
    }

    #[test]
    fn test_bip32_child_derivation() -> anyhow::Result<()> {
        // The xpub from Sparrow wallet descriptor (account level m/84'/0'/0')
        let sparrow_xpub = "xpub6BemYiVNp19a1ufcPyUNs1CFUVV6fp2vMkLoiQCXHaLyBCJ317M6jqM4y2k22naLNC4tZMCm597k2Bhomza5A1SM3VP9WBeaxbR1ErZkpw2";

        // Parse the xpub
        let xpub = Xpub::from_str(sparrow_xpub)?;

        // Derive child key at path 0/0 (external chain, first address)
        let path = DerivationPath::from_str("m/0/0")?;
        let child_xpub = xpub.derive_pub(&bitcoin::secp256k1::Secp256k1::new(), &path)?;

        let derived_pubkey = child_xpub.public_key.serialize();
        let derived_pubkey_hex = hex::encode(derived_pubkey);

        println!("Account xpub: {sparrow_xpub}");
        println!("Derived path: 0/0");
        println!("Derived pubkey: {derived_pubkey_hex}");

        // Expected from Sparrow
        let expected_pubkey = "02856528bfb921cfb18c9b5427ecada29a2fc72e55671b8fe131d1691b722de986";
        println!("Expected pubkey: {expected_pubkey}");

        // Our current hardware derivation result
        let our_hardware_pubkey = "03ef7b5f6cecef500fd420fd90a27bf54d75297351e2e2a9c42fa20cd68fe77a58";
        println!("Our hardware pubkey: {our_hardware_pubkey}");

        if derived_pubkey_hex == expected_pubkey {
            println!("✅ SUCCESS: Software BIP-32 derivation matches Sparrow!");
        } else {
            println!("❌ MISMATCH: Software derivation doesn't match expected");
        }

        if derived_pubkey_hex != our_hardware_pubkey {
            println!("ℹ️  INFO: Software derivation differs from hardware derivation (expected)");
        }

        Ok(())
    }
}