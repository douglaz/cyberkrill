//! Error types for jade-bitcoin

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Serial port error: {0}")]
    SerialPort(#[from] tokio_serial::Error),

    #[error("CBOR encoding error: {0}")]
    CborEncode(#[from] serde_cbor::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("No Jade device found")]
    DeviceNotFound,

    #[error("Jade device error: code={code}, message={message}")]
    JadeError { code: i32, message: String },

    #[error("Invalid response from Jade")]
    InvalidResponse,

    #[error("User cancelled operation on device")]
    UserCancelled,

    #[error("Device is locked")]
    DeviceLocked,

    #[error("Invalid derivation path: {0}")]
    InvalidPath(String),

    #[error("Network mismatch: device is on {device}, requested {requested}")]
    NetworkMismatch { device: String, requested: String },

    #[error("Timeout waiting for device response")]
    Timeout,

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Bitcoin error: {0}")]
    Bitcoin(#[from] bitcoin::address::ParseError),

    #[error("Invalid PSBT")]
    InvalidPsbt,

    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
