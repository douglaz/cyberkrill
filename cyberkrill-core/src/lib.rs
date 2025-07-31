pub mod bdk_wallet;
pub mod bitcoin_rpc;
pub mod decoder;
#[cfg(feature = "smartcards")]
pub mod satscard;
#[cfg(feature = "smartcards")]
pub mod tapsigner;

// Re-export main functionality for easier access
pub use decoder::{
    decode_invoice, decode_lnurl, generate_invoice_from_address, InvoiceOutput, LnurlOutput,
};

#[cfg(feature = "smartcards")]
pub use satscard::{generate_satscard_address, SatscardAddressOutput, SatscardInfo};

#[cfg(feature = "smartcards")]
pub use tapsigner::{
    generate_tapsigner_address, initialize_tapsigner, TapsignerAddressOutput, TapsignerInitOutput,
};

pub use bitcoin_rpc::{AmountInput, BitcoinRpcClient};

pub use bdk_wallet::{
    get_utxo_summary, list_utxos_bdk, scan_and_list_utxos_bitcoind, scan_and_list_utxos_electrum,
    BdkUtxo, BdkUtxoSummary,
};

// Re-export bitcoin types needed by CLI
pub use bitcoin::Network;

// Re-export fedimint functionality
pub use fedimint_lite;
