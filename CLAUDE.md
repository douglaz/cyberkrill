# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

**Important**: This file should be kept locally and not committed to the repository. It contains development instructions specific to Claude Code.

## Project Overview

CyberKrill is a Rust workspace containing multiple crates:
- **cyberkrill**: CLI application that provides command-line interface for all functionality. Focuses purely on argument parsing, user interaction, and output formatting.
- **cyberkrill-core**: Core library containing all business logic for Bitcoin, Lightning, Fedimint, and hardware wallet operations. Can be used by external projects as a dependency.
- **fedimint-lite**: Standalone library for Fedimint invite code encoding/decoding that can be used by external projects

## Architecture

The workspace follows a modular structure with clear separation of concerns:

### cyberkrill crate (CLI Application):
- **`cyberkrill/src/main.rs`**: Entry point with CLI argument parsing using `clap`. Defines command structure and dispatches to cyberkrill-core functions. Handles input/output, file operations, and user interaction.

### cyberkrill-core crate (Core Library):
- **`cyberkrill-core/src/lib.rs`**: Public API exports for external consumption
- **`cyberkrill-core/src/decoder.rs`**: Core decoding logic for Lightning invoices (BOLT11) and LNURL strings. Contains serializable output structures and conversion implementations
- **`cyberkrill-core/src/bitcoin_rpc.rs`**: Bitcoin Core RPC client implementation for UTXO management and PSBT operations
- **`cyberkrill-core/src/tapsigner.rs`**: Tapsigner hardware wallet integration (optional, requires `smartcards` feature)
- **`cyberkrill-core/src/satscard.rs`**: Satscard hardware wallet integration (optional, requires `smartcards` feature)

### fedimint-lite crate (Fedimint Library):
- **`fedimint-lite/src/lib.rs`**: Standalone library for Fedimint invite code encoding/decoding. Handles bech32m format with full fedimint-cli compatibility

### Key Components

1. **CLI Structure**: Hierarchical command structure with six main commands:
   - `cyberkrill decode-invoice|decode-lnurl|decode-fedimint-invite` - Decoding functionality
   - `cyberkrill generate-invoice` - Invoice generation from Lightning addresses
   - `cyberkrill fedimint-config` - Fedimint federation configuration retrieval
   - `cyberkrill tapsigner address|init` - Tapsigner hardware wallet operations (requires `smartcards` feature)
   - `cyberkrill satscard address` - Satscard address generation (requires `smartcards` feature)
   - `cyberkrill bitcoin list-utxos|create-psbt|create-funded-psbt|move-utxos` - Bitcoin Core RPC operations for UTXO management and PSBT creation
2. **Decoding Logic**: 
   - Lightning invoices: Uses `lightning-invoice` crate to parse BOLT11 invoices and extract payment details, routing hints, and features
   - LNURL: Decodes bech32-encoded URLs and parses query parameters
   - Fedimint invite codes: Supports bech32m (fed1...) encoded invite codes, extracting federation ID, guardian endpoints, and optional API secrets. Includes encoding functionality to convert JSON back to invite codes with fedimint-cli compatibility options
3. **Fedimint Integration**: 
   - Federation config retrieval: Fetches complete federation configuration from guardian endpoints using invite codes
   - Multi-guardian support: Attempts to fetch config from multiple guardians for resilience
   - Federation ID validation: Validates retrieved config against expected federation ID from invite code
4. **Invoice Generation**: Implements LNURL-pay protocol to generate invoices from Lightning addresses by:
   - Parsing Lightning address format (user@domain.com)
   - Making HTTP requests to `.well-known/lnurlp/<user>` endpoints
   - Validating amount ranges and comment length limits
   - Requesting invoice generation via callback URL
5. **Hardware Wallet Integration**: 
   - **Tapsigner**: Full support with rust-cktap library integration, PCSC communication, BIP-32 derivation path parsing, CVC authentication, P2WPKH address generation, and one-time initialization
   - **Satscard**: Address generation from card slots with fixed m/0 derivation path, slot validation, and usage tracking
6. **Bitcoin Core RPC Integration**:
   - **UTXO Management**: List UTXOs for specific addresses or output descriptors with confirmation counts
   - **PSBT Creation**: Three approaches for creating Partially Signed Bitcoin Transactions:
     - **Manual PSBT**: `create-psbt` with specific inputs/outputs and fee calculation for P2WPKH transactions
     - **Wallet-Funded PSBT**: `create-funded-psbt` with automatic input selection, change handling, and fee estimation
     - **UTXO Consolidation**: `move-utxos` with specific inputs consolidated to a single destination and precise fee control
   - **Coin Selection**: `--max-amount` parameter for move-utxos performs smart coin selection by selecting UTXOs (largest first) until reaching the specified maximum amount. Supports flexible amount formats: `0.5` or `0.5btc` for BTC, `50000000sats` or `50000000sat` for satoshis
   - **Fee Control**: Support for fee rates (sat/vB), confirmation targets, and estimation modes (ECONOMICAL/CONSERVATIVE)
   - **Output Flexibility**: Save JSON responses, raw PSBT data, or both to separate files with `--output` and `--psbt-output`
   - **Authentication**: Supports both cookie-based and username/password authentication
   - **Descriptor Support**: Handle complex descriptors including multisig with `<0;1>/*` syntax expansion
