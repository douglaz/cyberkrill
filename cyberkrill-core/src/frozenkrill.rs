use anyhow::{Context, Result};
use bitcoin::Network;
use serde::Deserialize;
use std::io::BufReader;
use std::path::Path;

use frozenkrill_core::wallet_export::GenericOutputExportJson;

// Re-define the structures with public fields to work around private field access
#[derive(Debug, Deserialize)]
struct SingleSigWalletData {
    #[allow(dead_code)]
    wallet: String,
    #[allow(dead_code)]
    version: u32,
    #[allow(dead_code)]
    sigtype: String,
    #[allow(dead_code)]
    master_fingerprint: String,
    #[allow(dead_code)]
    singlesig_xpub: String,
    #[allow(dead_code)]
    singlesig_derivation_path: String,
    #[allow(dead_code)]
    multisig_xpub: String,
    #[allow(dead_code)]
    multisig_derivation_path: String,
    singlesig_receiving_output_descriptor: String,
    singlesig_change_output_descriptor: String,
    #[allow(dead_code)]
    multisig_receiving_output_descriptor_key: String,
    #[allow(dead_code)]
    multisig_change_output_descriptor_key: String,
    #[allow(dead_code)]
    script_type: String,
    network: String,
    receiving_addresses: Vec<AddressInfo>,
    change_addresses: Vec<AddressInfo>,
}

#[derive(Debug, Deserialize)]
struct MultiSigWalletData {
    #[allow(dead_code)]
    wallet: String,
    #[allow(dead_code)]
    version: u32,
    sigtype: String,
    #[allow(dead_code)]
    script_type: String,
    network: String,
    receiving_output_descriptor: String,
    change_output_descriptor: String,
    receiving_addresses: Vec<AddressInfo>,
    change_addresses: Vec<AddressInfo>,
}

#[derive(Debug, Deserialize)]
struct AddressInfo {
    address: String,
    #[allow(dead_code)]
    #[serde(skip_serializing_if = "Option::is_none")]
    derivation_path: Option<String>,
    #[allow(dead_code)]
    #[serde(skip_serializing_if = "Option::is_none")]
    index: Option<u32>,
}

/// Represents a frozenkrill wallet export, either single-sig or multi-sig
#[derive(Debug)]
#[allow(private_interfaces)]
pub enum FrozenkrillWallet {
    #[doc(hidden)]
    SingleSig(SingleSigWalletData),
    #[doc(hidden)]
    MultiSig(MultiSigWalletData),
}

impl FrozenkrillWallet {
    /// Load a frozenkrill wallet from a JSON export file
    pub fn from_file(path: &Path) -> Result<Self> {
        // First pass: detect wallet type
        let file = std::fs::File::open(path).with_context(|| {
            format!("Failed to open wallet file: {path}", path = path.display())
        })?;
        let reader = BufReader::new(file);

        let generic = GenericOutputExportJson::deserialize(reader)?;
        let (_version, sigtype) = generic.version_sigtype()?;

        // Second pass: deserialize with our own structures
        let content = std::fs::read_to_string(path)?;

        match sigtype {
            Some(frozenkrill_core::wallet_description::SigType::Singlesig) => {
                let wallet: SingleSigWalletData = serde_json::from_str(&content)?;
                Ok(Self::SingleSig(wallet))
            }
            Some(frozenkrill_core::wallet_description::SigType::Multisig(_)) => {
                let wallet: MultiSigWalletData = serde_json::from_str(&content)?;
                Ok(Self::MultiSig(wallet))
            }
            _ => anyhow::bail!("Unknown or missing wallet signature type"),
        }
    }

    /// Get the receiving (external) descriptor
    pub fn receiving_descriptor(&self) -> &str {
        match self {
            Self::SingleSig(s) => &s.singlesig_receiving_output_descriptor,
            Self::MultiSig(m) => &m.receiving_output_descriptor,
        }
    }

    /// Get the change (internal) descriptor
    pub fn change_descriptor(&self) -> &str {
        match self {
            Self::SingleSig(s) => &s.singlesig_change_output_descriptor,
            Self::MultiSig(m) => &m.change_output_descriptor,
        }
    }

