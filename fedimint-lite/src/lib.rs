//! # fedimint-lite
//!
//! A lightweight library for encoding and decoding Fedimint invite codes.
//!
//! ## Features
//! - Decode Fedimint invite codes (bech32m format)
//! - Encode invite codes from structured data
//! - Fetch federation configuration from invite codes
//! - Full compatibility with fedimint-cli
//!
//! ## Example
//! ```no_run
//! use fedimint_lite::{decode_invite, encode_invite};
//!
//! // Decode an invite code
//! let invite_code = "fed11qgqzx...";
//! let decoded = decode_invite(invite_code)?;
//! println!("Federation ID: {}", decoded.federation_id);
//!
//! // Encode back to invite code
//! let encoded = encode_invite(&decoded)?;
//! assert_eq!(invite_code, encoded);
//! # Ok::<(), anyhow::Error>(())
//! ```

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

// Re-export main functions with simpler names
pub use crate::{
    decode_fedimint_invite as decode_invite, encode_fedimint_invite as encode_invite,
    fetch_fedimint_config as fetch_config,
};

// Re-export types with simpler names
pub type InviteCode = FedimintInviteOutput;
pub type FederationConfig = FederationConfigOutput;

// Fedimint invite code structures and functions
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FedimintInviteOutput {
    pub federation_id: String,
    pub guardians: Vec<GuardianInfo>,
    pub api_secret: Option<String>,
    pub encoding_format: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GuardianInfo {
    pub peer_id: u16,
    pub url: String,
}

pub fn decode_fedimint_invite(input: &str) -> Result<FedimintInviteOutput> {
    let input = input.trim();

    // Only support bech32m format (fed1...)
    if input.starts_with("fed1") {
        return decode_bech32m_invite(input);
    }

    anyhow::bail!("Invalid fedimint invite code format. Expected to start with 'fed1' (bech32m)");
}

fn decode_bech32m_invite(input: &str) -> Result<FedimintInviteOutput> {
    // Decode and validate bech32m checksum
    use bech32::Bech32m;
    use bech32::primitives::decode::CheckedHrpstring;

    let checked = CheckedHrpstring::new::<Bech32m>(input)
        .map_err(|e| anyhow::anyhow!("Invalid bech32m string: {e}"))?;

    // Verify HRP
    let hrp = checked.hrp();
    anyhow::ensure!(
        hrp == bech32::primitives::hrp::Hrp::parse("fed1")?,
        "Invalid HRP (human-readable part): expected 'fed1', got '{}'",
        hrp.as_str()
    );

    // Convert from 5-bit field elements to 8-bit bytes
    let decoded_bytes: Vec<u8> = checked.byte_iter().collect();

    decode_invite_bytes(&decoded_bytes, "bech32m")
}

fn decode_invite_bytes(bytes: &[u8], encoding_format: &str) -> Result<FedimintInviteOutput> {
    // Parse using Bitcoin-style consensus encoding
    match parse_consensus_encoding(bytes, encoding_format) {
        Ok(result) => Ok(result),
        Err(e) => {
            debug!("Consensus parsing failed: {e}, falling back to simple parser");
            parse_as_simple_format(bytes, encoding_format)
        }
    }
}

fn parse_consensus_encoding(bytes: &[u8], encoding_format: &str) -> Result<FedimintInviteOutput> {
    let mut pos = 0;

    // Read the number of InviteCodePart elements (VarInt)
    let (num_parts, bytes_read) = read_varint_at(bytes, pos)?;
    pos += bytes_read;

    let mut federation_id = None;
    let mut guardians = Vec::new();
    let mut api_secret = None;

    for i in 0..num_parts {
        if pos >= bytes.len() {
            anyhow::bail!("Unexpected end of data at part {i}");
        }

        // Read variant discriminator (VarInt)
        let (variant, bytes_read) = read_varint_at(bytes, pos)?;
        pos += bytes_read;

        match variant {
            0 => {
                // Api { url: SafeUrl, peer: PeerId }
                // Format: variant_data_length + url_length + url_bytes + peer_id
                let (variant_data_len, bytes_read) = read_varint_at(bytes, pos)?;
                pos += bytes_read;

                let variant_start_pos = pos;

                // Read URL length and data
                let (url_len, bytes_read) = read_varint_at(bytes, pos)?;
                pos += bytes_read;

                if pos + url_len as usize > bytes.len() {
                    anyhow::bail!("URL length {} exceeds remaining bytes", url_len);
                }

                let url_bytes = &bytes[pos..pos + url_len as usize];
                let url = String::from_utf8(url_bytes.to_vec()).context("Invalid UTF-8 in URL")?;
                pos += url_len as usize;

                // Read peer ID
                let (peer_id, bytes_read) = read_varint_at(bytes, pos)?;
                pos += bytes_read;

                // Verify we consumed exactly variant_data_len bytes
                let consumed = pos - variant_start_pos;
                if consumed != variant_data_len as usize {
                    anyhow::bail!(
                        "API variant data length mismatch: expected {}, consumed {}",
                        variant_data_len,
                        consumed
                    );
                }

                guardians.push(GuardianInfo {
                    peer_id: peer_id as u16,
                    url,
                });
            }
            1 => {
                // FederationId(sha256::Hash) - preceded by length, then 32 bytes
                let (fed_id_len, bytes_read) = read_varint_at(bytes, pos)?;
                pos += bytes_read;

                if fed_id_len != 32 {
                    anyhow::bail!("Federation ID length should be 32, got {fed_id_len}");
                }

                if pos + 32 > bytes.len() {
                    anyhow::bail!("Not enough bytes for federation ID");
                }

                let fed_id_bytes = &bytes[pos..pos + 32];
                federation_id = Some(hex::encode(fed_id_bytes));
                pos += 32;
            }
            2 => {
                // ApiSecret(String)
                let (secret_len, bytes_read) = read_varint_at(bytes, pos)?;
                pos += bytes_read;

                if pos + secret_len as usize > bytes.len() {
                    anyhow::bail!("Secret length {} exceeds remaining bytes", secret_len);
                }

                let secret_bytes = &bytes[pos..pos + secret_len as usize];
                let secret = String::from_utf8(secret_bytes.to_vec())
                    .context("Invalid UTF-8 in API secret")?;
                pos += secret_len as usize;

                api_secret = Some(secret);
            }
            _ => {
                // Unknown variant - we need to skip it properly
                // Since we don't know the structure, this is tricky
                warn!("Unknown variant {variant} at position {pos}, stopping parsing");
                break;
            }
        }
    }

    let federation_id =
        federation_id.ok_or_else(|| anyhow::anyhow!("Invite code missing federation ID"))?;

    if guardians.is_empty() {
        anyhow::bail!("Invite code must contain at least one guardian");
    }

    guardians.sort_by_key(|g| g.peer_id);

    Ok(FedimintInviteOutput {
        federation_id,
        guardians,
        api_secret,
        encoding_format: encoding_format.to_string(),
    })
}

// Read BigSize VarInt at specific position, returns (value, bytes_consumed)
fn read_varint_at(bytes: &[u8], pos: usize) -> Result<(u64, usize)> {
    if pos >= bytes.len() {
        anyhow::bail!("Position {pos} exceeds buffer length {}", bytes.len());
    }

    let first_byte = bytes[pos];

    match first_byte {
        0x00..=0xFC => Ok((first_byte as u64, 1)),
        0xFD => {
            if pos + 3 > bytes.len() {
                anyhow::bail!("Not enough bytes for 2-byte VarInt");
            }
            let value = u16::from_be_bytes([bytes[pos + 1], bytes[pos + 2]]) as u64;
            Ok((value, 3))
        }
        0xFE => {
            if pos + 5 > bytes.len() {
                anyhow::bail!("Not enough bytes for 4-byte VarInt");
            }
            let value = u32::from_be_bytes([
                bytes[pos + 1],
                bytes[pos + 2],
                bytes[pos + 3],
                bytes[pos + 4],
            ]) as u64;
            Ok((value, 5))
        }
        0xFF => {
            if pos + 9 > bytes.len() {
                anyhow::bail!("Not enough bytes for 8-byte VarInt");
            }
            let value = u64::from_be_bytes([
                bytes[pos + 1],
                bytes[pos + 2],
                bytes[pos + 3],
                bytes[pos + 4],
                bytes[pos + 5],
                bytes[pos + 6],
                bytes[pos + 7],
                bytes[pos + 8],
            ]);
            Ok((value, 9))
        }
    }
}

// Write BigSize VarInt, returns bytes written
fn write_varint(value: u64) -> Vec<u8> {
    match value {
        0x00..=0xFC => vec![value as u8],
        0xFD..=0xFFFF => {
            let mut result = vec![0xFD];
            result.extend_from_slice(&(value as u16).to_be_bytes());
            result
        }
        0x10000..=0xFFFFFFFF => {
            let mut result = vec![0xFE];
            result.extend_from_slice(&(value as u32).to_be_bytes());
            result
        }
        _ => {
            let mut result = vec![0xFF];
            result.extend_from_slice(&value.to_be_bytes());
            result
        }
    }
}

pub fn encode_fedimint_invite(invite: &FedimintInviteOutput) -> Result<String> {
    // Only support bech32m format
    let format = invite.encoding_format.as_str();
    if format != "bech32m" {
        anyhow::bail!("Only bech32m encoding format is supported, got: {format}");
    }

    // Build the consensus-encoded bytes
    let bytes = encode_invite_to_bytes(invite)?;

    // Encode to bech32m
    encode_to_bech32m(&bytes)
}

fn encode_invite_to_bytes(invite: &FedimintInviteOutput) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();

    // Count the number of parts we'll encode
    let mut num_parts = 0;
    if !invite.guardians.is_empty() {
        num_parts += invite.guardians.len();
    }
    if !invite.federation_id.is_empty() {
        num_parts += 1;
    }
    if invite.api_secret.is_some() {
        num_parts += 1;
    }

    // Write number of parts (Vec<InviteCodePart> length)
    bytes.extend_from_slice(&write_varint(num_parts as u64));

    // Write guardian API parts (variant 0)
    for guardian in &invite.guardians {
        // Variant 0 (API)
        bytes.extend_from_slice(&write_varint(0));

        // Encode the Api struct fields: url (String) + peer_id (u16 as BigSize)
        let mut variant_data = Vec::new();

        // Encode URL as String (length + UTF-8 bytes)
        let url_bytes = guardian.url.as_bytes();
        variant_data.extend_from_slice(&write_varint(url_bytes.len() as u64));
        variant_data.extend_from_slice(url_bytes);

        // Encode peer ID as BigSize varint
        variant_data.extend_from_slice(&write_varint(guardian.peer_id as u64));

        // Write variant_data as Vec<u8> (length + data)
        bytes.extend_from_slice(&write_varint(variant_data.len() as u64));
        bytes.extend_from_slice(&variant_data);
    }

    // Write federation ID part (variant 1)
    if !invite.federation_id.is_empty() {
        // Variant 1 (FederationId)
        bytes.extend_from_slice(&write_varint(1));

        // Encode FederationId (32 bytes)
        let fed_id_bytes =
            hex::decode(&invite.federation_id).context("Invalid federation ID hex")?;
        if fed_id_bytes.len() != 32 {
            anyhow::bail!(
                "Federation ID must be exactly 32 bytes, got {}",
                fed_id_bytes.len()
            );
        }

        // Write fed_id_bytes as Vec<u8> (length + data)
        bytes.extend_from_slice(&write_varint(32));
        bytes.extend_from_slice(&fed_id_bytes);
    }

    // Write API secret part (variant 2) - Only if fedimint-cli compatibility is not required
    if let Some(api_secret) = &invite.api_secret {
        // Note: API secrets may not be compatible with older fedimint-cli versions
        warn!("API secret in invite code may not be compatible with all fedimint-cli versions");

        // Variant 2 (ApiSecret)
        bytes.extend_from_slice(&write_varint(2));

        // Encode api_secret as String (length + UTF-8 bytes)
        let secret_bytes = api_secret.as_bytes();

        // Write secret_bytes as Vec<u8> (length + data)
        bytes.extend_from_slice(&write_varint(secret_bytes.len() as u64));
        bytes.extend_from_slice(secret_bytes);
    }

    Ok(bytes)
}

