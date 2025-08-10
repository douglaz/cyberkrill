use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AddressInfo {
    pub address: String,
    pub derivation_path: String,
    pub pubkey: String,
    pub xpub: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignedPsbt {
    pub psbt: Vec<u8>,
    pub psbt_base64: String,
    pub is_complete: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device_type: String,
    pub version: String,
    pub initialized: bool,
    pub fingerprint: Option<String>,
}

/// Helper function to parse and validate BIP32 derivation paths
pub fn parse_derivation_path(path: &str) -> Result<Vec<u32>> {
    if !path.starts_with("m/") {
        anyhow::bail!("Derivation path must start with 'm/'");
    }

    let path_str = &path[2..]; // Remove "m/"
    let mut components = Vec::new();

    for component in path_str.split('/') {
        if component.is_empty() {
            continue;
        }

        let (number_str, hardened) = if let Some(stripped) = component.strip_suffix('\'') {
            (stripped, true)
        } else {
            (component, false)
        };

        let number: u32 = number_str
            .parse()
            .with_context(|| format!("Invalid derivation path component: {component}"))?;

        // Apply hardened derivation bit for proper BIP-32 path handling
        let value = if hardened {
            number + 0x80000000
        } else {
            number
        };

        components.push(value);
    }

    Ok(components)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_derivation_path() -> Result<()> {
        // Test standard BIP-84 path
        let path = "m/84'/0'/0'/0/0";
        let components = parse_derivation_path(path)?;
        assert_eq!(
            components,
            vec![
                84 + 0x80000000, // 84' (hardened)
                0x80000000,      // 0' (hardened)
                0x80000000,      // 0' (hardened)
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
    fn test_invalid_derivation_paths() -> Result<()> {
        let invalid_paths = vec![
            "84'/0'/0'/0/0", // Missing "m/"
            "m/invalid/0",   // Non-numeric component
            "",              // Empty path
        ];

        for path in invalid_paths {
            let result = parse_derivation_path(path);
            assert!(result.is_err(), "Path '{path}' should be invalid");
        }
        Ok(())
    }
}