    /// Get the Bitcoin network this wallet is for
    pub fn network(&self) -> Network {
        let network_str = match self {
            Self::SingleSig(s) => &s.network,
            Self::MultiSig(m) => &m.network,
        };

        match network_str.as_str() {
            "bitcoin" => Network::Bitcoin,
            "testnet" => Network::Testnet,
            "regtest" => Network::Regtest,
            "signet" => Network::Signet,
            _ => Network::Bitcoin, // Default to mainnet
        }
    }

    /// Check if an address belongs to this wallet
    pub fn contains_address(&self, address: &str) -> bool {
        match self {
            Self::SingleSig(s) => {
                s.receiving_addresses.iter().any(|a| a.address == address)
                    || s.change_addresses.iter().any(|a| a.address == address)
            }
            Self::MultiSig(m) => {
                m.receiving_addresses.iter().any(|a| a.address == address)
                    || m.change_addresses.iter().any(|a| a.address == address)
            }
        }
    }

    /// Check if an address is a change address
    /// Returns Some(true) if it's a change address, Some(false) if it's a receiving address,
    /// or None if the address is not found in the wallet
    pub fn is_change_address(&self, address: &str) -> Option<bool> {
        match self {
            Self::SingleSig(s) => {
                if s.receiving_addresses.iter().any(|a| a.address == address) {
                    Some(false)
                } else if s.change_addresses.iter().any(|a| a.address == address) {
                    Some(true)
                } else {
                    None
                }
            }
            Self::MultiSig(m) => {
                if m.receiving_addresses.iter().any(|a| a.address == address) {
                    Some(false)
                } else if m.change_addresses.iter().any(|a| a.address == address) {
                    Some(true)
                } else {
                    None
                }
            }
        }
    }

    /// Get all receiving addresses
    pub fn receiving_addresses(&self) -> Vec<&str> {
        match self {
            Self::SingleSig(s) => s
                .receiving_addresses
                .iter()
                .map(|a| a.address.as_str())
                .collect(),
            Self::MultiSig(m) => m
                .receiving_addresses
                .iter()
                .map(|a| a.address.as_str())
                .collect(),
        }
    }

    /// Get all change addresses
    pub fn change_addresses(&self) -> Vec<&str> {
        match self {
            Self::SingleSig(s) => s
                .change_addresses
                .iter()
                .map(|a| a.address.as_str())
                .collect(),
            Self::MultiSig(m) => m
                .change_addresses
                .iter()
                .map(|a| a.address.as_str())
                .collect(),
        }
    }

    /// Get the master fingerprint for single-sig wallets
    pub fn master_fingerprint(&self) -> Option<&str> {
        match self {
            Self::SingleSig(s) => Some(&s.master_fingerprint),
            Self::MultiSig(_) => None,
        }
    }