7. **Amount Input Format**: Flexible amount parsing supports multiple formats:
   - **Plain numbers**: `0.5` (interpreted as BTC)
   - **BTC format**: `0.5btc` or `1.5BTC` (case-insensitive)
   - **Satoshi format**: `50000000sats`, `100000sat`, or `123SATS` (case-insensitive)
   - **Validation**: Negative amounts are rejected, satoshi amounts must be integers
8. **Output Format**: All outputs are JSON with detailed structured data including payment hashes, routing information, and metadata

## Development Commands

### Building and Testing
```bash
# Build the project (without smartcard support)
cargo build

# Build with smartcard support (Tapsigner/Satscard)
cargo build --features smartcards

# Build for release
cargo build --release

# Build for release with smartcard support
cargo build --release --features smartcards

# Run tests
cargo test

# Run tests with smartcard features
cargo test --features smartcards

# Run tests with output
cargo test -- --nocapture

# Run specific test
cargo test test_decode_invoice
```

### Running the Application
```bash
# Build and run
cargo run -- decode-invoice <invoice_string>
cargo run -- decode lnurl <lnurl_string>
cargo run -- decode fedimint-invite <invite_code_string>

# With file I/O
cargo run -- decode-invoice <input> -o output.json
cargo run -- decode fedimint-invite <input> -o output.json
echo "lnbc..." | cargo run -- decode-invoice
echo "fed1abc..." | cargo run -- decode fedimint-invite

# Generate invoice from Lightning address
cargo run -- generate-invoice user@domain.com 1000000 --comment "Payment for service"
cargo run -- generate-invoice user@domain.com 1000000 -o invoice.json

# Fetch Fedimint federation configuration
cargo run -- fedimint-config <invite_code_string>
cargo run -- fedimint-config <invite_code_string> -o config.json

# Encode Fedimint invite codes from JSON
cargo run -- encode-fedimint-invite invite.json
cargo run -- encode-fedimint-invite invite.json -o invite_code.txt
echo '{"federation_id":"...", "guardians":[...], "encoding_format":"bech32m"}' | cargo run -- encode-fedimint-invite -

# Encode with fedimint-cli compatibility (skips API secrets)
cargo run -- encode-fedimint-invite --skip-api-secret invite.json
echo '{"federation_id":"...", "guardians":[...], "api_secret":"...", "encoding_format":"bech32m"}' | cargo run -- encode-fedimint-invite --skip-api-secret -

# Initialize Tapsigner (one-time setup for new cards, requires smartcards feature)
cargo run --features smartcards -- tapsigner init
cargo run --features smartcards -- tapsigner init --chain-code "0123456789abcdef..." -o init.json

# Generate Bitcoin address from Tapsigner (requires hardware setup and smartcards feature)
cargo run --features smartcards -- tapsigner address --path "m/84'/0'/0'/0/0"
cargo run --features smartcards -- tapsigner address --path "m/84'/0'/0'/0/1" -o address.json

# Generate Bitcoin address from Satscard (requires hardware setup and smartcards feature)
cargo run --features smartcards -- satscard address
cargo run --features smartcards -- satscard address --slot 1 -o address.json

# List UTXOs with flexible backend support

# Using Bitcoin Core RPC (default)
cargo run -- list-utxos --bitcoin-dir ~/.bitcoin --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Using Electrum backend via BDK
cargo run -- list-utxos --electrum ssl://electrum.blockstream.info:50002 --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Using Esplora backend via BDK
cargo run -- list-utxos --esplora https://blockstream.info/api --descriptor "wpkh([fingerprint/84'/0'/0']xpub..."

# Using username/password authentication
cargo run -- bitcoin list-utxos --rpc-user myuser --rpc-password mypass --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# For specific addresses (legacy approach)  
cargo run -- bitcoin list-utxos --addresses "bc1qtest1,bc1qtest2"
cargo run -- bitcoin list-utxos --bitcoin-dir /custom/bitcoin/dir --addresses "bc1qtest1,bc1qtest2"

# Custom RPC URL with cookie auth
cargo run -- bitcoin list-utxos --rpc-url http://192.168.1.100:8332 --bitcoin-dir /home/user/.bitcoin --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)"

# Save output to file
cargo run -- bitcoin list-utxos --bitcoin-dir ~/.bitcoin --addresses "bc1qtest1" -o utxos.json

# Create PSBT for spending UTXOs

## Manual PSBT Creation (create-psbt)
# Supports both specific UTXO inputs and output descriptors with any backend

# Using Bitcoin Core RPC (default)
cargo run -- create-psbt --bitcoin-dir ~/.bitcoin --inputs "txid1:vout1" --inputs "txid2:vout2" --outputs "address:amount_btc,address:amount_btc"

# Using Electrum backend via BDK
cargo run -- create-psbt --electrum ssl://electrum.blockstream.info:50002 --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" --inputs "txid:0" --outputs "bc1qaddr:0.001" --fee-rate 10

# Using Esplora backend via BDK
cargo run -- create-psbt --esplora https://blockstream.info/api --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" --inputs "txid:0" --outputs "bc1qaddr:0.001" --fee-rate 10

# Using output descriptors to automatically find and include all UTXOs
cargo run -- create-psbt --bitcoin-dir ~/.bitcoin --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" --outputs "bc1qaddr:0.001" --fee-rate 20

# Mixed inputs: specific UTXOs and descriptors
cargo run -- create-psbt --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" --inputs "txid2:1" --outputs "bc1qaddr:0.001" --fee-rate 15

# With fee rate calculation (estimates P2WPKH transaction size)
cargo run -- bitcoin create-psbt --bitcoin-dir ~/.bitcoin --inputs "txid:0" --outputs "bc1qaddr:0.001" --fee-rate 20

# Multiple inputs and outputs with fee calculation
cargo run -- bitcoin create-psbt --bitcoin-dir ~/.bitcoin --inputs "tx1:0" --inputs "tx2:1" --outputs "bc1qaddr1:0.001,bc1qaddr2:0.002" --fee-rate 15

# Save PSBT directly to file (plus JSON response to stdout)
cargo run -- bitcoin create-psbt --bitcoin-dir ~/.bitcoin --inputs "txid:0" --outputs "bc1qaddr:0.001" --fee-rate 20 --psbt-output transaction.psbt

# Save both JSON response and PSBT to separate files
cargo run -- bitcoin create-psbt --bitcoin-dir ~/.bitcoin --inputs "txid:0" --outputs "bc1qaddr:0.001" --fee-rate 20 -o response.json --psbt-output transaction.psbt

## Wallet-Funded PSBT Creation (create-funded-psbt) 
# Automatic input selection and change handling with precise fee control
# Supports all backends (Bitcoin Core RPC, Electrum, Esplora)

# Automatic input selection with confirmation target
cargo run -- bitcoin create-funded-psbt --bitcoin-dir ~/.bitcoin --outputs "bc1qaddr:0.001" --conf-target 6 --estimate-mode "ECONOMICAL"

# Automatic with specific fee rate (overrides conf-target)
cargo run -- bitcoin create-funded-psbt --bitcoin-dir ~/.bitcoin --outputs "bc1qaddr:0.001" --fee-rate 25

# Partial input specification with specific UTXOs (wallet adds more inputs if needed)
cargo run -- bitcoin create-funded-psbt --bitcoin-dir ~/.bitcoin --inputs "txid:0" --outputs "bc1qaddr:0.001" --conf-target 3

# With Electrum backend
cargo run -- create-funded-psbt --electrum ssl://electrum.blockstream.info:50002 --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" --outputs "bc1qaddr:0.001" --fee-rate 20

# With Esplora backend
cargo run -- create-funded-psbt --esplora https://blockstream.info/api --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" --outputs "bc1qaddr:0.001" --fee-rate 20

# Save JSON response to file
cargo run -- bitcoin create-funded-psbt --bitcoin-dir ~/.bitcoin --outputs "bc1qaddr:0.001" --fee-rate 20 -o funded.json

# Save PSBT directly to file (plus JSON response to stdout)
cargo run -- bitcoin create-funded-psbt --bitcoin-dir ~/.bitcoin --outputs "bc1qaddr:0.001" --fee-rate 20 --psbt-output funded.psbt

# Save both JSON response and PSBT to separate files
cargo run -- bitcoin create-funded-psbt --bitcoin-dir ~/.bitcoin --outputs "bc1qaddr:0.001" --fee-rate 20 -o funded.json --psbt-output funded.psbt

# Move/Consolidate UTXOs (move-utxos)
# Consolidate multiple specific UTXOs into a single destination with precise fee control
# Supports both specific UTXOs and output descriptors across all backends
# Fee control: Use either --fee-rate (sats/vB) or --fee (absolute satoshis)

# Basic consolidation with specific UTXOs (cookie authentication recommended)
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --inputs "txid3:0" --destination "bc1qconsolidated_address" --fee-rate 15

# Consolidate all UTXOs from a descriptor
cargo run -- move-utxos --bitcoin-dir ~/.bitcoin --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" --destination "bc1qconsolidated_address" --fee-rate 15

# Using Electrum backend
cargo run -- move-utxos --electrum ssl://electrum.blockstream.info:50002 --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" --inputs "txid:0" --destination "bc1qaddr" --fee-rate 10

# Using Esplora backend
cargo run -- move-utxos --esplora https://blockstream.info/api --descriptor "wpkh([fingerprint/84'/0'/0']xpub...)" --inputs "txid:0" --destination "bc1qaddr" --fee-rate 10

# Mixed consolidation: specific UTXOs and descriptors (NEW!)
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" --inputs "txid2:1" --destination "bc1qconsolidated_address" --fee-rate 20

# With username/password authentication
cargo run -- bitcoin move-utxos --rpc-user myuser --rpc-password mypass --inputs "txid1:0" --inputs "txid2:1" --destination "bc1qmy_address" --fee-rate 20

# Custom RPC URL with cookie auth
cargo run -- bitcoin move-utxos --rpc-url http://192.168.1.100:8332 --bitcoin-dir /home/user/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --inputs "txid3:2" --destination "bc1qmy_address" --fee-rate 25

# Save JSON response to file
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --destination "bc1qmy_address" --fee-rate 15 -o consolidation.json

# Save PSBT directly to file (plus JSON response to stdout)
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --inputs "txid3:0" --destination "bc1qmy_address" --fee-rate 20 --psbt-output consolidation.psbt

# Save both JSON response and PSBT to separate files
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --destination "bc1qmy_address" --fee-rate 15 -o consolidation.json --psbt-output consolidation.psbt

# Use absolute fee amount in satoshis instead of fee rate (NEW!)
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --destination "bc1qmy_address" --fee 5000

# Absolute fee with descriptor inputs (NEW!)
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" --destination "bc1qconsolidated_address" --fee 10000

# Absolute fee with mixed authentication and file output (NEW!)
cargo run -- bitcoin move-utxos --rpc-user myuser --rpc-password mypass --inputs "txid1:0" --inputs "txid2:1" --destination "bc1qmy_address" --fee 7500 -o consolidation.json --psbt-output consolidation.psbt

# Use --max-amount to limit the total amount moved (NEW!)
# Selects UTXOs from largest to smallest until reaching the max amount
# Supports multiple formats: BTC (0.5 or 0.5btc), satoshis (50000000sats or 50000000sat)
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --inputs "txid3:0" --destination "bc1qmy_address" --fee-rate 15 --max-amount 0.5

# Max amount with BTC suffix
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" --destination "bc1qmy_address" --fee-rate 20 --max-amount 1.0btc

# Max amount in satoshis
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --destination "bc1qmy_address" --fee-rate 15 --max-amount 25000000sats

# Mix different amount formats (NEW!)
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "wpkh([fingerprint/84'/0'/0']xpub...)" --destination "bc1qmy_address" --fee-rate 15 --max-amount 0.25btc

# Small amounts in satoshis with absolute fee (NEW!)
cargo run -- bitcoin move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid1:0" --inputs "txid2:1" --inputs "txid3:2" --destination "bc1qmy_address" --fee 5000 --max-amount 100000sat
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

    // Decode LNURL
    let lnurl_str = "LNURL1DP68GURN8GHJ7...";
    let lnurl_data = decode_lnurl(lnurl_str)?;
    println!("URL: {}", lnurl_data.url);

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
        None,
        None,
    )?;
    
    let utxos = client.list_utxos_for_addresses(vec![
        "bc1qtest...".to_string()
    ]).await?;

    Ok(())
}
```

