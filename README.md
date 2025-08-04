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
- **frozenkrill Wallet Support**: Import and use frozenkrill wallet export files instead of raw descriptors
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

### Bitcoin Operations

CyberKrill provides a unified interface for Bitcoin operations with support for multiple backends:

**Backend Options:**
- **Bitcoin Core RPC** (default): Direct connection to your Bitcoin node
- **Electrum**: Fast and lightweight blockchain queries via Electrum protocol
- **Esplora**: RESTful API for blockchain data (no authentication required)

All Bitcoin commands support these backends through backend selection flags:
- `--bitcoin-dir` - Use Bitcoin Core RPC with cookie authentication (default)
- `--rpc-user/--rpc-password` - Use Bitcoin Core RPC with username/password
- `--electrum <URL>` - Use Electrum server (e.g., `ssl://electrum.blockstream.info:50002`)
- `--esplora <URL>` - Use Esplora API (e.g., `https://blockstream.info/api`)

#### PSBT Creation Commands - Key Differences

| Command | Input Selection | Output Specification | Change Handling | Primary Use Case |
|---------|----------------|---------------------|-----------------|------------------|
| **`create-psbt`** | Manual - you specify exact UTXOs | Manual - you specify all outputs including change | Manual - you calculate and add change output | Full control transactions |
| **`create-funded-psbt`** | Automatic - wallet selects optimal inputs | Manual - you specify recipient outputs only | Automatic - wallet adds change output | Standard send transactions |
| **`move-utxos`** | Manual - you specify UTXOs to consolidate | Automatic - single output (total - fee) | N/A - all funds go to destination | UTXO consolidation |

**Quick Guide:**
- **`create-psbt`**: Use when you need complete control over every aspect of the transaction
- **`create-funded-psbt`**: Use for typical "send payment" scenarios where you want the wallet to handle complexity
- **`move-utxos`**: Use specifically for consolidating UTXOs or moving all funds from specific inputs

#### List UTXOs

```bash
# Using Bitcoin Core RPC (default)
cyberkrill list-utxos --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Using Electrum backend
cyberkrill list-utxos --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --electrum ssl://electrum.blockstream.info:50002

# Using Esplora backend
cyberkrill list-utxos --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" \
  --esplora https://blockstream.info/api

# Using specific addresses with Bitcoin Core
cyberkrill list-utxos --addresses "bc1qtest1,bc1qtest2"

# Using frozenkrill wallet export file
cyberkrill list-utxos --wallet-file mywallet_pub.json

# Custom Bitcoin directory
cyberkrill list-utxos --bitcoin-dir /path/to/.bitcoin --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Username/password authentication
cyberkrill list-utxos --rpc-user myuser --rpc-password mypass --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"
```

#### Create PSBT (Manual Transaction Building)

**When to use**: When you need complete control over every aspect of the transaction - which UTXOs to spend, exact output amounts, and manual change calculation. Perfect for advanced use cases like specific UTXO selection for privacy or when implementing custom transaction logic.

```bash
# Create PSBT with manual inputs/outputs using Bitcoin Core (default)
cyberkrill create-psbt \
  --inputs "txid1:0" --inputs "txid2:1" \
  --outputs "bc1qaddr1:0.001,bc1qaddr2:0.002" \
  --fee-rate 10.5sats

# Using Electrum backend
cyberkrill create-psbt \
  --inputs "txid1:0" --inputs "txid2:1" \
  --outputs "bc1qaddr:0.001" \
  --fee-rate 15sats \
  --electrum ssl://electrum.blockstream.info:50002

# Using Esplora backend
cyberkrill create-psbt \
  --inputs "txid1:0" --inputs "txid2:1" \
  --outputs "bc1qaddr:0.001" \
  --fee-rate 20sats \
  --esplora https://blockstream.info/api

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

#### Create Funded PSBT (Automatic Input Selection & Change)

**When to use**: For standard "send payment" transactions where you want Bitcoin Core to handle the complexity. The wallet automatically selects optimal inputs, calculates fees, and adds a change output. This is the recommended approach for most payment scenarios.

```bash
# Let wallet select inputs and handle change automatically
cyberkrill create-funded-psbt \
  --outputs "bc1qaddr1:0.001,bc1qaddr2:0.002" \
  --conf-target 6 \
  --estimate-mode CONSERVATIVE

