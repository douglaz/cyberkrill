# fedimint-lite

A lightweight Rust library for encoding and decoding Fedimint invite codes.

## Features

- 🔓 **Decode** Fedimint invite codes (bech32m format)
- 🔐 **Encode** structured data back to invite codes
- 🌐 **Fetch** federation configuration from invite codes
- ✅ **Full compatibility** with fedimint-cli
- 🚀 **Lightweight** with minimal dependencies
- 🦀 **Pure Rust** implementation

## Installation

```toml
[dependencies]
fedimint-lite = "0.1"
```

## Usage

### Decoding Invite Codes

```rust
use fedimint_lite::{decode_invite, InviteCode};

fn main() -> anyhow::Result<()> {
    let invite_str = "fed11qgqzx..."; // Your invite code
    
    let invite: InviteCode = decode_invite(invite_str)?;
    
    println!("Federation ID: {}", invite.federation_id);
    println!("Guardians:");
    for guardian in &invite.guardians {
        println!("  - Peer {}: {}", guardian.peer_id, guardian.url);
    }
    
    Ok(())
}
```

### Encoding Invite Codes

```rust
use fedimint_lite::{encode_invite, InviteCode, GuardianInfo};

fn main() -> anyhow::Result<()> {
    let invite = InviteCode {
        federation_id: "b21068c84f5b12ca4fdf93f3e443d3bd7c27e8642d0d52ea2e4dce6fdbbee9df".to_string(),
        guardians: vec![
            GuardianInfo {
                peer_id: 0,
                url: "wss://api.bitcoin-principles.com/".to_string(),
            }
        ],
        api_secret: None,
        encoding_format: "bech32m".to_string(),
    };
    
    let encoded = encode_invite(&invite)?;
    println!("Invite code: {}", encoded);
    
    Ok(())
}
```

### Fetching Federation Config

```rust
use fedimint_lite::fetch_config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let invite_code = "fed11qgqzx...";
    
    let config = fetch_config(invite_code).await?;
    
    println!("Federation name: {:?}", config.federation_name);
    println!("Consensus version: {}", config.consensus_version);
    
    Ok(())
}
```

## Compatibility

This library generates invite codes that are fully compatible with fedimint-cli. However, note that:

- Only bech32m format (`fed1...`) is supported
- API secrets in invite codes may not be compatible with all fedimint-cli versions

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.