### Smartcard Integration (Optional)

```rust
#[cfg(feature = "smartcards")]
use cyberkrill_core::{generate_tapsigner_address, generate_satscard_address};

#[cfg(feature = "smartcards")]
async fn hardware_wallet_example() -> anyhow::Result<()> {
    // Generate Tapsigner address
    let tapsigner_addr = generate_tapsigner_address("m/84'/0'/0'/0/0").await?;
    println!("Tapsigner address: {}", tapsigner_addr.address);

    // Generate Satscard address
    let satscard_addr = generate_satscard_address(Some(0)).await?;
    println!("Satscard address: {}", satscard_addr.address);

    Ok(())
}
```

### Available Features

- **default**: Includes `smartcards` feature
- **smartcards**: Enables Tapsigner and Satscard hardware wallet support (requires USB dependencies)

### Nix Development
The project uses Nix flakes for reproducible builds:
```bash
# Run from repository (without smartcard support)
nix run .

# Run with smartcard support (requires building from source with feature)
nix run . -- --help  # Note: Smartcard support not included in default package

# Run from GitHub (now working)
nix run 'git+https://github.com/douglaz/cyberkrill.git'

# Enter development shell (includes smartcard dependencies)
nix develop

# IMPORTANT: Always use 'nix develop -c' to run commands to ensure proper environment
# This is especially important for SQLite and musl static linking
nix develop -c cargo build
nix develop -c cargo test
nix develop -c cargo build --features smartcards

# Build with BDK support (includes SQLite)
nix develop -c cargo build

# Run tests
nix develop -c cargo test
```

