pub mod bdk_wallet;
pub mod bitcoin_rpc;
pub mod decoder;
#[cfg(feature = "frozenkrill")]
pub mod frozenkrill;
#[cfg(feature = "smartcards")]
pub mod satscard;
#[cfg(feature = "smartcards")]
pub mod tapsigner;
#[cfg(feature = "trezor")]
pub mod slip132;
#[cfg(feature = "trezor")]
pub mod trezor;

// Hardware wallet common trait
#[cfg(feature = "coldcard")]
pub mod coldcard;
pub mod hardware_wallet;

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
    create_funded_psbt_bdk, create_psbt_bdk, get_utxo_summary, list_utxos_bdk, move_utxos_bdk,
    scan_and_list_utxos_bitcoind, scan_and_list_utxos_electrum, scan_and_list_utxos_esplora,
    BdkPsbtResponse, BdkUtxo, BdkUtxoSummary,
};

// Re-export bitcoin types needed by CLI
pub use bitcoin::{self, Network};

// Re-export fedimint functionality
pub use fedimint_lite;

// Re-export frozenkrill functionality
#[cfg(feature = "frozenkrill")]
pub use frozenkrill::FrozenkrillWallet;

// Re-export coldcard functionality
#[cfg(feature = "coldcard")]
pub use coldcard::{
    export_psbt_to_coldcard, generate_coldcard_address, sign_psbt_with_coldcard,
    ColdcardAddressOutput, ColdcardSignOutput, ColdcardWallet,
};

// Re-export trezor functionality
#[cfg(feature = "trezor")]
pub use trezor::{
    generate_trezor_address, sign_psbt_with_trezor, TrezorAddressOutput, TrezorSignOutput,
    TrezorWallet,
};
