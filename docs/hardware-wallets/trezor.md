# Trezor Hardware Wallet Support

cyberkrill provides comprehensive support for Trezor hardware wallets.

## Overview

Trezor is a popular hardware wallet that:
- Stores private keys securely offline
- Signs transactions without exposing keys
- Supports multiple cryptocurrencies
- Provides a secure display for verification

## Features

- Extended public key (xpub) extraction
- Address generation with custom paths
- Network support (mainnet, testnet, regtest)
- BIP-32/39/44/49/84 compliance

## Setup

### Prerequisites

1. **Trezor Device**: Model One or Model T
2. **USB Connection**: Direct USB connection to computer
3. **Build Flag**: Compile with Trezor support
   ```bash
   cargo build --features trezor
   ```

### Device Preparation

1. Connect Trezor via USB
2. Unlock with PIN if required
3. Ensure device is in ready state

## Usage

### Get Extended Public Key

```bash
# Get xpub for standard Bitcoin account
cyberkrill trezor-xpub --path "m/84'/0'/0'" --network mainnet

# Testnet xpub
cyberkrill trezor-xpub --path "m/84'/1'/0'" --network testnet

# Save to file
cyberkrill trezor-xpub --path "m/84'/0'/0'" -o xpub.json
```

### Generate Address

```bash
# Generate first receive address
cyberkrill trezor-address --path "m/84'/0'/0'/0/0" --network mainnet

# Generate change address
cyberkrill trezor-address --path "m/84'/0'/0'/1/0" --network mainnet
```

## Derivation Paths

### Standard Paths

| Purpose | Path Template | Description |
|---------|--------------|-------------|
| BIP-44 (Legacy) | m/44'/0'/0'/0/0 | P2PKH addresses (1...) |
| BIP-49 (Nested SegWit) | m/49'/0'/0'/0/0 | P2SH-P2WPKH (3...) |
| BIP-84 (Native SegWit) | m/84'/0'/0'/0/0 | P2WPKH (bc1q...) |

### Path Components

- `m` - Master key
- `84'` - Purpose (BIP-84 for native segwit)
- `0'` - Coin type (0 for Bitcoin, 1 for testnet)
- `0'` - Account number
- `0` - Chain (0 for receive, 1 for change)
- `0` - Address index

## Security Considerations

### Device Verification

Always verify addresses on the Trezor display:
- Check the address matches what's shown in cyberkrill
- Confirm on the device screen
- Never trust addresses only shown on computer

### PIN Protection

- Always use a strong PIN
- Never share your PIN
- Be aware of shoulder surfing

### Passphrase (Optional)

Trezor supports an optional passphrase:
- Acts as a 25th word to your seed
- Creates hidden wallets
- Currently requires manual entry on device

## Troubleshooting

### Device Not Found

```bash
# Check if device is connected
lsusb | grep -i trezor

# Ensure udev rules are installed (Linux)
# Download from: https://github.com/trezor/trezor-common/blob/master/udev/51-trezor.rules
sudo cp 51-trezor.rules /etc/udev/rules.d/
sudo udevadm control --reload-rules
sudo udevadm trigger
```

### Communication Errors

1. Disconnect and reconnect device
2. Try different USB port
3. Ensure device firmware is up to date
4. Close Trezor Suite/Bridge if running

### Permission Issues (Linux)

```bash
# Add user to plugdev group
sudo usermod -a -G plugdev $USER
# Log out and back in for changes to take effect
```

## Advanced Usage

### Multiple Accounts

```bash
# Account 0 (default)
cyberkrill trezor-xpub --path "m/84'/0'/0'"

# Account 1
cyberkrill trezor-xpub --path "m/84'/0'/1'"

# Account 2
cyberkrill trezor-xpub --path "m/84'/0'/2'"
```

### Integration with Bitcoin Operations

```bash
# Get xpub from Trezor
XPUB=$(cyberkrill trezor-xpub --path "m/84'/0'/0'" | jq -r .xpub)

# Use xpub to list UTXOs
cyberkrill list-utxos --descriptor "wpkh($XPUB/0/*)"

# Create PSBT for signing
cyberkrill create-psbt --inputs "txid:0" --outputs "bc1q...:0.001"
```

## Technical Details

### Implementation

- Uses `trezor-client` Rust library
- Communicates via USB HID protocol
- Supports Trezor protocol v1 and v2
- Async/await for non-blocking operations

### Supported Networks

- Bitcoin Mainnet
- Bitcoin Testnet
- Bitcoin Regtest
- Bitcoin Signet (planned)