//! CBOR message structures for Jade protocol

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Request message to Jade
#[derive(Debug, Serialize)]
pub struct Request {
    pub id: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl Request {
    pub fn new(id: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            method: method.into(),
            params: None,
        }
    }

    pub fn with_params(id: impl Into<String>, method: impl Into<String>, params: Value) -> Self {
        Self {
            id: id.into(),
            method: method.into(),
            params: Some(params),
        }
    }
}

/// Response message from Jade
#[derive(Debug, Deserialize)]
pub struct Response {
    pub id: String,
    #[serde(flatten)]
    pub body: ResponseBody,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ResponseBody {
    Result { result: Value },
    Error { error: ErrorResponse },
}

#[derive(Debug, Deserialize)]
pub struct ErrorResponse {
    pub code: i32,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}

/// Methods supported by Jade
pub mod methods {
    pub const GET_VERSION_INFO: &str = "get_version_info";
    pub const AUTH_USER: &str = "auth_user";
    pub const LOGOUT: &str = "logout";
    pub const GET_XPUB: &str = "get_xpub";
    pub const GET_RECEIVE_ADDRESS: &str = "get_receive_address";
    pub const SIGN_PSBT: &str = "sign_psbt";
    pub const SIGN_MESSAGE: &str = "sign_message";
    pub const GET_MASTER_BLINDING_KEY: &str = "get_master_blinding_key";
    pub const GET_SHARED_NONCE: &str = "get_shared_nonce";
    pub const GET_COMMITMENTS: &str = "get_commitments";
    pub const GET_SIGNATURE: &str = "get_signature";
    pub const HTTP_REQUEST: &str = "http_request";
}

/// Error codes from Jade
pub mod error_codes {
    pub const USER_CANCELLED: i32 = -32000;
    pub const INVALID_REQUEST: i32 = -32600;
    pub const METHOD_NOT_FOUND: i32 = -32601;
    pub const INVALID_PARAMS: i32 = -32602;
    pub const INTERNAL_ERROR: i32 = -32603;
    pub const HW_LOCKED: i32 = -32001;
    pub const NETWORK_MISMATCH: i32 = -32002;
    pub const USER_DECLINED: i32 = -32003;
}