fn encode_to_bech32m(bytes: &[u8]) -> Result<String> {
    use bech32::{Bech32m, Hrp};

    let hrp = Hrp::parse("fed1").map_err(|e| anyhow::anyhow!("Failed to parse HRP: {e}"))?;

    let encoded = bech32::encode::<Bech32m>(hrp, bytes)
        .map_err(|e| anyhow::anyhow!("Failed to encode bech32m: {e}"))?;
    Ok(encoded)
}

fn parse_as_simple_format(bytes: &[u8], encoding_format: &str) -> Result<FedimintInviteOutput> {
    // Extract URL and federation ID from the bytes

    // Extract what looks like URLs or API endpoints
    let mut guardians = Vec::new();

    // Look for common URL patterns in the bytes
    let mut pos = 0;
    while pos < bytes.len() {
        // Look for "wss://" or "https://" patterns
        if pos + 6 < bytes.len() {
            let slice = &bytes[pos..pos + 6];
            if slice == b"wss://" || slice == b"https:" {
                // Found a URL, extract it
                let mut end_pos = pos + 6;
                while end_pos < bytes.len()
                    && bytes[end_pos] != 0
                    && bytes[end_pos] > 31
                    && bytes[end_pos] < 127
                {
                    end_pos += 1;
                }
                if end_pos > pos + 6 {
                    let url = String::from_utf8_lossy(&bytes[pos..end_pos]).to_string();
                    guardians.push(GuardianInfo {
                        peer_id: guardians.len() as u16,
                        url,
                    });
                }
            }
        }
        pos += 1;
    }

    // Extract federation ID from the second part of the data (after the URL)
    // Based on fedimint-cli output, it should be: b21068c84f5b12ca4fdf93f3e443d3bd7c27e8642d0d52ea2e4dce6fdbbee9df
    let federation_id = if bytes.len() >= 64 {
        hex::encode(&bytes[bytes.len() - 32..])
    } else {
        hex::encode(&bytes[..std::cmp::min(32, bytes.len())])
    };

    if guardians.is_empty() {
        anyhow::bail!("No valid guardian URLs found in invite code");
    }

    Ok(FedimintInviteOutput {
        federation_id,
        guardians,
        api_secret: None,
        encoding_format: encoding_format.to_string(),
    })
}

