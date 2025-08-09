# BDK Wallet List UTXOs Implementation

## Overview

I've successfully implemented a BDK-based `bdk-list-utxos` command for the cyberkrill CLI toolkit. This command allows listing UTXOs from Bitcoin descriptors using the BDK (Bitcoin Development Kit) library.

## Features Implemented

### 1. Core BDK Wallet Module (`cyberkrill-core/src/bdk_wallet.rs`)
- `list_utxos_bdk()` - List UTXOs from a descriptor without blockchain connection
- `scan_and_list_utxos_electrum()` - Scan blockchain via Electrum and list UTXOs
- `get_utxo_summary()` - Generate summary statistics for UTXOs
- Support for all standard Bitcoin networks (mainnet, testnet, signet, regtest)

### 2. CLI Command (`cyberkrill bdk-list-utxos`)
- `--descriptor` - Required Bitcoin output descriptor
- `--network` - Bitcoin network selection (default: mainnet)
- `--electrum` - Optional Electrum server URL for blockchain scanning
- `--stop-gap` - Address derivation gap limit (default: 200)
- `-o, --output` - Save results to file

### 3. UTXO Information Returned
- Transaction ID and output index
- Bitcoin address
- Amount in satoshis and BTC
- Confirmation count
- Change output detection
- Keychain type (external/internal)

### 4. Summary Statistics
- Total UTXO count
- Total amount (sats and BTC)
- Confirmed vs unconfirmed counts
- Detailed UTXO list

## Usage Examples

### Basic Usage (No Blockchain Scan)
```bash
cyberkrill bdk-list-utxos --descriptor "wpkh([fingerprint/84h/0h/0h]xpub.../0/*)" --network mainnet
```

### With Electrum Server
```bash
cyberkrill bdk-list-utxos \
  --descriptor "wpkh(xpub.../0/*)" \
  --network testnet \
  --electrum "tcp://localhost:50001" \
  --stop-gap 100
```

### Save to File
```bash
cyberkrill bdk-list-utxos --descriptor "wpkh(xpub.../0/*)" -o utxos.json
```

## Technical Details

### Dependencies Added
- `bdk_wallet = "2.0.0"` - Core BDK wallet functionality
- `bdk_electrum = "0.23.0"` - Electrum blockchain backend
- `bdk_esplora = "0.22.0"` - Esplora blockchain backend (for future use)

All dependencies configured to use rustls instead of OpenSSL for better portability.

### Key Design Decisions

1. **Single Descriptor Support**: Uses `Wallet::create_single()` to support wallets with only an external descriptor (no separate change descriptor required).

2. **Network Handling**: Properly maps string network names to Bitcoin network types.

3. **Error Handling**: Comprehensive error messages with context for debugging.

4. **Compatibility**: Works with standard BIP32/BIP44/BIP49/BIP84 descriptors.

## Limitations

1. **No Multipath Support**: BDK doesn't yet support multipath descriptors with `<0;1>` syntax.

2. **SSL/TLS**: Electrum SSL connections require proper TLS initialization (rustls).

3. **Watch-Only**: Only supports public key descriptors (xpub), not private keys.

## Testing

The implementation includes:
- Unit tests for wallet creation
- Test scripts for various descriptor types
- Support for both local (no scan) and Electrum-based operation

## Future Enhancements

1. Add Esplora backend support
2. Support for multipath descriptors when BDK adds support
3. Transaction history retrieval
4. PSBT creation from BDK wallets
5. Wallet persistence options