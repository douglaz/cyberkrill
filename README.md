# CyberKrill

<img src="https://github.com/user-attachments/assets/246dc789-4a2d-4040-afeb-3ac9045dddfb" width="200" />

A comprehensive CLI toolkit for Bitcoin and Lightning Network operations, written in Rust. CyberKrill consists of a command-line application and a reusable core library.

## Features

- **Lightning Network**: BOLT11 invoice decoding, LNURL processing, and invoice generation from Lightning addresses
- **Hardware Wallets** (optional): Full integration with Coinkite Tapsigner and Satscard devices (requires `smartcards` feature)
- **Bitcoin Core RPC**: Advanced UTXO management, PSBT creation with automatic change address derivation, and transaction building
- **Sub-satoshi Precision**: Support for fractional satoshi fee rates (e.g., "0.1sats/vB") using millisatoshi precision
- **Smart Coin Selection**: Automatic coin selection with max-amount limits and descriptor-based input selection
- **Output Descriptor Support**: Use descriptors as inputs with automatic UTXO discovery and change address derivation
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

# Build with hardware wallet support (default)
cargo build --release

# Build without hardware wallet support (if needed)
cargo build --release --no-default-features

./target/release/cyberkrill --help
```

## Commands Overview

### Lightning Network Operations

#### Decode Lightning Invoices

```bash
# Decode a BOLT11 invoice
cyberkrill decode-invoice lnbc99810310n1pju0sy7pp555srgtgcg6t4jr4j5v0jysgee4zy6nr4msylnycfjezxm5w6t3csdy9w...

# From file or stdin
echo "lnbc..." | cyberkrill decode-invoice
cyberkrill decode-invoice -o decoded_invoice.json
```

#### Decode LNURL

```bash
# Decode LNURL strings
cyberkrill decode-lnurl lnurl1dp68gurn8ghj7mr0vdskc6r0wd6z7mrww4excttsv9un7um9wdekjmmw84jxywf5x43rvv35xgmr2enrxanr2cfcvsmnwe3jxcukvde48qukgdec89snwde3vfjxvepjxpjnjvtpxd3kvdnxx5crxwpjvyunsephsz36jf

# Save to file
cyberkrill decode-lnurl <lnurl_string> -o decoded_lnurl.json
```

#### Generate Lightning Invoices

```bash
# Generate invoice from Lightning address using LNURL-pay
cyberkrill generate-invoice user@domain.com 1000000 --comment "Payment for service"

# Save to file
cyberkrill generate-invoice user@domain.com 1000000 -o invoice.json
```

### Hardware Wallet Operations

**Note**: Hardware wallet functionality is included by default. No special build flags required.

#### Tapsigner

**Initial Setup (One-time Only):**
```bash
# Set your card's 6-digit PIN
export TAPSIGNER_CVC=123456

# Initialize new card (IRREVERSIBLE)
cyberkrill tapsigner-init

# Initialize with custom entropy
cyberkrill tapsigner-init --chain-code "0123456789abcdef..."
```

**Generate Bitcoin Addresses:**
```bash
# Generate address with default BIP-84 path
cyberkrill tapsigner-address

# Custom derivation path
cyberkrill tapsigner-address --path "m/84'/0'/0'/0/5"

# Save to file
cyberkrill tapsigner-address -o address.json
```

#### Satscard

```bash
# Generate address from current active slot
cyberkrill satscard-address

# Generate from specific slot (0-9)
cyberkrill satscard-address --slot 2

# Save to file
cyberkrill satscard-address -o satscard_address.json
```

### Bitcoin Core RPC Operations

#### List UTXOs

```bash
# Using output descriptor (recommended)
cyberkrill list-utxos --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Using specific addresses
cyberkrill list-utxos --addresses "bc1qtest1,bc1qtest2"

# Custom Bitcoin directory
cyberkrill list-utxos --bitcoin-dir /path/to/.bitcoin --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Username/password authentication
cyberkrill list-utxos --rpc-user myuser --rpc-password mypass --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"
```

#### Create PSBT (Partially Signed Bitcoin Transaction)

```bash
# Create PSBT with manual inputs/outputs
cyberkrill create-psbt \
  --inputs "txid1:0" --inputs "txid2:1" \
  --outputs "bc1qaddr1:0.001,bc1qaddr2:0.002" \
  --fee-rate 10.5sats

