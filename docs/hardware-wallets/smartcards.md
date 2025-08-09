# Smartcard Support (Tapsigner & Satscard)

cyberkrill provides native support for Coinkite smartcards via NFC/USB card readers.

## Overview

Smartcards are NFC-enabled devices that require a card reader for communication. They differ from traditional hardware wallets in that they:
- Use NFC communication instead of USB/Bluetooth
- Are card-form factor (credit card size)
- Require PIN/CVC authentication
- Are designed for specific use cases

## Tapsigner

### What is Tapsigner?

Tapsigner is a BIP-32 HD wallet in a card form factor that:
- Generates and stores private keys securely
- Never exposes private keys
- Signs transactions when tapped
- Supports custom derivation paths

### Setup

1. **System Requirements**
   - USB NFC card reader (e.g., OMNIKEY 5022 CL)
   - cyberkrill built with smartcard support (default)

2. **Initial Setup**
   ```bash
   # Set your 6-digit PIN (found on card back)
   export TAPSIGNER_CVC=123456
   
   # Initialize the card (ONE-TIME ONLY)
   cyberkrill tapsigner-init
   ```

3. **Generate Addresses**
   ```bash
   # Default BIP-84 address
   cyberkrill tapsigner-address
   
   # Custom derivation path
   cyberkrill tapsigner-address --path "m/84'/0'/0'/0/5"
   
   # Save to file
   cyberkrill tapsigner-address -o address.json
   ```

### Security Notes

- The initialization process is **irreversible**
- The card combines your entropy with internal randomness
- PIN is required for all operations
- Backup your card after initialization

## Satscard

### What is Satscard?

Satscard is a bearer instrument that:
- Contains 10 independent key slots
- Each slot can be used once for spending
- Works like physical cash - whoever holds it can spend
- No PIN required for address generation

### Usage

```bash
# Get address from current active slot
cyberkrill satscard-address

# Get address from specific slot (0-9)
cyberkrill satscard-address --slot 2

# Save to file
cyberkrill satscard-address -o address.json
```

### Key Differences from Tapsigner

| Feature | Tapsigner | Satscard |
|---------|-----------|----------|
| Purpose | HD wallet | Bearer instrument |
| Slots | Unlimited addresses | 10 fixed slots |
| PIN Required | Yes | No (for addresses) |
| Derivation Path | Customizable | Fixed (m/0) |
| Reusable | Yes | Each slot once |

## Troubleshooting

### Card Not Detected

1. Ensure NFC reader is connected
2. Check reader is recognized: `lsusb`
3. Place card firmly on reader
4. Try different USB ports

### Permission Issues

If you get permission errors:
```bash
# Run with sudo (temporary fix)
sudo cyberkrill tapsigner-address

# Or set up udev rules (permanent fix)
# Create /etc/udev/rules.d/99-nfc.rules with:
SUBSYSTEM=="usb", ATTR{idVendor}=="076b", MODE="0666"
```

### CVC/PIN Issues

- CVC must be exactly 6 digits
- Check card back or documentation for CVC
- CVC is case-sensitive (numbers only)

## Technical Details

### Implementation

- Uses `rust-cktap` library for communication
- PCSC protocol for card interaction
- Supports all major NFC readers
- Cross-platform support (Linux, macOS, Windows)

### Supported Operations

**Tapsigner:**
- Initialize card
- Generate addresses
- Sign PSBTs (planned)
- Backup/restore (planned)

**Satscard:**
- Generate addresses
- Check slot status
- Sweep slots (planned)