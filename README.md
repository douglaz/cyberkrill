# CyberKrill

<img src="https://github.com/user-attachments/assets/246dc789-4a2d-4040-afeb-3ac9045dddfb" width="200" />

A comprehensive CLI toolkit for Bitcoin and Lightning Network operations, written in Rust.

## Features

- **Lightning Network**: BOLT11 invoice decoding, LNURL processing, and invoice generation from Lightning addresses
- **Hardware Wallets** (optional): Full integration with Coinkite Tapsigner and Satscard devices (requires `smartcards` feature)
- **Bitcoin Core RPC**: UTXO management, PSBT creation, and transaction building
- **JSON Output**: All commands output structured JSON for easy integration with other tools

## Installation

### Using Nix (Recommended)

```bash
# Run directly from GitHub
nix run 'git+https://github.com/douglaz/cyberkrill.git'

# Or install locally
git clone https://github.com/douglaz/cyberkrill.git
cd cyberkrill
nix run .
```

### Using Cargo

```bash
git clone https://github.com/douglaz/cyberkrill.git
cd cyberkrill

# Build without hardware wallet support
cargo build --release

# Build with hardware wallet support (Tapsigner/Satscard)
cargo build --release --features smartcards

./target/release/cyberkrill --help
```

## Commands Overview

### Lightning Network Operations

#### Decode Lightning Invoices

```bash
# Decode a BOLT11 invoice
cyberkrill decode invoice lnbc99810310n1pju0sy7pp555srgtgcg6t4jr4j5v0jysgee4zy6nr4msylnycfjezxm5w6t3csdy9w...

# From file or stdin
echo "lnbc..." | cyberkrill decode invoice
cyberkrill decode invoice -o decoded_invoice.json
```

#### Decode LNURL

```bash
# Decode LNURL strings
cyberkrill decode lnurl lnurl1dp68gurn8ghj7mr0vdskc6r0wd6z7mrww4excttsv9un7um9wdekjmmw84jxywf5x43rvv35xgmr2enrxanr2cfcvsmnwe3jxcukvde48qukgdec89snwde3vfjxvepjxpjnjvtpxd3kvdnxx5crxwpjvyunsephsz36jf

# Save to file
cyberkrill decode lnurl <lnurl_string> -o decoded_lnurl.json
```

#### Generate Lightning Invoices

```bash
# Generate invoice from Lightning address using LNURL-pay
cyberkrill generate invoice user@domain.com 1000000 --comment "Payment for service"

# Save to file
cyberkrill generate invoice user@domain.com 1000000 -o invoice.json
```

### Hardware Wallet Operations

**Note**: Hardware wallet commands require building with the `smartcards` feature:
```bash
# Build with smartcard support
cargo build --features smartcards

# Or run directly with the feature
cargo run --features smartcards -- tapsigner --help
```

#### Tapsigner

**Initial Setup (One-time Only):**
```bash
# Set your card's 6-digit PIN
export TAPSIGNER_CVC=123456

# Initialize new card (IRREVERSIBLE)
cyberkrill tapsigner init  # (requires --features smartcards)

# Initialize with custom entropy
cyberkrill tapsigner init --chain-code "0123456789abcdef..."  # (requires --features smartcards)
```

**Generate Bitcoin Addresses:**
```bash
# Generate address with default BIP-84 path
cyberkrill tapsigner address  # (requires --features smartcards)

# Custom derivation path
cyberkrill tapsigner address --path "m/84'/0'/0'/0/5"  # (requires --features smartcards)

# Save to file
cyberkrill tapsigner address -o address.json  # (requires --features smartcards)
```

#### Satscard

```bash
# Generate address from current active slot
cyberkrill satscard address  # (requires --features smartcards)

# Generate from specific slot (0-9)
cyberkrill satscard address --slot 2  # (requires --features smartcards)

# Save to file
cyberkrill satscard address -o satscard_address.json  # (requires --features smartcards)
```

### Bitcoin Core RPC Operations

#### List UTXOs

```bash
# Using output descriptor (recommended)
cyberkrill bitcoin list-utxos --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Using specific addresses
cyberkrill bitcoin list-utxos --addresses "bc1qtest1,bc1qtest2"

# Custom Bitcoin directory
cyberkrill bitcoin list-utxos --bitcoin-dir /path/to/.bitcoin --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Username/password authentication
cyberkrill bitcoin list-utxos --rpc-user myuser --rpc-password mypass --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"
```

#### Create PSBT (Partially Signed Bitcoin Transaction)

```bash
# Create PSBT with manual inputs/outputs
cyberkrill bitcoin create-psbt \
  --inputs "txid1:0,txid2:1" \
  --outputs "bc1qaddr1:0.001,bc1qaddr2:0.002" \
  --fee-rate 10.5

# Save both JSON and raw PSBT
cyberkrill bitcoin create-psbt \
  --inputs "txid:vout" \
  --outputs "address:amount" \
  --output response.json \
  --psbt-output transaction.psbt
```