# Using Electrum backend with automatic input selection
cyberkrill create-funded-psbt \
  --outputs "bc1qaddr:0.01btc" \
  --fee-rate 0.1sats \
  --electrum ssl://electrum.blockstream.info:50002

# Using Esplora backend
cyberkrill create-funded-psbt \
  --outputs "bc1qaddr:0.01btc" \
  --fee-rate 1.5sats \
  --esplora https://blockstream.info/api

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

**When to use**: Specifically for UTXO consolidation or moving all funds from selected inputs to a single destination. Unlike the other commands, you don't specify output amounts - the command automatically sends (total input value - fee) to the destination address. Perfect for cleaning up fragmented UTXOs or emptying specific addresses.

```bash
# Consolidate specific UTXOs to single address using Bitcoin Core (default)
cyberkrill move-utxos \
  --inputs "txid1:0" --inputs "txid2:1" --inputs "txid3:0" \
  --destination "bc1qconsolidated_address" \
  --fee-rate 15sats

# Using Electrum backend
cyberkrill move-utxos \
  --inputs "txid1:0" --inputs "txid2:1" \
  --destination "bc1qmy_address" \
  --fee-rate 20sats \
  --electrum ssl://electrum.blockstream.info:50002

# Using Esplora backend
cyberkrill move-utxos \
  --inputs "txid1:0" --inputs "txid2:1" \
  --destination "bc1qmy_address" \
  --fee-rate 25sats \
  --esplora https://blockstream.info/api

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

### Backend Details

**Bitcoin Core RPC (Default)**
- Uses your local Bitcoin node for maximum privacy and security
- Supports both cookie and username/password authentication
- Requires Bitcoin Core to be running and accessible
- Best for users running their own node

**Electrum Backend**
- Fast and lightweight - doesn't require a full node
- Uses the Electrum protocol for efficient blockchain queries
- Example servers:
  - Mainnet: `ssl://electrum.blockstream.info:50002`
  - Testnet: `ssl://electrum.blockstream.info:60002`
- Some privacy trade-offs as queries go to external servers

**Esplora Backend**
- RESTful API for blockchain data
- No authentication required
- Example servers:
  - Mainnet: `https://blockstream.info/api`
  - Testnet: `https://blockstream.info/testnet/api`
- Good for quick queries without running infrastructure

**Important Notes:**
- All backends support the same functionality
- Multipath descriptors (`<0;1>/*`) are automatically expanded
- Address derivation is handled automatically when needed
- Network must match between descriptor and backend

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

### frozenkrill Wallet Integration

CyberKrill supports frozenkrill wallet export files as an alternative to raw descriptors. This provides a more user-friendly way to work with wallets that have pre-generated addresses and metadata.

**Using frozenkrill wallet files:**
```bash
# List UTXOs from a frozenkrill wallet export
cyberkrill list-utxos --wallet-file mywallet_pub.json

# Create PSBT with wallet file (future enhancement)
cyberkrill create-psbt --wallet-file mywallet_pub.json --outputs "bc1qaddr:0.001btc"

# Move UTXOs using wallet file (future enhancement)
cyberkrill move-utxos --wallet-file mywallet_pub.json --destination "bc1qconsolidated"
```

**Supported wallet types:**
- Single-signature wallets (singlesig)
- Multi-signature wallets (2-of-3, 3-of-5, etc.)

**Benefits:**
- No need to manually specify descriptors
- Automatically includes both receiving and change addresses
- Pre-validated addresses with derivation paths
- Network and script type metadata included

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