# jade-bitcoin

Bitcoin-focused Rust client for Blockstream Jade hardware wallet.

## Features

- Pure Rust implementation (no Python dependencies)
- Bitcoin-only focus (no Liquid/Elements dependencies)
- Auto-detection of Jade devices
- Support for all Bitcoin networks (mainnet, testnet, regtest, signet)
- BIP32/44/49/84/86 derivation paths
- PSBT signing
- Message signing
- Clean, simple API

## Installation

```toml
[dependencies]
jade-bitcoin = "0.1"
```

## Usage

```rust
use jade_bitcoin::{JadeClient, Network};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to Jade
    let mut jade = JadeClient::connect()?;
    
    // Unlock for Bitcoin mainnet
    jade.unlock(Network::Bitcoin)?;
    
    // Get a Bitcoin address
    let address = jade.get_address("m/84'/0'/0'/0/0", Network::Bitcoin)?;
    println!("Address: {}", address);
    
    // Get extended public key
    let xpub = jade.get_xpub("m/84'/0'/0'")?;
    println!("xpub: {}", xpub);
    
    // Sign a PSBT
    let psbt_bytes = std::fs::read("transaction.psbt")?;
    let signed = jade.sign_psbt(&psbt_bytes, Network::Bitcoin)?;
    std::fs::write("signed.psbt", signed)?;
    
    Ok(())
}
```

## Hardware Setup

### Linux USB Permissions

To access Jade without root privileges, add udev rules:

```bash
# /etc/udev/rules.d/51-jade.rules
SUBSYSTEM=="tty", ATTRS{idVendor}=="10c4", ATTRS{idProduct}=="ea60", MODE="0666"
SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="55d4", MODE="0666"
SUBSYSTEM=="tty", ATTRS{idVendor}=="0403", ATTRS{idProduct}=="6001", MODE="0666"
```

Then reload udev rules:
```bash
sudo udevadm control --reload-rules
sudo udevadm trigger
```

## Examples

See the `examples/` directory for more usage examples:
- `get_address.rs` - Generate Bitcoin addresses
- `sign_psbt.rs` - Sign transactions
- `get_xpub.rs` - Get extended public keys

## License

MIT OR Apache-2.0