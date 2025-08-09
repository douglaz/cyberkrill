# cyberkrill

<img src="https://github.com/user-attachments/assets/246dc789-4a2d-4040-afeb-3ac9045dddfb" width="200" />

A comprehensive Bitcoin and Lightning Network toolkit written in Rust. cyberkrill provides a unified command-line interface and reusable core library for working with Bitcoin, Lightning, and various hardware signing devices.

## Features

### 🌩️ Lightning Network
- **BOLT11 Invoice Decoding**: Parse and analyze Lightning invoices
- **LNURL Support**: Decode and process LNURL strings
- **Lightning Address**: Generate invoices from Lightning addresses (user@domain.com)
- **Fedimint Integration**: Encode/decode federation invite codes

### 💳 Smartcard Support (NFC/USB)
Native support for Coinkite smartcards via NFC readers:
- **Tapsigner**: BIP-32 HD wallet with secure key generation
  - Initialize new cards with secure entropy
  - Generate addresses with custom derivation paths
  - PIN-protected operations
- **Satscard**: Bearer instrument with 10 independent slots
  - Generate addresses from active slots
  - Track slot usage and history

### 🔐 Hardware Wallet Support
Integration with popular Bitcoin hardware wallets:
- **Coldcard**: Air-gapped signing device (USB/SD card)
  - Address generation and verification
  - PSBT signing and export
- **Trezor**: Full-featured hardware wallet (USB)
  - Extended public key extraction
  - Address generation with custom paths
- **Jade**: Blockstream's hardware wallet (USB/Bluetooth)
  - Async communication support
  - Address generation and PSBT signing

### ₿ Bitcoin Operations
Powered by BDK (Bitcoin Development Kit) with multiple backend support:

**Blockchain Backends:**
- **Bitcoin Core RPC**: Direct node integration for maximum privacy
- **Electrum**: Fast SPV operations without full node
- **Esplora**: RESTful API for lightweight setups

**Transaction Features:**
- **UTXO Management**: List and analyze unspent outputs
- **PSBT Creation**: Three approaches for different use cases:
  - Manual: Full control over inputs/outputs
  - Funded: Automatic coin selection and change
  - Consolidation: Merge multiple UTXOs efficiently