**Note**: The development environment includes static SQLite libraries for musl builds. When running any cargo commands, always use `nix develop -c <command>` to ensure all environment variables (SQLITE3_LIB_DIR, SQLITE3_STATIC, etc.) are properly set.

## Dependencies

- **Core**: `clap` for CLI, `anyhow` for error handling, `serde`/`serde_json` for serialization
- **HTTP**: `reqwest` with `rustls-tls` for HTTPS requests, `tokio` for async runtime
- **Bitcoin/Lightning**: `lightning-invoice` for BOLT11 parsing, `bech32` for encoding, `hex` for hex encoding
- **Hardware Wallet** (optional, requires `smartcards` feature): `rust-cktap` for Tapsigner communication, `pcsc` for smart card interface, `bitcoin` for address generation, `secp256k1` for cryptography, `sha2` for entropy generation
- **Utilities**: `url` for URL parsing
- **BDK (Bitcoin Development Kit)**: `bdk_wallet` v2.0 for wallet functionality, `bdk_electrum` for Electrum backend, `bdk_esplora` for Esplora backend

## BDK Integration Notes

### BDK 2.0 API Changes

BDK 2.0 introduced significant API changes that affect integration:

1. **Wallet Creation**:
   - Old: `Wallet::new(descriptor, change_descriptor, network, database)?`
   - New: `Wallet::create(descriptor, change_descriptor).network(network).create_wallet()?`
   - For single descriptor: `Wallet::create_single(descriptor).network(network).create_wallet_no_persist()?`

