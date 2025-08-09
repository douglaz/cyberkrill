# Jade Hardware Wallet Integration Plan for cyberkrill

## Overview
This document outlines the plan to add Blockstream Jade hardware wallet support to cyberkrill, following the existing patterns established for Coldcard and Trezor integration. The implementation will use serial/USB communication with CBOR protocol.

## Architecture Overview

### Communication Protocol
- **Transport**: USB Serial connection
- **Protocol**: CBOR (Concise Binary Object Representation)
- **Message Format**: Request/Response pattern with JSON-like structure encoded in CBOR
- **Authentication**: Device authentication required before operations

### Integration Pattern
Following the existing hardware wallet pattern in cyberkrill:
1. Core implementation in `cyberkrill-core/src/jade.rs`
2. Optional implementation of `HardwareWallet` trait
3. CLI commands in `cyberkrill/src/main.rs`
4. Feature flag `jade` for conditional compilation

## Implementation Phases

### Phase 1: Dependencies and Feature Flags

#### 1.1 Update `cyberkrill-core/Cargo.toml`
```toml
[features]
jade = ["dep:serde_cbor", "dep:serialport"]

[dependencies]
serde_cbor = { version = "0.11", optional = true }
serialport = { version = "4.2", optional = true }
```

#### 1.2 Update `cyberkrill/Cargo.toml`
```toml
[features]
jade = ["cyberkrill-core/jade"]
```

### Phase 2: Core Implementation

#### 2.1 Create `cyberkrill-core/src/jade.rs`

**Core Structure**:
```rust
pub struct JadeWallet {
    port: Box<dyn SerialPort>,
    message_counter: u32,
}
```

**Key Methods**:
- `connect()` - Auto-detect and connect to Jade device
- `connect_path(path: &str)` - Connect to specific serial port
- `auth_user(network: Network)` - Authenticate with the device
- `get_version_info()` - Retrieve device and firmware information
- `get_xpub(path: &str)` - Get extended public key at derivation path
- `get_receive_address(path: &str, network: Network)` - Generate Bitcoin address
- `sign_psbt(psbt: &[u8])` - Sign Partially Signed Bitcoin Transaction

**USB Device IDs** (for auto-detection):
```rust
const JADE_DEVICE_IDS: &[(u16, u16)] = &[
    (0x10c4, 0xea60),  // CP210x
    (0x1a86, 0x55d4),  // CH9102F
    (0x0403, 0x6001),  // FT232
];
```

#### 2.2 CBOR Message Protocol

**Request Structure**:
```rust
#[derive(Serialize)]
struct JadeRequest {
    method: String,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}
```

**Response Structure**:
```rust
#[derive(Deserialize)]
struct JadeResponse {
    id: String,
    #[serde(flatten)]
    body: ResponseBody,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ResponseBody {
    Result { result: Value },
    Error { error: JadeError },
}
```

#### 2.3 Helper Functions

Export public functions in `lib.rs`:
```rust
#[cfg(feature = "jade")]
pub use jade::{
    generate_jade_address,
    sign_psbt_with_jade,
    get_jade_xpub,
    JadeWallet,
    JadeAddressOutput,
    JadeSignOutput,
};
```

### Phase 3: CLI Integration

#### 3.1 Update `cyberkrill/src/main.rs`

**Add Commands**:
```rust
#[cfg(feature = "jade")]
#[command(about = "Generate Bitcoin address from Jade")]
JadeAddress(JadeAddressArgs),

#[cfg(feature = "jade")]
#[command(about = "Sign PSBT with Jade")]
JadeSignPsbt(JadeSignPsbtArgs),

#[cfg(feature = "jade")]
#[command(about = "Get extended public key from Jade")]
JadeXpub(JadeXpubArgs),
```

**Argument Structures**:
```rust
#[cfg(feature = "jade")]
#[derive(clap::Args, Debug)]
struct JadeAddressArgs {
    #[clap(short, long, default_value = "m/84'/0'/0'/0/0")]
    path: String,
    
    #[clap(short, long, default_value = "bitcoin")]
    network: String,
    
    #[clap(short, long)]
    output: Option<String>,
}
```

#### 3.2 Command Handlers