    /// Get the wallet type as a string
    pub fn wallet_type(&self) -> &str {
        match self {
            Self::SingleSig(s) => &s.sigtype,
            Self::MultiSig(m) => &m.sigtype,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_singlesig_wallet() -> String {
        r#"{
  "wallet": "frozenkrill",
  "version": 0,
  "sigtype": "singlesig",
  "master_fingerprint": "84577e03",
  "singlesig_xpub": "zpub6rerbAfYxT86ZiHXkVYcJJLMFZzy5MF1gLmjuDFNxwN3NPZEC5PesEhzm5AYY7TJixkEAeFrNFRWEyRKLN9jLtNLeZkk2YchzaPkyL7eXqw",
  "singlesig_derivation_path": "84'/0'/0'",
  "multisig_xpub": "Zpub75EVv3vodU4dLT8VaR4eworLZJY1gnKyE1thSST7oQNMvPheFPLNfZhj9em55PyMtcju8A3DzTP3n8HCwgK7JbLJ6KKZf22f4Lw9ouMS2C2",
  "multisig_derivation_path": "48'/0'/0'/2'",
  "singlesig_receiving_output_descriptor": "wpkh([84577e03/84'/0'/0']xpub6CzKyqKif638s7uJ5myMt89Ludi5C7G1r7jJLRTcCvcHGBvmgm4Xd7PiifFNYJ9TugWcfh4jSviQUQCBtyKhkR18utMtriyjT8GUCAqCaC7/0/*)#x703tmpk",
  "singlesig_change_output_descriptor": "wpkh([84577e03/84'/0'/0']xpub6CzKyqKif638s7uJ5myMt89Ludi5C7G1r7jJLRTcCvcHGBvmgm4Xd7PiifFNYJ9TugWcfh4jSviQUQCBtyKhkR18utMtriyjT8GUCAqCaC7/1/*)#h22skw3w",
  "multisig_receiving_output_descriptor_key": "[84577e03/48'/0'/0'/2']xpub6DqBpAififEzKdbFYBcTXZRWebb8b5TbLRnQwqCbnX5bAqPXPxvBMEaQ8vSYdaeGvxvQdQCN3cz78Q4wJkY1LMAcXHK4zXACrMGC8VJx3rZ/0/*",
  "multisig_change_output_descriptor_key": "[84577e03/48'/0'/0'/2']xpub6DqBpAififEzKdbFYBcTXZRWebb8b5TbLRnQwqCbnX5bAqPXPxvBMEaQ8vSYdaeGvxvQdQCN3cz78Q4wJkY1LMAcXHK4zXACrMGC8VJx3rZ/1/*",
  "script_type": "segwit-native",
  "network": "bitcoin",
  "receiving_addresses": [
    {
      "address": "bc1qtm70qpd2h0v7kyaga72tq2nnmmjj5zcjerqq0c",
      "derivation_path": "84'/0'/0'/0/0"
    },
    {
      "address": "bc1q5q8wrzt3w2safvegkmml3lz0avvfrevn4udmx9",
      "derivation_path": "84'/0'/0'/0/1"
    }
  ],
  "change_addresses": [
    {
      "address": "bc1qs0kf5pf8ftfu2pzyd70sl9vl3885459znalqy2",
      "derivation_path": "84'/0'/0'/1/0"
    },
    {
      "address": "bc1qp2ejr88vsschtnyvcdqkxmwutlqzepw6r7vfzg",
      "derivation_path": "84'/0'/0'/1/1"
    }
  ]
}"#
        .to_string()
    }

    fn create_test_multisig_wallet() -> String {
        r#"{
  "wallet": "frozenkrill",
  "version": 0,
  "sigtype": "2-of-3",
  "script_type": "segwit-native",
  "network": "bitcoin",
  "receiving_output_descriptor": "wsh(sortedmulti(2,[1c2b4725/48'/0'/0'/2']xpub6Dp8hC2nCxMi8E8LwP8Wd2KzTnoe7PE8eRXt411uwvNqvwxYCGRiAxZvu4GRQQmXisb5PvUmERsehSPCU7SJAJDYri3BB3q9YzymHs4idPS/0/*,[88c3e90a/48'/0'/0'/2']xpub6DrbCDR2BEyrc9yFqvwb1rUPamk9ULmn9RSFBykKxWu3Ryh7bYoFAM9vhGiZ7fgVSSu2MB4UzUGvkreuVuH19rwAcZ4skxVb9R5PzXn1dMu/0/*,[a0342720/48'/0'/0'/2']xpub6Er6q7NDEyU4Z4KRZFMqh5R5vWbQXhnL4PhCpkXMW1CVq4N7VdccEX2RoTuAZXTmVqaTirR4JcmnDkEASVwetHZkisiqmhjmKBUd6KndpcC/0/*))#vcntmeq2",
  "change_output_descriptor": "wsh(sortedmulti(2,[1c2b4725/48'/0'/0'/2']xpub6Dp8hC2nCxMi8E8LwP8Wd2KzTnoe7PE8eRXt411uwvNqvwxYCGRiAxZvu4GRQQmXisb5PvUmERsehSPCU7SJAJDYri3BB3q9YzymHs4idPS/1/*,[88c3e90a/48'/0'/0'/2']xpub6DrbCDR2BEyrc9yFqvwb1rUPamk9ULmn9RSFBykKxWu3Ryh7bYoFAM9vhGiZ7fgVSSu2MB4UzUGvkreuVuH19rwAcZ4skxVb9R5PzXn1dMu/1/*,[a0342720/48'/0'/0'/2']xpub6Er6q7NDEyU4Z4KRZFMqh5R5vWbQXhnL4PhCpkXMW1CVq4N7VdccEX2RoTuAZXTmVqaTirR4JcmnDkEASVwetHZkisiqmhjmKBUd6KndpcC/1/*))#fdc39lsz",
  "receiving_addresses": [
    {
      "address": "bc1q9h4r8pk4p6ufsaaqpk5w09h7fr983nf9pcef6vfqcujawge2m3ssaqs7dp",
      "index": 0
    },
    {
      "address": "bc1qm78ddhzndtlqzc0fszr7kj3fvvj7xwu9uhdqfzxu4n4eklwevwqqqgg8at",
      "index": 1
    }
  ],
  "change_addresses": [
    {
      "address": "bc1ql9jt3v4q4f0shyqw0dzjw3hflq04xskqf26mqj6p7t3dga7j4y9sq9nz6g",
      "index": 0
    },
    {
      "address": "bc1qnzfp4sjypfyq3fcamjszrvwgqyg3gzjufk4w2qvw3u0vj2pz5zns2gaj5r",
      "index": 1
    }
  ]
}"#
        .to_string()
    }

