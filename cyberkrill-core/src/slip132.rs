// SLIP-0132 extended public key format support
// Adapted from frozenkrill-core for Trezor compatibility

use anyhow::{bail, Result};
use bitcoin::base58;
use bitcoin::bip32::Xpub;

/// Magical version bytes for xpub: bitcoin mainnet public key for P2PKH or P2SH
pub const VERSION_MAGIC_XPUB: [u8; 4] = [0x04, 0x88, 0xB2, 0x1E];
/// Magical version bytes for ypub: bitcoin mainnet public key for P2WPKH in P2SH
pub const VERSION_MAGIC_YPUB: [u8; 4] = [0x04, 0x9D, 0x7C, 0xB2];
/// Magical version bytes for zpub: bitcoin mainnet public key for P2WPKH
pub const VERSION_MAGIC_ZPUB: [u8; 4] = [0x04, 0xB2, 0x47, 0x46];

/// Magical version bytes for tpub: bitcoin testnet/regtest public key for P2PKH or P2SH
pub const VERSION_MAGIC_TPUB: [u8; 4] = [0x04, 0x35, 0x87, 0xCF];
/// Magical version bytes for upub: bitcoin testnet/regtest public key for P2WPKH in P2SH
pub const VERSION_MAGIC_UPUB: [u8; 4] = [0x04, 0x4A, 0x52, 0x62];
/// Magical version bytes for vpub: bitcoin testnet/regtest public key for P2WPKH
pub const VERSION_MAGIC_VPUB: [u8; 4] = [0x04, 0x5F, 0x1C, 0xF6];

/// Trait for building standard BIP32 extended keys from SLIP132 variant.
pub trait FromSlip132 {
    /// Constructs standard BIP32 extended key from SLIP132 string.
    fn from_slip132_str(s: &str) -> Result<Self>
    where
        Self: Sized;
}

impl FromSlip132 for Xpub {
    fn from_slip132_str(s: &str) -> Result<Self> {
        let mut data = base58::decode_check(s)?;

        let mut prefix = [0u8; 4];
        prefix.copy_from_slice(&data[0..4]);
        
        // Convert SLIP-0132 format to standard xpub/tpub
        let slice = match prefix {
            // Mainnet variants -> xpub
            VERSION_MAGIC_XPUB | VERSION_MAGIC_YPUB | VERSION_MAGIC_ZPUB => VERSION_MAGIC_XPUB,
            // Testnet variants -> tpub
            VERSION_MAGIC_TPUB | VERSION_MAGIC_UPUB | VERSION_MAGIC_VPUB => VERSION_MAGIC_TPUB,
            _ => bail!("Unknown SLIP-0132 prefix: {:?}", prefix),
        };
        
        data[0..4].copy_from_slice(&slice);
        let xpub = Xpub::decode(&data)?;
        
        Ok(xpub)
    }
}

/// Helper function to convert any SLIP-0132 format to standard Xpub
pub fn parse_slip132_xpub(xpub_str: &str) -> Result<Xpub> {
    // First try to parse as standard xpub/tpub
    if let Ok(xpub) = xpub_str.parse::<Xpub>() {
        return Ok(xpub);
    }
    
    // Fall back to SLIP-0132 parsing
    Xpub::from_slip132_str(xpub_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_xpub_from_slip132_str() -> Result<()> {
        // Standard xpub should work
        let xpub_str = "xpub6BosfCnifzxcJJ1wYuntGJfF2zPJkDeG9ELNHcKNjezuea4tumswN9sH1psMdSVqCMoJC21Bv8usSeqSP4Sp1tLzW7aY59fGn9GCYzx5UTo";
        let xpub = Xpub::from_str(xpub_str)?;
        assert_eq!(Xpub::from_slip132_str(xpub_str)?, xpub);

        // ypub (BIP49) should convert to xpub
        let ypub_str = "ypub6We8xsTdpgW69bD4PGaWUPkkCxXkgqdm4Lrb51DG7fNnhft8AS3VzDXR32pwdM9kbzv6wVbkNoGRKwT16krpp82bNTGxf4Um3sKqwYoGn8q";
        assert_eq!(Xpub::from_slip132_str(ypub_str)?, xpub);

        // zpub (BIP84) should convert to xpub
        let zpub_str = "zpub6qUQGY8YyN3ZztQBDdN8gUrFNvgCdTdFyTNorQ79VfkfkmhMR6D4cHBZ4EnXdFog1e2ugyCJqTcyDE4ZpTGqcMiCEnyPEyJFKbPVL9knhKU";
        assert_eq!(Xpub::from_slip132_str(zpub_str)?, xpub);

        Ok(())
    }

    #[test]
    fn test_parse_slip132_xpub() -> Result<()> {
        // Standard xpub
        let xpub_str = "xpub6BosfCnifzxcJJ1wYuntGJfF2zPJkDeG9ELNHcKNjezuea4tumswN9sH1psMdSVqCMoJC21Bv8usSeqSP4Sp1tLzW7aY59fGn9GCYzx5UTo";
        let xpub = parse_slip132_xpub(xpub_str)?;
        assert_eq!(xpub.to_string(), xpub_str);

        // zpub should parse correctly
        let zpub_str = "zpub6qUQGY8YyN3ZztQBDdN8gUrFNvgCdTdFyTNorQ79VfkfkmhMR6D4cHBZ4EnXdFog1e2ugyCJqTcyDE4ZpTGqcMiCEnyPEyJFKbPVL9knhKU";
        let parsed = parse_slip132_xpub(zpub_str)?;
        assert_eq!(parsed, xpub);

        Ok(())
    }
}