2. **ChainPosition Structure**:
   - Changed from tuple variants to struct variants
   - Old: `ChainPosition::Confirmed((height, timestamp))`
   - New: `ChainPosition::Confirmed { anchor: ConfirmationBlockTime { block_id, ... }, ... }`

3. **Script and Address APIs**:
   - `Script::from_bytes()` → `ScriptBuf::from(bytes)`
   - `script.to_address(network)` → `Address::from_script(&script, network)`

### Multipath Descriptor Handling

BDK doesn't natively support multipath descriptors (e.g., `<0;1>/*`). CyberKrill implements custom expansion:

```rust
// Example: "wpkh(xpub.../<0;1>/*)" becomes:
// - "wpkh(xpub.../0/*)" for external addresses
// - "wpkh(xpub.../1/*)" for internal (change) addresses
```

**Implementation Details:**
- The `expand_multipath_descriptor()` function handles `<0;1>` syntax
- Each expanded descriptor is processed separately
- Results are combined and deduplicated

### Bitcoin Core RPC Integration

When integrating BDK with Bitcoin Core:

1. **Address Derivation Issue**:
   - `scantxoutset` RPC returns UTXOs without addresses
   - Addresses must be derived from scriptPubKey hex
   - Pattern: decode hex → create ScriptBuf → derive Address

2. **Backend Selection**:
   - `bdk_bitcoind_rpc` v0.20 has incompatible API changes
   - Fallback: Use existing `BitcoinRpcClient` and convert results
   - Future: Consider updating when `bdk_bitcoind_rpc` stabilizes

3. **UTXO Conversion**:
   - Bitcoin Core returns different UTXO structure than BDK expects
   - Manual conversion required for fields like confirmations, amounts
   - Missing data (keychain type, derivation index) filled with defaults

### Common Development Pitfalls

1. **Missing UTXOs**: If filtering by address presence, UTXOs from `scantxoutset` will be skipped
2. **Network Mismatch**: Ensure descriptor network matches BDK wallet network
3. **Lifetime Issues**: Descriptor strings often need `.to_string()` for ownership
4. **Feature Flags**: Use `default-features = false` with `use-rustls` to avoid OpenSSL

## Fedimint-cli Compatibility

CyberKrill's fedimint implementation is designed to be compatible with fedimint-cli:

### ✅ **Fully Compatible Features:**
- **Single and multi-guardian invites**: bech32m format (fed1...) works perfectly with fedimint-cli
- **Federation ID and guardian URLs**: Exact same output as fedimint-cli for basic invite decoding
- **Round-trip encoding/decoding**: CyberKrill can decode real fedimint invite codes and encode them back to identical format

### ⚠️ **Enhanced Features (CyberKrill advantages):**
- **Complete guardian information**: CyberKrill shows ALL guardians with peer IDs, while fedimint-cli only shows first guardian
- **Full invite structure**: CyberKrill preserves and displays complete invite code structure including encoding format

### 🔧 **Compatibility Options:**
- **API Secret support**: CyberKrill supports API secrets (fedimint feature), but fedimint-cli may not decode them correctly
- **`--skip-api-secret` flag**: Use this flag when encoding to ensure fedimint-cli compatibility
- **Format**: Only bech32m format (fed1...) is supported for full compatibility

### **Usage for Maximum Compatibility:**
```bash
# Always compatible with fedimint-cli
cargo run -- encode-fedimint-invite --skip-api-secret invite.json

# Use bech32m format for fedimint-cli compatibility
echo '{"federation_id":"...", "guardians":[...], "encoding_format":"bech32m"}' | cargo run -- encode-fedimint-invite --skip-api-secret -
```

## Testing

Tests are located in `src/decoder.rs` with comprehensive coverage of:
- Valid Lightning invoice decoding with verification of all fields
- Valid LNURL decoding with URL parsing validation  
- Fedimint invite code decoding for both bech32m and base32 formats
- Lightning address parsing and validation
- BIP-32 derivation path parsing for Tapsigner
- Error handling for invalid inputs

Test data includes real Lightning invoice and LNURL examples with expected outputs.

### Tapsigner Hardware Setup

The Tapsigner functionality is fully implemented with hardware integration and requires the `smartcards` feature to be enabled:

1. **Feature Requirement**:
   ```bash
   # Build with smartcards feature enabled
   cargo build --features smartcards
   
   # Or run directly with the feature
   cargo run --features smartcards -- tapsigner --help
   ```

