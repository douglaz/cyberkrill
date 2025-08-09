# CLAUDE.md

**Important**: Keep locally, don't commit. Development instructions for Claude Code.

## Project Overview

Rust workspace with three crates:
- **cyberkrill**: CLI application (argument parsing, user interaction, output formatting)
- **cyberkrill-core**: Core library (business logic for Bitcoin, Lightning, Fedimint, hardware wallets)
- **fedimint-lite**: Standalone Fedimint invite code encoding/decoding library

## Architecture

### cyberkrill (CLI)
- `src/main.rs`: Entry point, CLI parsing with clap, dispatches to core functions

### cyberkrill-core (Library)
- `src/lib.rs`: Public API exports
- `src/decoder.rs`: Lightning invoice (BOLT11) and LNURL decoding
- `src/bitcoin_rpc.rs`: Bitcoin Core RPC client for UTXO/PSBT operations
- `src/tapsigner.rs`: Tapsigner hardware wallet (requires `smartcards` feature)
- `src/satscard.rs`: Satscard hardware wallet (requires `smartcards` feature)
- `src/frozenkrill.rs`: frozenkrill wallet JSON support (requires `frozenkrill` feature)
- `src/coldcard.rs`: Coldcard hardware wallet (requires `coldcard` feature)
- `src/hardware_wallet.rs`: Common trait for hardware wallet implementations

### fedimint-lite
- `src/lib.rs`: Fedimint invite code handling with fedimint-cli compatibility

## Key Features

1. **Commands**:
   - `decode-invoice|decode-lnurl|decode-fedimint-invite`: Decode various formats
   - `generate-invoice`: Create invoices from Lightning addresses
   - `fedimint-config`: Fetch federation configuration
   - `tapsigner address|init`: Tapsigner hardware wallet operations
   - `satscard address`: Generate addresses from card slots
   - `coldcard-address|coldcard-sign-psbt|coldcard-export-psbt`: Coldcard hardware wallet operations
   - `list-utxos|create-psbt|create-funded-psbt|move-utxos`: Bitcoin operations

2. **Amount Formats**: `0.5` (BTC), `0.5btc`, `50000000sats`

3. **Backend Support**: Bitcoin Core RPC, Electrum, Esplora (via BDK)

4. **Authentication**: Cookie-based (recommended) or username/password

## Quick Commands

```bash
# Build
cargo build
cargo build --features smartcards
cargo build --features coldcard
cargo build --features smartcards,coldcard

# Test
cargo test
cargo test test_name

# Run with nix
nix develop -c cargo build
nix develop -c cargo test
nix develop -c cargo build --features coldcard

# Common operations
cargo run -- decode-invoice <invoice>
cargo run -- generate-invoice user@domain.com 1000000
cargo run -- list-utxos --bitcoin-dir ~/.bitcoin --descriptor "wpkh(...)"
cargo run -- create-psbt --bitcoin-dir ~/.bitcoin --inputs "txid:0" --outputs "addr:0.001" --fee-rate 10
cargo run -- create-funded-psbt --bitcoin-dir ~/.bitcoin --outputs "addr:0.001" --fee-rate 20
cargo run -- move-utxos --bitcoin-dir ~/.bitcoin --inputs "txid:0" --destination "addr" --fee-rate 15

# With alternative backends
cargo run -- list-utxos --electrum ssl://electrum.blockstream.info:50002 --descriptor "wpkh(...)"
cargo run -- list-utxos --esplora https://blockstream.info/api --descriptor "wpkh(...)"

# Hardware wallets
# Tapsigner/Satscard (requires smartcards feature)
export TAPSIGNER_CVC=123456
cargo run --features smartcards -- tapsigner address --path "m/84'/0'/0'/0/0"
cargo run --features smartcards -- satscard address --slot 1

# Coldcard (requires coldcard feature + GNU target)
# IMPORTANT: Coldcard doesn't work with musl builds due to hidapi error handling
cargo run --target x86_64-unknown-linux-gnu --features coldcard -- coldcard-address --path "m/84'/0'/0'/0/0" --network mainnet
cargo run --target x86_64-unknown-linux-gnu --features coldcard -- coldcard-sign-psbt transaction.psbt -o signed.json --psbt-output signed.psbt
cargo run --target x86_64-unknown-linux-gnu --features coldcard -- coldcard-export-psbt transaction.psbt --filename "to-sign.psbt"
```

## Development Workflow

### Before Starting
```bash
git checkout master && git pull origin master
git checkout -b feature/description
```

### Quality Checks (REQUIRED before commits/PRs)
```bash
# Run in order:
nix develop -c cargo fmt
nix develop -c cargo clippy
nix develop -c cargo test

# Or all at once:
nix develop -c bash -c "cargo clippy --fix --allow-dirty && cargo fmt && cargo test && cargo clippy && cargo fmt --check"
```

### Commit Guidelines
- Use conventional commits: `feat|fix|refactor|docs|test|chore(scope): description`
- NO Claude/AI references in commits
- Only add changed files: `git add specific/file.rs`

### PR Requirements
1. Review against CONVENTIONS.md
2. Pass all quality checks
3. Update documentation if needed
4. Keep CLAUDE.md local

## Library Usage

```toml
[dependencies]
cyberkrill-core = { git = "https://github.com/douglaz/cyberkrill" }
# With smartcards: features = ["smartcards"]
```

```rust
use cyberkrill_core::{decode_invoice, decode_lnurl, BitcoinRpcClient};

let invoice = decode_invoice("lnbc...")?;
let lnurl = decode_lnurl("LNURL...")?;
```

## Technical Notes

### BDK 2.0 Integration
- New API: `Wallet::create(...).network(...).create_wallet()`
- Custom multipath descriptor expansion for `<0;1>/*` syntax
- Manual UTXO conversion from Bitcoin Core RPC

### Fedimint Compatibility
- Use `--skip-api-secret` for fedimint-cli compatibility
- Only bech32m format (fed1...) supported

### Coldcard Musl Limitation
- **Issue**: hidapi's error handling is incompatible with musl (both static and dynamic)
- **Error**: "hid_error is not implemented yet" at runtime
- **Solution**: Use GNU target: `--target x86_64-unknown-linux-gnu`
- **Backend**: Currently using `linux-static-libusb` (compiles but runtime fails with musl)

### Common Pitfalls
- Always use `nix develop -c` for SQLite/musl builds
- Network mismatches between descriptors and wallets
- Missing UTXOs if filtering by address presence
- Coldcard requires GNU target due to hidapi limitations