- **Smart Coin Selection**: Intelligent UTXO selection with amount limits
- **Sub-satoshi Precision**: Support for fractional fee rates (0.1 sats/vB)
- **Descriptor Support**: Full output descriptor compatibility
- **[frozenkrill](https://github.com/planktonlabs/frozenkrill) Integration**: Import wallet export files

## Command Structure

Commands are organized with logical prefixes for better clarity:
- `ln-*` - Lightning Network operations
- `fm-*` - Fedimint operations  
- `hw-*` - Hardware wallet operations
- `onchain-*` - Bitcoin onchain operations

> **Note**: Old command names (without prefixes) are still supported for backward compatibility but are deprecated.

## Installation

### Using Nix (Recommended)

```bash
# Run directly from GitHub
nix run 'git+https://github.com/douglaz/cyberkrill.git'

# Or clone and run locally
git clone https://github.com/douglaz/cyberkrill.git
cd cyberkrill
nix run .
```

### Using Cargo

```bash
git clone https://github.com/douglaz/cyberkrill.git
cd cyberkrill

# Build with all features (recommended)
cargo build --release

# The binary will be at ./target/release/cyberkrill
./target/release/cyberkrill --help
```

## Quick Start

### Lightning Operations

```bash
# Decode a Lightning invoice
cyberkrill ln-decode-invoice lnbc1000n1pn...

# Decode LNURL
cyberkrill ln-decode-lnurl lnurl1dp68gurn8ghj7mr0v...

# Generate invoice from Lightning address
cyberkrill ln-generate-invoice user@getalby.com 100000 --comment "Payment"
```

### Fedimint Operations

```bash
# Decode Fedimint invite code
cyberkrill fm-decode-invite fed11qgqz...

# Encode invite code from JSON
cyberkrill fm-encode-invite invite.json

# Fetch federation configuration
cyberkrill fm-fetch-config fed11qgqz...
```

### Hardware Wallet Operations

#### Smartcards (Tapsigner/Satscard)

```bash
# Initialize Tapsigner (one-time setup)
export TAPSIGNER_CVC=123456  # Your 6-digit PIN
cyberkrill hw-tapsigner-init

# Generate Bitcoin address from Tapsigner
cyberkrill hw-tapsigner-address --path "m/84'/0'/0'/0/0"

# Generate address from Satscard
cyberkrill hw-satscard-address --slot 1
```

#### Hardware Wallets

```bash
# Coldcard - Generate address
cyberkrill hw-coldcard-address --path "m/84'/0'/0'/0/0" --network mainnet

# Trezor - Generate address
cyberkrill hw-trezor-address --path "m/84'/0'/0'/0/0" --network mainnet

# Jade - Generate address
cyberkrill hw-jade-address --path "m/84'/0'/0'/0/0" --network mainnet

# Jade - Get extended public key
cyberkrill hw-jade-xpub --path "m/84'/0'/0'" --network mainnet
```

### Bitcoin Onchain Operations

#### List UTXOs

```bash
# Using different backends
# Bitcoin Core (default)
cyberkrill onchain-list-utxos --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Electrum backend
cyberkrill onchain-list-utxos --descriptor "wpkh([...]xpub...)" \
  --electrum ssl://electrum.blockstream.info:50002

# Esplora backend
cyberkrill onchain-list-utxos --descriptor "wpkh([...]xpub...)" \
  --esplora https://blockstream.info/api

# Using frozenkrill wallet file
cyberkrill onchain-list-utxos --wallet-file mywallet_pub.json
```

#### Create Transactions

```bash
# Manual PSBT - Full control
cyberkrill onchain-create-psbt \
  --inputs "txid:0" --inputs "txid:1" \
  --outputs "bc1qaddr:0.001" \
  --fee-rate 10sats

# Funded PSBT - Automatic coin selection
cyberkrill onchain-create-funded-psbt \
  --outputs "bc1qaddr:0.001" \
  --fee-rate 15.5sats

# UTXO Consolidation
cyberkrill onchain-move-utxos \
  --inputs "txid:0" --inputs "txid:1" \
  --destination "bc1qconsolidated" \
  --fee-rate 5sats

# Decode PSBT
cyberkrill onchain-decode-psbt transaction.psbt
```

## Complete Command Reference

### Lightning Network Commands (ln-*)

| Command | Description |
|---------|-------------|
| `ln-decode-invoice` | Decode BOLT11 Lightning invoice |
| `ln-decode-lnurl` | Decode LNURL string |
| `ln-generate-invoice` | Generate invoice from Lightning address |

### Fedimint Commands (fm-*)

| Command | Description |
|---------|-------------|
| `fm-decode-invite` | Decode Fedimint invite code |
| `fm-encode-invite` | Encode Fedimint invite code from JSON |
| `fm-fetch-config` | Fetch Fedimint federation configuration |

### Hardware Wallet Commands (hw-*)

| Command | Description |
|---------|-------------|
| `hw-tapsigner-init` | Initialize Tapsigner (one-time setup) |
| `hw-tapsigner-address` | Generate Bitcoin address from Tapsigner |
| `hw-satscard-address` | Generate Bitcoin address from Satscard |
| `hw-coldcard-address` | Generate Bitcoin address from Coldcard |
| `hw-coldcard-sign-psbt` | Sign PSBT with Coldcard |
| `hw-coldcard-export-psbt` | Export PSBT to Coldcard SD card |
| `hw-trezor-address` | Generate Bitcoin address from Trezor |
| `hw-trezor-sign-psbt` | Sign PSBT with Trezor |
| `hw-jade-address` | Generate Bitcoin address from Jade |
| `hw-jade-xpub` | Get extended public key from Jade |
| `hw-jade-sign-psbt` | Sign PSBT with Jade |

### Bitcoin Onchain Commands (onchain-*)

| Command | Description |
|---------|-------------|
| `onchain-list-utxos` | List UTXOs for addresses or descriptors |
| `onchain-create-psbt` | Create PSBT with manual input/output specification |
| `onchain-create-funded-psbt` | Create funded PSBT with automatic coin selection |
| `onchain-move-utxos` | Consolidate/move UTXOs to a single destination |
| `onchain-decode-psbt` | Decode a PSBT (Partially Signed Bitcoin Transaction) |

## Backend Configuration

### Bitcoin Core RPC

```bash
# Using cookie authentication (recommended)
cyberkrill onchain-list-utxos --bitcoin-dir ~/.bitcoin --descriptor "..."

# Using username/password
cyberkrill onchain-list-utxos --rpc-user myuser --rpc-password mypass --descriptor "..."
```

### Electrum

Popular public servers:
- Mainnet: `ssl://electrum.blockstream.info:50002`
- Testnet: `ssl://electrum.blockstream.info:60002`

### Esplora

Public instances:
- Mainnet: `https://blockstream.info/api`
- Testnet: `https://blockstream.info/testnet/api`

## Advanced Features

### Amount Formats

cyberkrill supports flexible amount inputs:

```bash
# Fee rates (supports fractional satoshis)
--fee-rate 0.1sats     # 0.1 sat/vB
--fee-rate 15.5sats    # 15.5 sat/vB
--fee-rate 0.00000020btc  # In BTC

# Output amounts
--outputs "bc1q...:0.001"      # 0.001 BTC
--outputs "bc1q...:0.001btc"   # Explicit BTC
--outputs "bc1q...:100000sats" # In satoshis
```

### Output Descriptors

Full support for Bitcoin output descriptors:

```bash
# Single-sig descriptor
"wpkh([fingerprint/84'/0'/0']xpub...)"

# Multi-sig descriptor with change paths
"wsh(sortedmulti(2,[fp1/48'/0'/0'/2']xpub1/<0;1>/*,[fp2/48'/0'/0'/2']xpub2/<0;1>/*))"
```

### frozenkrill Wallet Files

Import [frozenkrill](https://github.com/planktonlabs/frozenkrill) wallet export files instead of raw descriptors:

```bash
cyberkrill onchain-list-utxos --wallet-file mywallet_pub.json
cyberkrill onchain-create-funded-psbt --wallet-file mywallet_pub.json --outputs "bc1q...:0.001"
```

## Documentation

Detailed documentation for specific topics:

- [Hardware Wallet Setup](docs/hardware-wallets/)
  - [Coldcard Guide](docs/hardware-wallets/coldcard.md)
  - [Smartcards Guide](docs/hardware-wallets/smartcards.md)
  - [Trezor Guide](docs/hardware-wallets/trezor.md)
  - [Jade Integration](docs/hardware-wallets/jade-integration-plan.md)
- [Development](docs/development/)
  - [BDK Implementation](docs/development/bdk-implementation.md)

## Using as a Library

The `cyberkrill-core` crate can be used as a dependency:

```toml
[dependencies]
# Basic functionality
cyberkrill-core = { git = "https://github.com/douglaz/cyberkrill", default-features = false }

# With smartcard support
cyberkrill-core = { git = "https://github.com/douglaz/cyberkrill", features = ["smartcards"] }

# With hardware wallet support
cyberkrill-core = { git = "https://github.com/douglaz/cyberkrill", features = ["coldcard", "trezor", "jade"] }
```

```rust
use cyberkrill_core::{decode_invoice, decode_lnurl, BitcoinRpcClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Decode Lightning invoice
    let invoice = decode_invoice("lnbc...")?;
    println!("Amount: {} msat", invoice.amount_msat);
    
    // Bitcoin operations
    let client = BitcoinRpcClient::new_auto(
        "http://127.0.0.1:8332".to_string(),
        Some(std::path::Path::new("~/.bitcoin")),
        None, None,
    )?;
    
    Ok(())
}
```

## Architecture

cyberkrill is organized as a Rust workspace with three main crates:

- **cyberkrill**: CLI application providing the command-line interface
- **cyberkrill-core**: Core library with all business logic
- **fedimint-lite**: Standalone Fedimint invite code handling

## Development

```bash
# Enter development environment
nix develop

# Build
cargo build

# Run tests
cargo test

# Format and lint
cargo fmt
cargo clippy
```

## Contributing

1. Follow the coding conventions in `CONVENTIONS.md`
2. Add tests for new functionality
3. Run quality checks before submitting PRs
4. Update documentation for new features

## License

This project is open source. See the repository for license details.