Implement async handlers:
- `jade_address(args: JadeAddressArgs)`
- `jade_sign_psbt(args: JadeSignPsbtArgs)`
- `jade_xpub(args: JadeXpubArgs)`

### Phase 4: Protocol Implementation Details

#### 4.1 Serial Communication
- **Baud Rate**: 115200
- **Data Bits**: 8
- **Stop Bits**: 1
- **Parity**: None
- **Flow Control**: None
- **RTS/DTR**: Set to false to prevent device reboot

#### 4.2 Authentication Flow
1. Send `auth_user` request with network parameter
2. Handle user confirmation on device
3. Receive authentication confirmation
4. Proceed with operations

#### 4.3 Error Handling
- Device not found errors
- Communication timeouts
- User cancellation on device
- CBOR encoding/decoding errors
- Invalid responses

### Phase 5: Testing Strategy

#### 5.1 Unit Tests
- CBOR message encoding/decoding
- Derivation path parsing
- Response parsing
- Error handling

#### 5.2 Integration Tests (with device)
- Device detection
- Address generation
- PSBT signing
- Network switching

#### 5.3 Mock Tests
- Simulate device responses
- Test error conditions
- CI-friendly testing

### Phase 6: Documentation

#### 6.1 Update README.md
Add Jade examples:
```bash
# Generate address
cargo run --features jade -- jade-address --path "m/84'/0'/0'/0/0" --network bitcoin

# Sign PSBT
cargo run --features jade -- jade-sign-psbt transaction.psbt --network bitcoin

# Get xpub
cargo run --features jade -- jade-xpub --path "m/84'/0'/0'" --network bitcoin
```

#### 6.2 Update CLAUDE.md
Add Jade-specific development commands and notes.

#### 6.3 Linux USB Permissions
Document udev rules for Jade access:
```bash
# /etc/udev/rules.d/51-jade.rules
SUBSYSTEM=="tty", ATTRS{idVendor}=="10c4", ATTRS{idProduct}=="ea60", MODE="0666"
SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="55d4", MODE="0666"
SUBSYSTEM=="tty", ATTRS{idVendor}=="0403", ATTRS{idProduct}=="6001", MODE="0666"
```

## Implementation Considerations

### Option A: Use lwk_jade Crate
**Pros**:
- Already implemented and tested
- Maintained by Blockstream
- Handles protocol complexities

**Cons**:
- Designed for Liquid Network
- May include unnecessary dependencies
- Less control over implementation

### Option B: Custom Implementation
**Pros**:
- Bitcoin-focused implementation
- Consistent with cyberkrill patterns
- Full control over features
- Minimal dependencies

**Cons**:
- More development effort
- Need to implement CBOR protocol
- Requires thorough testing

### Recommendation
Implement Option B (custom implementation) to maintain consistency with existing hardware wallet implementations and have full control over Bitcoin-specific features.

## Technical References

- **Jade Repository**: https://github.com/Blockstream/Jade
- **jadepy Source**: Reference implementation in Python
- **lwk_jade**: https://docs.rs/lwk_jade/ (for protocol reference)
- **CBOR Specification**: RFC 7049

## Development Timeline

| Phase | Description | Estimated Time |
|-------|-------------|----------------|
| Phase 1 | Dependencies and setup | 2 hours |
| Phase 2 | Core implementation | 2-3 days |
| Phase 3 | CLI integration | 1 day |
| Phase 4 | Protocol refinement | 1 day |
| Phase 5 | Testing | 1 day |
| Phase 6 | Documentation | 4 hours |
| **Total** | **Complete Integration** | **~5-6 days** |

## Success Criteria

- [ ] Device auto-detection works reliably
- [ ] Address generation matches other wallets
- [ ] PSBT signing produces valid signatures
- [ ] All networks (mainnet, testnet, regtest) supported
- [ ] Error handling is robust and user-friendly
- [ ] Documentation is complete and accurate
- [ ] Tests provide good coverage
- [ ] Feature flag properly isolates Jade code

## Future Enhancements

1. **Bluetooth Support**: Add BLE communication option
2. **Multisig Support**: Register and use multisig wallets
3. **Message Signing**: Sign arbitrary messages
4. **Firmware Updates**: Support firmware update protocol
5. **QR Code Support**: Camera-based air-gapped operation