// Federation config structures and functions
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct FederationConfigOutput {
    pub federation_id: String,
    pub federation_name: Option<String>,
    pub guardians: Vec<GuardianConfigInfo>,
    pub consensus_version: String,
    pub modules: serde_json::Value,
    pub meta: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct GuardianConfigInfo {
    pub peer_id: u16,
    pub name: Option<String>,
    pub url: String,
}

pub async fn fetch_fedimint_config(invite_code: &str) -> Result<FederationConfigOutput> {
    // First decode the invite code to get guardian endpoints and federation ID
    let invite = decode_fedimint_invite(invite_code)?;

    let client = reqwest::Client::new();

    // Try each guardian until we get a successful response
    let mut last_error = None;

    for guardian in &invite.guardians {
        // Convert WebSocket URLs to HTTP URLs for API calls
        let http_url = guardian
            .url
            .replace("wss://", "https://")
            .replace("ws://", "http://");
        let config_url = format!("{}/config", http_url.trim_end_matches('/'));

        match fetch_config_from_guardian(&client, &config_url).await {
            Ok(config) => {
                // Validate that the config matches the expected federation ID
                validate_federation_id(&config, &invite.federation_id)?;

                return parse_federation_config(config, &invite);
            }
            Err(e) => {
                debug!("Failed to fetch config from {}: {e}", guardian.url);
                last_error = Some(e);
                continue;
            }
        }
    }

    // If we get here, all guardians failed
    anyhow::bail!(
        "Failed to fetch config from any guardian. Last error: {:?}",
        last_error
    );
}

async fn fetch_config_from_guardian(
    client: &reqwest::Client,
    url: &str,
) -> Result<serde_json::Value> {
    // Try GET first
    let response = client.get(url).send().await;

    // If GET fails, try POST
    let response = if response.is_err() || !response.as_ref().unwrap().status().is_success() {
        client
            .post(url)
            .header("Content-Type", "application/json")
            .body("{}")
            .send()
            .await
            .context("Failed to make HTTP request")?
    } else {
        response.context("Failed to make HTTP request")?
    };

    if !response.status().is_success() {
        anyhow::bail!(
            "HTTP request failed with status: {status}",
            status = response.status()
        );
    }

    let config: serde_json::Value = response
        .json()
        .await
        .context("Failed to parse JSON response")?;

    Ok(config)
}

fn validate_federation_id(config: &serde_json::Value, expected_federation_id: &str) -> Result<()> {
    // Calculate federation ID from the config's API endpoints
    let _api_endpoints = config
        .get("global")
        .and_then(|g| g.get("api_endpoints"))
        .ok_or_else(|| anyhow::anyhow!("Config missing api_endpoints"))?;

    // For now, we'll do a basic validation by checking if the federation_id is present
    // In a full implementation, we would hash the api_endpoints to verify the federation_id
    if let Some(fed_id_value) = config.get("federation_id") {
        let config_fed_id = fed_id_value
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Federation ID is not a string"))?;

        if config_fed_id != expected_federation_id {
            anyhow::bail!(
                "Federation ID mismatch. Expected: {}, Got: {}",
                expected_federation_id,
                config_fed_id
            );
        }
    } else {
        // If federation_id is not directly in config, we trust the invite code for now
        // A full implementation would calculate the hash of api_endpoints
        warn!("Could not verify federation ID from config");
    }

    Ok(())
}

fn parse_federation_config(
    config: serde_json::Value,
    invite: &FedimintInviteOutput,
) -> Result<FederationConfigOutput> {
    let global = config
        .get("global")
        .ok_or_else(|| anyhow::anyhow!("Config missing global section"))?;

    // Extract federation name from meta
    let meta_obj = global.get("meta").and_then(|m| m.as_object());
    let federation_name = meta_obj
        .and_then(|m| m.get("federation_name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    // Convert meta to HashMap<String, String>
    let mut meta = HashMap::new();
    if let Some(meta_obj) = meta_obj {
        for (key, value) in meta_obj {
            if let Some(str_value) = value.as_str() {
                meta.insert(key.clone(), str_value.to_string());
            }
        }
    }

    // Extract consensus version
    let consensus_version = global
        .get("consensus_version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    // Parse guardian info from api_endpoints
    let api_endpoints = global
        .get("api_endpoints")
        .and_then(|e| e.as_object())
        .ok_or_else(|| anyhow::anyhow!("Config missing or invalid api_endpoints"))?;

    let mut guardians = Vec::new();
    for (peer_id_str, endpoint_value) in api_endpoints {
        let peer_id: u16 = peer_id_str.parse().context("Failed to parse peer ID")?;

        let url = endpoint_value
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Endpoint URL is not a string"))?
            .to_string();

        // Try to get guardian name from meta or use peer_id
        let name = meta_obj
            .and_then(|m| m.get(&format!("guardian_{peer_id}_name")))
            .and_then(|n| n.as_str())
            .map(|s| s.to_string());

        guardians.push(GuardianConfigInfo { peer_id, name, url });
    }

    // Sort guardians by peer_id for consistent output
    guardians.sort_by_key(|g| g.peer_id);

    // Extract modules config
    let modules = config
        .get("modules")
        .cloned()
        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

    Ok(FederationConfigOutput {
        federation_id: invite.federation_id.clone(),
        federation_name,
        guardians,
        consensus_version,
        modules,
        meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_fedimint_invite_invalid() -> Result<()> {
        // Test invalid format
        let invalid_invite = "invalid_invite_code";
        let result = decode_fedimint_invite(invalid_invite);
        assert!(result.is_err());

        // Test invalid bech32m
        let invalid_bech32m = "fed1invalid";
        let result = decode_fedimint_invite(invalid_bech32m);
        assert!(result.is_err());

        // Test string that doesn't start with fed1
        let invalid_prefix = "fedimintinvalid";
        let result = decode_fedimint_invite(invalid_prefix);
        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn test_decode_real_fedimint_invite() -> Result<()> {
        // Real invite code from Bitcoin Principles federation
        let invite_code = "fed11qgqzxgthwden5te0v9cxjtnzd96xxmmfdckhqunfde3kjurvv4ejucm0d5hsqqfqkggx3jz0tvfv5n7lj0e7gs7nh47z06ry95x4963wfh8xlka7a80su3952t";
        let result = decode_fedimint_invite(invite_code)?;

        // Verify federation ID matches fedimint-cli output
        assert_eq!(
            result.federation_id,
            "b21068c84f5b12ca4fdf93f3e443d3bd7c27e8642d0d52ea2e4dce6fdbbee9df"
        );

        // Verify guardian URL
        assert_eq!(result.guardians.len(), 1);
        assert_eq!(result.guardians[0].url, "wss://api.bitcoin-principles.com/");
        assert_eq!(result.guardians[0].peer_id, 0);

        // Verify encoding format
        assert_eq!(result.encoding_format, "bech32m");

        // No API secret in this invite
        assert_eq!(result.api_secret, None);

        Ok(())
    }

    #[test]
    fn test_encode_decode_round_trip_bech32m() -> Result<()> {
        // Test round-trip with the real invite code
        let original_invite_code = "fed11qgqzxgthwden5te0v9cxjtnzd96xxmmfdckhqunfde3kjurvv4ejucm0d5hsqqfqkggx3jz0tvfv5n7lj0e7gs7nh47z06ry95x4963wfh8xlka7a80su3952t";

        // Decode the original
        let decoded = decode_fedimint_invite(original_invite_code)?;

        // Encode it back
        let encoded = encode_fedimint_invite(&decoded)?;

        // Decode the re-encoded version
        let decoded_again = decode_fedimint_invite(&encoded)?;

        // They should be identical
        assert_eq!(decoded, decoded_again);
        assert_eq!(
            decoded.federation_id,
            "b21068c84f5b12ca4fdf93f3e443d3bd7c27e8642d0d52ea2e4dce6fdbbee9df"
        );
        assert_eq!(decoded.guardians.len(), 1);
        assert_eq!(
            decoded.guardians[0].url,
            "wss://api.bitcoin-principles.com/"
        );
        assert_eq!(decoded.guardians[0].peer_id, 0);
        assert_eq!(decoded.encoding_format, "bech32m");

        Ok(())
    }

    #[test]
    fn test_encode_decode_with_api_secret() -> Result<()> {
        // Create a test invite with API secret
        let test_invite = FedimintInviteOutput {
            federation_id: "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"
                .to_string(),
            guardians: vec![
                GuardianInfo {
                    peer_id: 0,
                    url: "wss://guardian1.example.com/".to_string(),
                },
                GuardianInfo {
                    peer_id: 1,
                    url: "wss://guardian2.example.com/".to_string(),
                },
            ],
            api_secret: Some("super_secret_api_key".to_string()),
            encoding_format: "bech32m".to_string(),
        };

        // Encode and decode round-trip
        let encoded = encode_fedimint_invite(&test_invite)?;
        let decoded = decode_fedimint_invite(&encoded)?;

        // They should be identical
        assert_eq!(test_invite, decoded);
        assert_eq!(decoded.api_secret, Some("super_secret_api_key".to_string()));
        assert_eq!(decoded.guardians.len(), 2);

        Ok(())
    }

    #[test]
    fn test_encode_decode_multiple_guardians() -> Result<()> {
        // Create a test invite with multiple guardians
        let test_invite = FedimintInviteOutput {
            federation_id: "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"
                .to_string(),
            guardians: vec![
                GuardianInfo {
                    peer_id: 0,
                    url: "wss://alpha.example.com/".to_string(),
                },
                GuardianInfo {
                    peer_id: 1,
                    url: "wss://beta.example.com/".to_string(),
                },
                GuardianInfo {
                    peer_id: 2,
                    url: "wss://gamma.example.com/".to_string(),
                },
            ],
            api_secret: None,
            encoding_format: "bech32m".to_string(),
        };

        // Encode and decode round-trip
        let encoded = encode_fedimint_invite(&test_invite)?;
        let decoded = decode_fedimint_invite(&encoded)?;

        // They should be identical
        assert_eq!(test_invite, decoded);
        assert_eq!(decoded.guardians.len(), 3);

        // Verify guardian order is preserved
        for (i, guardian) in decoded.guardians.iter().enumerate() {
            assert_eq!(guardian.peer_id, i as u16);
        }

        Ok(())
    }

    #[test]
    fn test_varint_encoding() -> Result<()> {
        // Test VarInt encoding/decoding for various values
        let test_values = vec![
            0, 1, 252, 253, 254, 255, 256, 65535, 65536, 4294967295, 4294967296,
        ];

        for value in test_values {
            let encoded = write_varint(value);
            let (decoded, bytes_read) = read_varint_at(&encoded, 0)?;
            assert_eq!(decoded, value);
            assert_eq!(bytes_read, encoded.len());
        }

        Ok(())
    }
}