2. **System Dependencies** (via NixOS flake):
   ```bash
   # Dependencies are automatically managed by flake.nix
   # Includes: pkg-config, pcsclite
   nix develop
   ```

3. **Hardware Requirements**:
   - USB NFC card reader (e.g., OMNIKEY 5022 CL)
   - Coinkite Tapsigner device
   - 6-digit PIN code for authentication (found on card or documentation)

4. **Usage**:
   ```bash
   # Set PIN authentication (6-digit PIN from your card)
   export TAPSIGNER_CVC=123456
   
   # Generate address using default BIP-84 path
   cargo run --features smartcards -- tapsigner address
   
   # Generate address with custom derivation path
   cargo run --features smartcards -- tapsigner address --path "m/84'/0'/0'/0/5"
   
   # Save to file
   cargo run --features smartcards -- tapsigner address -o address.json
   ```

The implementation uses rust-cktap library for PCSC communication and generates P2WPKH (native segwit) addresses compatible with BIP-84 derivation paths.

#### Tapsigner Initialization

**IMPORTANT**: Tapsigner cards ship blank from the factory and must be initialized before first use. This is a **one-time operation** that cannot be undone.

**Initialization Process**:
```bash
# Initialize with auto-generated entropy (recommended)
export TAPSIGNER_CVC=123456  # Your card's 6-digit PIN
cargo run --features smartcards -- tapsigner init

# Initialize with custom entropy (advanced users)
cargo run --features smartcards -- tapsigner init --chain-code "your64characterhexstringhere..."

# Save initialization details to file
cargo run --features smartcards -- tapsigner init -o init_details.json
```

**What happens during initialization**:
1. **Entropy Generation**: Creates 32 bytes of cryptographically secure randomness using double SHA-256
2. **Master Key Creation**: Card's hardware combines your entropy with its internal randomness
3. **Path Setup**: Sets default BIP-84 derivation path (`m/84'/0'/0'`) for native segwit addresses
4. **Permanent State**: Card transitions from uninitialized to ready-for-use (irreversible)

**Security Notes**:
- Initialization uses the card's 6-digit CVC (printed on card back)
- Generated entropy is cryptographically secure (128 bytes → double SHA-256)
- Custom chain codes must be exactly 64 hex characters (32 bytes)
- **Backup your card** after initialization for recovery purposes

### Satscard Hardware Setup

The Satscard functionality is fully implemented for address generation from card slots and requires the `smartcards` feature to be enabled:

1. **Feature Requirement**:
   ```bash
   # Build with smartcards feature enabled
   cargo build --features smartcards
   
   # Or run directly with the feature
   cargo run --features smartcards -- satscard --help
   ```

2. **System Dependencies** (via NixOS flake):
   ```bash
   # Dependencies are automatically managed by flake.nix
   # Includes: pkg-config, pcsclite, rand (for nonce generation)
   nix develop
   ```

3. **Hardware Requirements**:
   - USB NFC card reader (e.g., OMNIKEY 5022 CL)
   - Coinkite Satscard device
   - No PIN required for address generation (unlike Tapsigner)

4. **Usage**:
   ```bash
   # Generate address from current active slot
   cargo run --features smartcards -- satscard address
   
   # Generate address from specific slot (0-9)
   cargo run --features smartcards -- satscard address --slot 2
   
   # Save to file
   cargo run --features smartcards -- satscard address -o satscard_address.json
   ```

**Key Differences from Tapsigner**:
- **Fixed Derivation**: Satscard always uses `m/0` derivation path (not configurable)
- **Slot-Based**: Satscard has 10 slots (0-9), each with independent keys
- **No Authentication**: No CVC/PIN required for address generation
- **Usage Tracking**: Shows if a slot has been used (based on slot position vs current active slot)

## Development Workflow

### Git Workflow and Branch Management

This project follows a standard Git workflow with feature branches and pull requests:

#### 1. **Creating a New Feature Branch**
```bash
# Always start from the latest master
git checkout master
git pull origin master

# Create a new feature branch with descriptive name
git checkout -b feature/add-lightning-payments
git checkout -b fix/bitcoin-rpc-timeout
git checkout -b refactor/split-core-library
git checkout -b docs/update-readme
```

#### 2. **Branch Naming Conventions**
Use descriptive prefixes for different types of changes:
- `feature/` - New features or major enhancements
- `fix/` - Bug fixes
- `refactor/` - Code refactoring without functional changes
- `docs/` - Documentation updates
- `test/` - Test additions or improvements
- `chore/` - Maintenance tasks, dependency updates

#### 3. **Development Cycle**
```bash
# Make your changes following the development checklist below
# IMPORTANT: Only add the specific files you changed, not all files
# First, check what files were changed:
git status

# Add ONLY the changed files individually:
git add path/to/changed/file1.rs
git add path/to/changed/file2.toml
# etc.

# NEVER use git add -A or git add . as this may include unintended files
# Commit with descriptive messages
git commit -m "feat: add lightning payment functionality

- Implement BOLT11 invoice parsing
- Add payment routing logic
- Include comprehensive error handling
- Add unit tests with 95% coverage"

# Push feature branch to remote
git push -u origin feature/add-lightning-payments
```

