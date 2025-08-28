pub mod bdk_wallet;
pub mod bitcoin_rpc;
pub mod dca_report;
pub mod decoder;
#[cfg(feature = "frozenkrill")]
pub mod frozenkrill;
#[cfg(feature = "smartcards")]
pub mod satscard;
#[cfg(feature = "trezor")]
pub mod slip132;
#[cfg(feature = "smartcards")]
pub mod tapsigner;
#[cfg(feature = "trezor")]
pub mod trezor;

// Hardware wallet common trait
#[cfg(feature = "coldcard")]
pub mod coldcard;
pub mod hardware_wallet;
#[cfg(feature = "jade")]
pub mod jade;

// Re-export main functionality for easier access
pub use decoder::{
    GeneratedInvoiceOutput, InvoiceOutput, LnurlOutput, decode_invoice, decode_lnurl,
    generate_invoice_from_address,
};

#[cfg(feature = "smartcards")]
pub use satscard::{SatscardAddressOutput, SatscardInfo, generate_satscard_address};

#[cfg(feature = "smartcards")]
pub use tapsigner::{
    TapsignerAddressOutput, TapsignerInitOutput, generate_tapsigner_address, initialize_tapsigner,
};

pub use bitcoin_rpc::{AmountInput, BitcoinRpcClient};

pub use bdk_wallet::{
    BdkPsbtResponse, BdkUtxo, BdkUtxoSummary, create_funded_psbt_bdk, create_psbt_bdk,
    get_utxo_summary, list_utxos_bdk, move_utxos_bdk, scan_and_list_utxos_bitcoind,
    scan_and_list_utxos_electrum, scan_and_list_utxos_esplora,
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
    ColdcardAddressOutput, ColdcardSignOutput, ColdcardWallet, export_psbt_to_coldcard,
    generate_coldcard_address, sign_psbt_with_coldcard,
};

// Re-export trezor functionality
#[cfg(feature = "trezor")]
pub use trezor::{
    TrezorAddressOutput, TrezorSignOutput, TrezorWallet, generate_trezor_address,
    sign_psbt_with_trezor,
};

// Re-export jade functionality
#[cfg(feature = "jade")]
pub use jade::{
    JadeAddressResult, JadeSignedPsbtResult, JadeXpubResult, generate_jade_address,
    generate_jade_xpub, sign_psbt_with_jade,
};

// Re-export DCA report functionality
pub use dca_report::{Backend, DcaMetrics, DcaReport, DcaUtxo, generate_dca_report};