#### Create Funded PSBT (Automatic Input Selection)

```bash
# Let Bitcoin Core select inputs automatically
cyberkrill bitcoin create-funded-psbt \
  --outputs "bc1qaddr1:0.001,bc1qaddr2:0.002" \
  --conf-target 6 \
  --estimate-mode CONSERVATIVE

# With specific fee rate
cyberkrill bitcoin create-funded-psbt \
  --outputs "bc1qaddr1:0.001" \
  --fee-rate 15.0 \
  --output funded.json \
  --psbt-output funded.psbt
```

## Configuration

### Hardware Wallet Setup

**Feature Requirement:**
Hardware wallet functionality requires building with the `smartcards` feature:
```bash
cargo build --features smartcards
```

**System Requirements:**
- USB NFC card reader (e.g., OMNIKEY 5022 CL)
- PCSC daemon running (`pcscd`)
- Coinkite Tapsigner or Satscard device

**Tapsigner Authentication:**
```bash
# Set 6-digit PIN (found on card back or documentation)
export TAPSIGNER_CVC=123456
```

### Bitcoin Core RPC Setup

**Cookie Authentication (Recommended):**
```bash
# Default Bitcoin directory
cyberkrill bitcoin list-utxos --descriptor "wpkh([...]xpub...)"

# Custom directory
cyberkrill bitcoin list-utxos --bitcoin-dir /custom/bitcoin/dir --descriptor "wpkh([...]xpub...)"
```

**Username/Password Authentication:**
Add to your `bitcoin.conf`:
```
rpcuser=myuser
rpcpassword=mypassword
```

Then use:
```bash
cyberkrill bitcoin list-utxos --rpc-user myuser --rpc-password mypassword --descriptor "wpkh([...]xpub...)"
```

## Example Outputs

### Lightning Invoice Decoding

```json
{
  "network": "bitcoin",
  "amount_msats": 9981031000,
  "timestamp_millis": 1707589790000,
  "payment_hash": "a520342d184697590eb2a31f224119cd444d4c75dc09f9930996446dd1da5c71",
  "payment_secret": "2a334f966d764998566b48dd08f62b85c3602cf9243869ae81df3042dd865df6",
  "description": "swap - script: 5120ca672c2616841c55dddcb1ddfa429fd35191d72afd8f77cf88d21154fb907859",
  "destination": "03fb2a0ca79c005f493f1faa83071d3a937cf220d4051dc48b8fe3a087879cf14a",
  "expiry_seconds": 31536000,
  "min_final_cltv_expiry": 200,
  "fallback_addresses": [],
  "routes": [...]
}
```

### Tapsigner Address Generation

```json
{
  "derivation_path": "m/84'/0'/0'/0/0",
  "address": "bc1qy80agvcq084qtsdg3wayr2uzxweqmsx7xed9s5",
  "pubkey": "02856528bfb921cfb18c9b5427ecada29a2fc72e55671b8fe131d1691b722de986",
  "master_pubkey": "0379890f62200b30e6c33ece95d7be439184c1280366f5b3ebed60b3e946681b68",
  "master_fingerprint": "a1b2c3d4",
  "chain_code": "b278131303d560983aa72e0ee571a9c9b7b38b19aab335a1f3a0b8395338b4e7"
}
```

### Bitcoin UTXO Listing

```json
{
  "utxos": [
    {
      "txid": "abc123...",
      "vout": 0,
      "address": "bc1qtest123...",
      "amount_btc": 0.001,
      "amount_sats": 100000,
      "confirmations": 6,
      "spendable": true,
      "solvable": true
    }
  ],
  "total_amount_btc": 0.001,
  "total_amount_sats": 100000,
  "utxo_count": 1
}
```

## Development

### Prerequisites

```bash
# Enter development environment
nix develop

# Or install dependencies manually:
# - Rust toolchain
# - pkg-config
# - pcsclite (for hardware wallet support)
```

### Building and Testing

```bash
# Build (without hardware wallet support)
cargo build

# Build with hardware wallet support
cargo build --features smartcards

# Run tests
cargo test

# Run tests with hardware wallet features
cargo test --features smartcards

# Run linting
cargo clippy

# Format code
cargo fmt
```

### Quality Checks

Before committing, run:
```bash
cargo test && cargo clippy && cargo fmt --check
```

## Architecture

- **`src/main.rs`** - CLI interface and command dispatching
- **`src/decoder.rs`** - Lightning invoice/LNURL decoding and generation
- **`src/tapsigner.rs`** - Tapsigner hardware wallet operations (requires `smartcards` feature)
- **`src/satscard.rs`** - Satscard hardware wallet operations (requires `smartcards` feature)
- **`src/bitcoin_rpc.rs`** - Bitcoin Core RPC client and transaction building

## License

This project is open source. See the repository for license details.

## Contributing

1. Follow the coding conventions in `CONVENTIONS.md`
2. Add tests for new functionality
3. Run quality checks before submitting
4. Update documentation for new features