#### **Commit Message Guidelines**
Follow conventional commit format for clear, consistent history:

**IMPORTANT**: Never include Claude or AI-related references in commit messages. These are not appropriate for version control history.

```bash
# Format: <type>(<scope>): <subject>
#
# <body>
#
# <footer>

# Examples:
git commit -m "feat(decoder): add lightning payment decoding"
git commit -m "fix(bitcoin-rpc): handle connection timeout gracefully"
git commit -m "refactor(core): extract common validation logic"
git commit -m "docs(readme): update installation instructions"
git commit -m "test(tapsigner): add integration tests for address generation"
git commit -m "chore(deps): update bitcoin crate to v0.32"
```

**Commit Types:**
- `feat`: New features
- `fix`: Bug fixes
- `refactor`: Code refactoring
- `docs`: Documentation changes
- `test`: Test additions/modifications
- `chore`: Maintenance tasks
- `perf`: Performance improvements
- `style`: Code style/formatting changes

**Scopes (optional):**
- `decoder`: Lightning/LNURL decoding
- `bitcoin-rpc`: Bitcoin Core RPC functionality
- `tapsigner`: Tapsigner hardware wallet
- `satscard`: Satscard hardware wallet
- `fedimint`: Fedimint integration
- `core`: Core library functionality
- `cli`: CLI interface
- `deps`: Dependencies

#### 4. **Creating Pull Requests**

**IMPORTANT: Before creating a PR, you MUST:**
1. Review code against `CONVENTIONS.md` guidelines
2. Run the complete quality check sequence:
   ```bash
   # Run all quality checks in order
   nix develop -c cargo fmt        # Format code
   nix develop -c cargo clippy     # Check for issues
   nix develop -c cargo test       # Run all tests
   
   # Or run the complete sequence from CONVENTIONS.md:
   nix develop -c bash -c "cargo clippy --fix --allow-dirty && cargo fmt && cargo test && cargo clippy && cargo fmt --check"
   ```
3. Fix any issues found before proceeding

Only after all checks pass, create the PR:
```bash
# Create PR using GitHub CLI (recommended)
nix develop -c gh pr create --title "feat: add lightning payment functionality" --body "$(cat <<'EOF'
## Summary
Brief description of what this PR does

## Changes
- List of specific changes made
- Technical details if needed

## Test Plan
- [ ] Reviewed code against CONVENTIONS.md
- [ ] cargo fmt - code formatted
- [ ] cargo clippy - no warnings
- [ ] cargo test - all tests pass
- [ ] Manual testing completed

## Breaking Changes
List any breaking changes if applicable
EOF
)"

# Alternative: Use the GitHub web interface
# Visit the URL provided when you push your branch
```

#### 5. **Pull Request Requirements**
Before creating a PR, ensure:
- [ ] **Code reviewed against CONVENTIONS.md** for compliance
- [ ] **All quality checks pass**: `cargo fmt && cargo clippy && cargo test`
- [ ] Branch is up to date with master
- [ ] Descriptive title and detailed description
- [ ] Tests are included and passing
- [ ] Documentation is updated (README.md, CLI help) if needed
- [ ] **IMPORTANT**: CLAUDE.md remains local only - never commit to repository
- [ ] **IMPORTANT**: No Claude or AI references in commit messages

#### 6. **Review and Merge Process**
- PRs require review before merging
- Address all review feedback
- Ensure CI checks pass
- Squash commits if requested
- Delete feature branch after merge

### Development Process

When implementing new features or making changes, follow these steps to ensure code quality:

### Feature Development Checklist

For every new feature or significant change, complete this checklist:

#### 0. Pre-Development Setup (Critical)
- [ ] **REQUIRED**: Ensure you're working on latest master branch
- [ ] **REQUIRED**: Run `git checkout master && git pull origin master` before creating feature branch
- [ ] **REQUIRED**: Verify you have the latest changes before starting work
- [ ] Create feature branch from latest master (see Git Workflow section above)

#### 1. Code Review & Conventions
- [ ] Review existing similar code to understand patterns and conventions
- [ ] **REQUIRED**: Follow all conventions in CONVENTIONS.md
- [ ] Follow established naming conventions (snake_case for functions/variables, PascalCase for types)
- [ ] Use existing libraries and utilities already in the codebase
- [ ] Ensure error handling follows project patterns (using `anyhow::Result`, `bail!`, `.context()`)
- [ ] Follow string interpolation rules (use named placeholders)
- [ ] Add appropriate documentation/comments if needed
- [ ] Follow security best practices (no secrets/keys in code)

#### 2. Implementation
- [ ] Implement the feature following existing code patterns
- [ ] Add appropriate error handling
- [ ] Ensure functions have clear, single responsibilities
- [ ] Use existing dependencies where possible instead of adding new ones