# Using output descriptors as inputs (NEW!)
cyberkrill create-psbt \
  --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --outputs "bc1qaddr:0.001btc" \
  --fee-rate 15sats

# Mix specific UTXOs and descriptors
cyberkrill create-psbt \
  --inputs "txid1:0" \
  --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --outputs "bc1qaddr:0.001" \
  --fee-rate 20.5sats

# Save both JSON and raw PSBT
cyberkrill create-psbt \
  --inputs "txid:vout" \
  --outputs "address:amount" \
  --output response.json \
  --psbt-output transaction.psbt
```

#### Create Funded PSBT (Automatic Input Selection)

```bash
# Let Bitcoin Core select inputs automatically
cyberkrill create-funded-psbt \
  --outputs "bc1qaddr1:0.001,bc1qaddr2:0.002" \
  --conf-target 6 \
  --estimate-mode CONSERVATIVE

# With specific fee rate (supports fractional sats/vB)
cyberkrill create-funded-psbt \
  --outputs "bc1qaddr1:0.01btc" \
  --fee-rate 0.1sats \
  --output funded.json \
  --psbt-output funded.psbt

# Using descriptors with automatic change address derivation (NEW!)
cyberkrill create-funded-psbt \
  --inputs "wpkh([fingerprint/84'/0'/0']xpub.../<0;1>/*)" \
  --outputs "bc1qaddr:0.01btc" \
  --fee-rate 1.5sats

# Partial input specification with descriptors
cyberkrill create-funded-psbt \
  --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --outputs "bc1qaddr:0.001" \
  --fee-rate 20sats
```

#### Consolidate UTXOs (Move UTXOs)

```bash
# Consolidate specific UTXOs to single address
cyberkrill move-utxos \
  --inputs "txid1:0" --inputs "txid2:1" --inputs "txid3:0" \
  --destination "bc1qconsolidated_address" \
  --fee-rate 15sats

# Consolidate all UTXOs from a descriptor (NEW!)
cyberkrill move-utxos \
  --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --destination "bc1qconsolidated_address" \
  --fee-rate 20sats

# Mix specific UTXOs and descriptors
cyberkrill move-utxos \
  --inputs "txid1:0" \
  --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --destination "bc1qmy_address" \
  --fee-rate 25sats

# Use absolute fee instead of fee rate
cyberkrill move-utxos \
  --inputs "txid1:0" --inputs "txid2:1" \
  --destination "bc1qmy_address" \
  --fee 5000sats

# Limit total amount moved with coin selection (NEW!)
cyberkrill move-utxos \
  --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --destination "bc1qmy_address" \
  --fee-rate 15sats \
  --max-amount 0.5btc

# Max amount in satoshis with absolute fee
cyberkrill move-utxos \
  --inputs "txid1:0" --inputs "txid2:1" --inputs "txid3:2" \
  --destination "bc1qmy_address" \
  --fee 10000sats \
  --max-amount 25000000sats

# Save to files
cyberkrill move-utxos \
  --inputs "txid1:0" --inputs "txid2:1" \
  --destination "bc1qmy_address" \
  --fee-rate 15sats \
  --output consolidation.json \
  --psbt-output consolidation.psbt
```

### BDK Wallet Operations (Bitcoin Development Kit)

CyberKrill includes BDK 2.0 integration for working with Bitcoin descriptors and UTXOs. The `bdk-list-utxos` command provides an alternative way to list UTXOs using BDK's wallet functionality.

#### List UTXOs with BDK

```bash
# List UTXOs using BDK with Bitcoin Core backend
cyberkrill bdk-list-utxos \
  --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --bitcoin-dir ~/libre

# With multipath descriptors (automatically expanded)
cyberkrill bdk-list-utxos \
  --descriptor "wpkh([fingerprint/84'/0'/0']xpub.../<0;1>/*)" \
  --bitcoin-dir ~/.bitcoin

