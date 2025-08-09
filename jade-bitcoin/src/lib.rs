//! Bitcoin-focused Rust client for Blockstream Jade hardware wallet
//!
//! This crate provides a clean, Bitcoin-only interface for interacting with
//! Jade hardware wallets. It handles serial communication, CBOR protocol,
//! and provides simple methods for common Bitcoin operations.
//!
//! # Examples
//!
//! ```no_run
//! use jade_bitcoin::{JadeClient, Network};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to Jade device
//! let mut jade = JadeClient::connect()?;
//!
//! // Unlock the device
//! jade.unlock(Network::Bitcoin)?;
//!
//! // Get a Bitcoin address
//! let address = jade.get_address("m/84'/0'/0'/0/0", Network::Bitcoin)?;
//! println!("Address: {}", address);
//!
//! // Get extended public key
//! let xpub = jade.get_xpub("m/84'/0'/0'")?;
//! println!("xpub: {}", xpub);
//! # Ok(())
//! # }
//! ```

mod client;
mod error;
mod messages;
mod protocol;
mod serial;
mod types;

pub use client::JadeClient;
pub use error::{Error, Result};
pub use types::{Network, VersionInfo};

// Re-export commonly used types
pub use bitcoin::psbt::Psbt;