#### 3. Testing
- [ ] Add unit tests for new functionality
- [ ] **REQUIRED**: Ensure test functions return `Result<()>` and use `?` operator (see CONVENTIONS.md)
- [ ] Ensure tests cover both success and error cases
- [ ] Update existing tests if behavior changes
- [ ] Run all tests: `cargo test`
- [ ] Verify tests pass with: `cargo test -- --nocapture`
- [ ] Run specific tests if needed: `cargo test test_name`

#### 4. Code Quality Checks (Required)
- [ ] Run linting: `cargo clippy`
- [ ] Fix any clippy warnings or errors
- [ ] Run strict linting: `cargo clippy -- -D warnings`
- [ ] Format code: `cargo fmt`
- [ ] Verify formatting: `cargo fmt --check`

#### 5. Integration Testing
- [ ] Test the feature manually with realistic inputs
- [ ] Test error conditions and edge cases
- [ ] Verify output format is correct (especially for CLI commands)
- [ ] Test with different authentication methods if applicable (Bitcoin RPC)

#### 6. Documentation (Critical)
- [ ] **REQUIRED**: Update README.md with any new features, commands, or usage changes
- [ ] **REQUIRED**: Update CLAUDE.md if new commands or workflows are added (keep local, don't commit)
- [ ] **REQUIRED**: Update usage examples in both README.md and CLAUDE.md as needed
- [ ] **REQUIRED**: Ensure CLI help text is accurate and helpful (`--help`)
- [ ] **IMPORTANT**: Always update documentation as part of feature development, not as an afterthought

#### 7. Final Verification
- [ ] Run complete check sequence: `cargo test && cargo clippy && cargo fmt --check`
- [ ] Ensure all checks pass before committing
- [ ] Test the feature end-to-end one final time

### Quality Standards
- **Review code against CONVENTIONS.md** before committing
- All new code must pass `cargo clippy` without warnings
- All code must be formatted with `cargo fmt`
- Tests must be added for new functionality
- Error handling must use `anyhow::Result` consistently
- Follow existing code patterns and naming conventions  
- No hardcoded secrets or credentials
- **Note**: `return Err(anyhow::anyhow!(...))` can always be replaced with `anyhow::bail!(...)` for cleaner code

### Before Committing or Creating PRs
**ALWAYS** run the complete check sequence and ensure all steps pass:
```bash
# Required quality checks in order:
nix develop -c cargo fmt        # Format code first
nix develop -c cargo clippy     # Check for issues
nix develop -c cargo test       # Run all tests

# Alternative: Run complete sequence from CONVENTIONS.md
nix develop -c bash -c "cargo clippy --fix --allow-dirty && cargo fmt && cargo test && cargo clippy && cargo fmt --check"
```

If any step fails, fix the issues before proceeding with the commit or PR.

### Critical Workflow Reminders

**Pre-Development Requirements:**
- **ALWAYS** start from latest master: `git checkout master && git pull origin master`
- **NEVER** work on stale branches - this leads to merge conflicts and outdated code
- Verify you have the latest changes before creating any feature branch

**Documentation Requirements:**
- README.md **MUST** be updated for any user-facing changes (new commands, modified behavior, etc.)
- CLAUDE.md should be updated for development workflow changes but **NEVER** committed to git
- Documentation updates are not optional - they are a required part of every feature

**Commit Guidelines:**
- **NEVER** include Claude, AI, or similar references in commit messages
- Commit messages should focus on the technical change, not the tool used to implement it
- Use conventional commit format: `type(scope): description`

## Bitcoin Core RPC Integration

The Bitcoin RPC functionality supports two authentication methods:

1. **Cookie Authentication (Recommended)**: Uses the `.cookie` file from Bitcoin Core's data directory
   - Bitcoin Core automatically generates a cookie file on startup
   - Default location: `~/.bitcoin/.cookie` (Linux/macOS) or `%APPDATA%\Bitcoin\.cookie` (Windows)
   - Provides secure, automatic authentication without manual credential management
   - Use `--bitcoin-dir` parameter to specify custom Bitcoin data directory

2. **Username/Password Authentication**: Uses manually configured RPC credentials
   - Requires `rpcuser` and `rpcpassword` in `bitcoin.conf`
   - Use `--rpc-user` and `--rpc-password` parameters
   - Conflicts with `--bitcoin-dir` to prevent authentication confusion

The client automatically tries cookie authentication first (if `--bitcoin-dir` is provided), then falls back to username/password if cookie reading fails.

# Roadmap

## LNURL related

- [x] Implement generating an invoice from a lnurl address like darkparty@walletofsatoshi.com

## Tapsigner and SatsCard support

- [x] Generate a new address on tapsigner
- [x] Generate a new address on satscard

## Bitcoin Core RPC Wrappers

- [x] List UTXOs for a given Output Descriptor or Wallet
- [x] Create PSBT (Partially Signed Bitcoin Transaction) for spending a UTXO
- [x] Support custom fee rates in sats/vB (both manual calculation and wallet-funded approaches)
- [x] Automatic input selection and change handling with `walletcreatefundedpsbt`
- [x] Fee estimation with confirmation targets and estimation modes
- [x] UTXO consolidation functionality with `move-utxos` command
- [x] Coin selection with --max-amount to limit total amount moved
- [ ] Transaction broadcast functionality