    #[test]
    fn test_load_singlesig_wallet() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(create_test_singlesig_wallet().as_bytes())?;

        let wallet = FrozenkrillWallet::from_file(temp_file.path())?;

        match &wallet {
            FrozenkrillWallet::SingleSig(s) => {
                assert_eq!(s.master_fingerprint, "84577e03");
                assert_eq!(s.network, "bitcoin");
                assert_eq!(s.sigtype, "singlesig");
                assert_eq!(s.receiving_addresses.len(), 2);
                assert_eq!(s.change_addresses.len(), 2);
            }
            _ => panic!("Expected SingleSig wallet"),
        }

        assert_eq!(wallet.network(), Network::Bitcoin);
        assert_eq!(wallet.wallet_type(), "singlesig");
        assert!(wallet.contains_address("bc1qtm70qpd2h0v7kyaga72tq2nnmmjj5zcjerqq0c"));
        assert_eq!(
            wallet.is_change_address("bc1qtm70qpd2h0v7kyaga72tq2nnmmjj5zcjerqq0c"),
            Some(false)
        );
        assert_eq!(
            wallet.is_change_address("bc1qs0kf5pf8ftfu2pzyd70sl9vl3885459znalqy2"),
            Some(true)
        );
        assert_eq!(wallet.is_change_address("bc1qinvalid"), None);

        Ok(())
    }

    #[test]
    fn test_load_multisig_wallet() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(create_test_multisig_wallet().as_bytes())?;

        let wallet = FrozenkrillWallet::from_file(temp_file.path())?;

        match &wallet {
            FrozenkrillWallet::MultiSig(m) => {
                assert_eq!(m.network, "bitcoin");
                assert_eq!(m.sigtype, "2-of-3");
                assert_eq!(m.receiving_addresses.len(), 2);
                assert_eq!(m.change_addresses.len(), 2);
            }
            _ => panic!("Expected MultiSig wallet"),
        }

        assert_eq!(wallet.network(), Network::Bitcoin);
        assert_eq!(wallet.wallet_type(), "2-of-3");
        assert!(wallet
            .contains_address("bc1q9h4r8pk4p6ufsaaqpk5w09h7fr983nf9pcef6vfqcujawge2m3ssaqs7dp"));
        assert_eq!(wallet.master_fingerprint(), None); // No fingerprint for multisig

        Ok(())
    }

    #[test]
    fn test_invalid_wallet_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"invalid json").unwrap();

        let result = FrozenkrillWallet::from_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_wallet_descriptors() -> Result<()> {
        let mut temp_file = NamedTempFile::new()?;
        temp_file.write_all(create_test_singlesig_wallet().as_bytes())?;

        let wallet = FrozenkrillWallet::from_file(temp_file.path())?;

        assert!(wallet
            .receiving_descriptor()
            .starts_with("wpkh([84577e03/84'/0'/0']"));
        assert!(wallet
            .change_descriptor()
            .starts_with("wpkh([84577e03/84'/0'/0']"));
        assert!(wallet.receiving_descriptor().ends_with("/0/*)#x703tmpk"));
        assert!(wallet.change_descriptor().ends_with("/1/*)#h22skw3w"));

        Ok(())
    }
}