# Complex multisig with multipath
cyberkrill bdk-list-utxos \
  --descriptor "wsh(sortedmulti(4,xpub1/<0;1>/*,xpub2/<0;1>/*,...))" \
  --bitcoin-dir ~/libre

# Different networks
cyberkrill bdk-list-utxos \
  --descriptor "wpkh([fingerprint/84'/1'/0']tpub...)" \
  --network testnet \
  --bitcoin-dir ~/.bitcoin/testnet3

# Save output to file
cyberkrill bdk-list-utxos \
  --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --bitcoin-dir ~/libre \
  --output utxos.json
```

#### BDK Features and Limitations

**Key Features:**
- Supports complex descriptors including multisig and miniscript
- Automatically expands multipath descriptors (`<0;1>/*` syntax)
- Derives addresses from scriptPubKey when not provided by Bitcoin Core
- Compatible with BDK 2.0 API

**Important Notes:**
- **Multipath Descriptors**: BDK doesn't natively support `<0;1>/*` syntax. CyberKrill automatically expands these into separate descriptors for external (0) and internal/change (1) addresses
- **Bitcoin Core Integration**: Requires `--bitcoin-dir` to connect to Bitcoin Core RPC. Without it, only shows UTXOs already in the BDK wallet (typically none)
- **Address Derivation**: When using Bitcoin Core's `scantxoutset`, addresses aren't included in the response. CyberKrill automatically derives them from the scriptPubKey

**Common Gotchas:**
1. **Missing UTXOs**: If you don't see expected UTXOs, ensure Bitcoin Core is running and `--bitcoin-dir` points to the correct data directory
2. **Multipath Syntax**: Always use `<0;1>/*` for descriptors that need both receive and change addresses
3. **Network Mismatch**: Ensure the descriptor network matches the `--network` parameter (mainnet by default)

## Configuration

### Hardware Wallet Setup

**System Requirements:**
- USB NFC card reader (e.g., OMNIKEY 5022 CL)
- Coinkite Tapsigner or Satscard device

**Note**: Hardware wallet functionality is included by default. No additional system dependencies like PCSC daemon are required.

**Tapsigner Authentication:**
```bash
# Set 6-digit PIN (found on card back or documentation)
export TAPSIGNER_CVC=123456
```

### Bitcoin Core RPC Setup

**Cookie Authentication (Recommended):**
```bash
# Default Bitcoin directory
cyberkrill list-utxos --descriptor "wpkh([...]xpub...)"

# Custom directory
cyberkrill list-utxos --bitcoin-dir /custom/bitcoin/dir --descriptor "wpkh([...]xpub...)"
```

**Username/Password Authentication:**
Add to your `bitcoin.conf`:
```
rpcuser=myuser
rpcpassword=mypassword
```

Then use:
```bash
cyberkrill list-utxos --rpc-user myuser --rpc-password mypassword --descriptor "wpkh([...]xpub...)"
```

## Amount Input Formats

CyberKrill supports flexible amount input formats across all Bitcoin commands:

### Fee Rates
- **Plain numbers**: `15` (interpreted as sats/vB)
- **Satoshi format**: `15sats`, `0.1sats`, `20.5SATS` (supports fractional satoshis)
- **BTC format**: `0.00000015btc` (converted to sats/vB)

### Output Amounts
- **Plain numbers**: `0.001` (interpreted as BTC)
- **BTC format**: `0.001btc`, `1.5BTC` (case-insensitive)
- **Satoshi format**: `100000sats`, `150000000sat`

### Examples
```bash
# Sub-satoshi fee rates for low-priority transactions
--fee-rate 0.1sats   # 0.1 sat/vB (using millisatoshi precision)
--fee-rate 0.5sats   # 0.5 sat/vB
--fee-rate 1.5sats   # 1.5 sat/vB

# Various output amount formats
--outputs "bc1qaddr:0.01btc"        # 0.01 BTC
--outputs "bc1qaddr:1000000sats"    # 1 million satoshis
--outputs "bc1qaddr:0.001"          # 0.001 BTC (default)

# Absolute fees in move-utxos
--fee 5000sats       # 5000 satoshi absolute fee
--fee 0.00005btc     # 5000 satoshi in BTC format

# Max amount limits with coin selection
--max-amount 0.5btc    # 0.5 BTC maximum
--max-amount 50000000sats  # 50 million satoshis
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
      "amount_sats": 100000,
      "confirmations": 6,
      "spendable": true,
      "solvable": true,
      "safe": true,
      "script_pub_key": "0014...",
      "descriptor": "wpkh([fingerprint/84'/0'/0']xpub...)#checksum"
    }
  ],
  "total_amount_sats": 100000,
  "total_count": 1
}
```

### Bitcoin PSBT Creation

```json
{
  "psbt": "cHNidP8BAHECAAAAAea2/lMA5WyAk9UuMJPJ7wfhNzrhAAAAAA0AAAA=",
  "fee_sats": 21,
  "change_position": 1
}
```

## Using cyberkrill-core as a Library

The `cyberkrill-core` crate can be used as a dependency in other Rust projects:

### Adding as Dependency

```toml
[dependencies]
# For basic functionality (Lightning, Bitcoin RPC, Fedimint)
cyberkrill-core = { git = "https://github.com/douglaz/cyberkrill", default-features = false }

# With smartcard support (Tapsigner, Satscard)
cyberkrill-core = { git = "https://github.com/douglaz/cyberkrill", features = ["smartcards"] }

# With all features (default)
cyberkrill-core = { git = "https://github.com/douglaz/cyberkrill" }
```

### Example Usage

```rust
use cyberkrill_core::{decode_invoice, decode_lnurl, generate_invoice_from_address};
use cyberkrill_core::{BitcoinRpcClient, AmountInput};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Decode Lightning invoice
    let invoice_str = "lnbc1m1...";
    let invoice_data = decode_invoice(invoice_str)?;
    println!("Amount: {} msat", invoice_data.amount_msat);

    // Generate invoice from Lightning address
    let invoice = generate_invoice_from_address(
        "user@domain.com", 
        1000000, 
        Some("Payment")
    ).await?;

    // Bitcoin RPC operations
    let client = BitcoinRpcClient::new_auto(
        "http://127.0.0.1:8332".to_string(),
        Some(std::path::Path::new("~/.bitcoin")),
        None, None,
    )?;
    
    let utxos = client.list_utxos_for_addresses(vec![
        "bc1qtest...".to_string()
    ]).await?;

    Ok(())
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
# - libusb (for hardware wallet support)
```

### Building and Testing

```bash
# Build (hardware wallet support included by default)
cargo build

# Build without hardware wallet support (if needed)
cargo build --no-default-features

# Run tests
cargo test

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

CyberKrill is structured as a Rust workspace with multiple crates:

### cyberkrill (CLI Application)
- **`cyberkrill/src/main.rs`** - CLI interface with argument parsing and command dispatching
- Focuses on user interaction, input/output handling, and file operations
- Uses `cyberkrill-core` for all business logic

### cyberkrill-core (Core Library)  
- **`cyberkrill-core/src/lib.rs`** - Public API exports for external consumption
- **`cyberkrill-core/src/decoder.rs`** - Lightning invoice/LNURL decoding and generation
- **`cyberkrill-core/src/tapsigner.rs`** - Tapsigner hardware wallet operations (requires `smartcards` feature)
- **`cyberkrill-core/src/satscard.rs`** - Satscard hardware wallet operations (requires `smartcards` feature)
- **`cyberkrill-core/src/bitcoin_rpc.rs`** - Bitcoin Core RPC client and transaction building
- Can be used as a dependency in other Rust projects

### fedimint-lite (Fedimint Library)
- **`fedimint-lite/src/lib.rs`** - Standalone Fedimint invite code encoding/decoding
- Fully compatible with fedimint-cli

## License

This project is open source. See the repository for license details.

## Contributing

1. Follow the coding conventions in `CONVENTIONS.md`
2. Add tests for new functionality
3. Run quality checks before submitting
4. Update documentation for new features