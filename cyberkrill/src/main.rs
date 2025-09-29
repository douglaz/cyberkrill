use anyhow::{Context, bail};
use clap::{Parser, Subcommand};
use cyberkrill_core::AmountInput;
use std::io::{BufWriter, Read, Write};
use std::path::Path;
use std::str::FromStr;

mod mcp_server;

const DEFAULT_BITCOIN_RPC_URL: &str = "http://127.0.0.1:8332";

#[derive(Parser)]
#[command(name = "cyberkrill")]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(about = "A CLI toolkit for Bitcoin and Lightning Network operations")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    // Lightning Network Operations (ln-*)
    #[command(name = "ln-decode-invoice", about = "Decode BOLT11 Lightning invoice")]
    LnDecodeInvoice(DecodeInvoiceArgs),
    #[command(name = "ln-decode-lnurl", about = "Decode LNURL string")]
    LnDecodeLnurl(DecodeLnurlArgs),
    #[command(
        name = "ln-encode-invoice",
        about = "Encode BOLT11 Lightning invoice from JSON data"
    )]
    LnEncodeInvoice(EncodeInvoiceArgs),
    #[command(
        name = "ln-generate-invoice",
        about = "Generate Lightning invoice from Lightning address using LNURL-pay protocol"
    )]
    LnGenerateInvoice(GenerateInvoiceArgs),

    // Fedimint Operations (fm-*)
    #[command(name = "fm-decode-invite", about = "Decode Fedimint invite code")]
    FmDecodeInvite(DecodeFedimintInviteArgs),
    #[command(
        name = "fm-encode-invite",
        about = "Encode Fedimint invite code from JSON"
    )]
    FmEncodeInvite(EncodeFedimintInviteArgs),
    #[command(
        name = "fm-fetch-config",
        about = "Fetch Fedimint federation configuration"
    )]
    FmFetchConfig(FedimintConfigArgs),

    // Hardware Wallet Operations (hw-*)
    #[cfg(feature = "smartcards")]
    #[command(
        name = "hw-tapsigner-address",
        about = "Generate Bitcoin address from Tapsigner"
    )]
    HwTapsignerAddress(TapsignerAddressArgs),
    #[cfg(feature = "smartcards")]
    #[command(
        name = "hw-tapsigner-init",
        about = "Initialize Tapsigner (one-time setup)"
    )]
    HwTapsignerInit(TapsignerInitArgs),
    #[cfg(feature = "smartcards")]
    #[command(
        name = "hw-satscard-address",
        about = "Generate Bitcoin address from Satscard"
    )]
    HwSatscardAddress(SatscardAddressArgs),

    // Coldcard Hardware Wallet Operations
    #[cfg(feature = "coldcard")]
    #[command(
        name = "hw-coldcard-address",
        about = "Generate Bitcoin address from Coldcard"
    )]
    HwColdcardAddress(ColdcardAddressArgs),
    #[cfg(feature = "coldcard")]
    #[command(name = "hw-coldcard-sign-psbt", about = "Sign PSBT with Coldcard")]
    HwColdcardSignPsbt(ColdcardSignPsbtArgs),
    #[cfg(feature = "coldcard")]
    #[command(
        name = "hw-coldcard-export-psbt",
        about = "Export PSBT to Coldcard SD card"
    )]
    HwColdcardExportPsbt(ColdcardExportPsbtArgs),
    #[cfg(feature = "trezor")]
    #[command(
        name = "hw-trezor-address",
        about = "Generate Bitcoin address from Trezor"
    )]
    HwTrezorAddress(TrezorAddressArgs),
    #[cfg(feature = "trezor")]
    #[command(name = "hw-trezor-sign-psbt", about = "Sign PSBT with Trezor")]
    HwTrezorSignPsbt(TrezorSignPsbtArgs),

    // Jade Hardware Wallet Operations
    #[cfg(feature = "jade")]
    #[command(name = "hw-jade-address", about = "Generate Bitcoin address from Jade")]
    HwJadeAddress(JadeAddressArgs),
    #[cfg(feature = "jade")]
    #[command(name = "hw-jade-xpub", about = "Get extended public key from Jade")]
    HwJadeXpub(JadeXpubArgs),
    #[cfg(feature = "jade")]
    #[command(name = "hw-jade-sign-psbt", about = "Sign PSBT with Jade")]
    HwJadeSignPsbt(JadeSignPsbtArgs),

    // Bitcoin Onchain Operations (onchain-*)
    #[command(
        name = "onchain-list-utxos",
        about = "List UTXOs for addresses or descriptors"
    )]
    OnchainListUtxos(ListUtxosArgs),
    #[command(
        name = "onchain-create-psbt",
        about = "Create PSBT with manual input/output specification (you specify exact inputs, outputs, and change)"
    )]
    OnchainCreatePsbt(CreatePsbtArgs),
    #[command(
        name = "onchain-create-funded-psbt",
        about = "Create funded PSBT with automatic input selection and change output (wallet handles coin selection)"
    )]
    OnchainCreateFundedPsbt(CreateFundedPsbtArgs),
    #[command(
        name = "onchain-move-utxos",
        about = "Consolidate/move UTXOs to a single destination address (output = total inputs - fee)"
    )]
    OnchainMoveUtxos(MoveUtxosArgs),
    #[command(
        name = "onchain-decode-psbt",
        about = "Decode a PSBT (Partially Signed Bitcoin Transaction)"
    )]
    OnchainDecodePsbt(DecodePsbtArgs),
    #[command(
        name = "onchain-dca-report",
        about = "Generate DCA (Dollar Cost Averaging) report for UTXOs"
    )]
    OnchainDcaReport(DcaReportArgs),

    // Utility Commands
    #[command(name = "version", about = "Print version information")]
    Version,
    #[command(name = "generate-mnemonic", about = "Generate a BIP39 mnemonic phrase")]
    GenerateMnemonic(GenerateMnemonicArgs),

    // MCP Server
    #[command(
        name = "mcp-server",
        about = "Start MCP server for AI assistant integration"
    )]
    McpServer(McpServerArgs),
}

// Lightning Network Args

#[derive(clap::Args, Debug)]
struct DecodeInvoiceArgs {
    input: Option<String>,
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct DecodeLnurlArgs {
    input: Option<String>,
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct EncodeInvoiceArgs {
    /// Input JSON file path (or - for stdin)
    input: Option<String>,
    /// Private key in hex format for signing the invoice
    #[clap(short = 'k', long)]
    private_key: String,
    /// Output file path for the encoded invoice
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct DecodeFedimintInviteArgs {
    input: Option<String>,
    #[clap(short, long)]
    output: Option<String>,
}

// Fedimint Args

#[derive(clap::Args, Debug)]
struct EncodeFedimintInviteArgs {
    /// Input JSON file path (or - for stdin)
    input: String,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
    /// Skip API secrets for fedimint-cli compatibility
    #[clap(long)]
    skip_api_secret: bool,
}

#[derive(clap::Args, Debug)]
struct FedimintConfigArgs {
    /// Fedimint invite code
    invite_code: String,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct GenerateInvoiceArgs {
    /// Lightning address (e.g., user@domain.com)
    address: String,
    /// Amount to request. Supports multiple formats:
    /// - Plain number (interpreted as BTC): "0.5" or "1.5"
    /// - BTC with suffix: "0.5btc" or "1.5BTC"
    /// - Satoshis: "50000000sats" or "100000sat"
    /// - Millisatoshis: "50000000000msats" or "100000000msat"
    amount: String,
    /// Optional comment for the payment request
    #[clap(short, long)]
    comment: Option<String>,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

// MCP Server Args

#[derive(clap::Args, Debug)]
struct GenerateMnemonicArgs {
    /// Word count (12, 15, 18, 21, or 24)
    #[clap(short, long, default_value = "24")]
    words: u32,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct McpServerArgs {
    /// Transport type (stdio or sse)
    #[clap(short, long, default_value = "stdio")]
    transport: String,
    /// Host address for SSE transport
    #[clap(long, default_value = "127.0.0.1")]
    host: String,
    /// Port for SSE transport
    #[clap(short, long, default_value_t = 8080)]
    port: u16,
}

// Hardware Wallet Args

#[cfg(feature = "smartcards")]
#[derive(clap::Args, Debug)]
struct TapsignerAddressArgs {
    /// Derivation path (e.g., m/84'/0'/0'/0/0)
    #[clap(short, long, default_value = "m/84'/0'/0'/0/0")]
    path: String,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[cfg(feature = "smartcards")]
#[derive(clap::Args, Debug)]
struct TapsignerInitArgs {
    /// Optional custom chain code (64 hex chars = 32 bytes). If not provided, will generate random.
    #[clap(long)]
    chain_code: Option<String>,
    /// Output file path for initialization details
    #[clap(short, long)]
    output: Option<String>,
}

// Bitcoin RPC Args

#[cfg(feature = "smartcards")]
#[derive(clap::Args, Debug)]
struct SatscardAddressArgs {
    /// Slot number (0-9, default: current active slot)
    #[clap(short, long)]
    slot: Option<u8>,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

// Coldcard Args

#[cfg(feature = "coldcard")]
#[derive(clap::Args, Debug)]
struct ColdcardAddressArgs {
    /// Derivation path (e.g., m/84'/0'/0'/0/0)
    #[clap(short, long, default_value = "m/84'/0'/0'/0/0")]
    path: String,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[cfg(feature = "coldcard")]
#[derive(clap::Args, Debug)]
struct ColdcardSignPsbtArgs {
    /// PSBT file path or base64/hex string
    input: String,
    /// Output file path for signed PSBT
    #[clap(short, long)]
    output: Option<String>,
    /// Also save raw PSBT binary to this file
    #[clap(long)]
    psbt_output: Option<String>,
}

#[cfg(feature = "coldcard")]
#[derive(clap::Args, Debug)]
struct ColdcardExportPsbtArgs {
    /// PSBT file path or base64/hex string
    input: String,
    /// Filename on SD card (e.g., "tx-to-sign.psbt")
    #[clap(short, long, default_value = "unsigned.psbt")]
    filename: String,
}

#[cfg(feature = "trezor")]
#[derive(clap::Args, Debug)]
struct TrezorAddressArgs {
    /// Derivation path (e.g., m/84'/0'/0'/0/0)
    #[clap(short, long, default_value = "m/84'/0'/0'/0/0")]
    path: String,
    /// Network (bitcoin, testnet, signet, regtest)
    #[clap(short = 'n', long, default_value = "bitcoin")]
    network: String,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[cfg(feature = "trezor")]
#[derive(clap::Args, Debug)]
struct TrezorSignPsbtArgs {
    /// PSBT file path or base64/hex string
    input: String,
    /// Network (bitcoin, testnet, signet, regtest)
    #[clap(short = 'n', long, default_value = "bitcoin")]
    network: String,
    /// Output file path for signed PSBT
    #[clap(short, long)]
    output: Option<String>,
    /// Also save raw PSBT binary to this file
    #[clap(long)]
    psbt_output: Option<String>,
}

// Jade Hardware Wallet Args

#[cfg(feature = "jade")]
#[derive(clap::Args, Debug)]
struct JadeAddressArgs {
    /// Derivation path (e.g., m/84'/0'/0'/0/0)
    #[clap(short, long, default_value = "m/84'/0'/0'/0/0")]
    path: String,
    /// Network (bitcoin, testnet, signet, regtest)
    #[clap(short = 'n', long, default_value = "bitcoin")]
    network: String,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[cfg(feature = "jade")]
#[derive(clap::Args, Debug)]
struct JadeXpubArgs {
    /// Derivation path (e.g., m/84'/0'/0')
    #[clap(short, long, default_value = "m/84'/0'/0'")]
    path: String,
    /// Network (bitcoin, testnet, signet, regtest)
    #[clap(short = 'n', long, default_value = "bitcoin")]
    network: String,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[cfg(feature = "jade")]
#[derive(clap::Args, Debug)]
struct JadeSignPsbtArgs {
    /// PSBT file path or base64/hex string
    input: String,
    /// Network (bitcoin, testnet, signet, regtest)
    #[clap(short = 'n', long, default_value = "bitcoin")]
    network: String,
    /// Output file path for signed PSBT
    #[clap(short, long)]
    output: Option<String>,
    /// Also save raw PSBT binary to this file
    #[clap(long)]
    psbt_output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct ListUtxosArgs {
    /// frozenkrill wallet export file to list UTXOs from
    #[cfg(feature = "frozenkrill")]
    #[clap(long, conflicts_with_all = ["descriptor", "addresses"])]
    wallet_file: Option<std::path::PathBuf>,
    /// Output descriptor to scan for UTXOs (required when using BDK backends)
    #[cfg_attr(feature = "frozenkrill", clap(long, conflicts_with_all = ["addresses", "wallet_file"]))]
    #[cfg_attr(not(feature = "frozenkrill"), clap(long, conflicts_with = "addresses"))]
    descriptor: Option<String>,
    /// Comma-separated list of addresses to list UTXOs for (only for Bitcoin Core RPC)
    #[cfg_attr(feature = "frozenkrill", clap(long, conflicts_with_all = ["descriptor", "wallet_file"]))]
    #[cfg_attr(
        not(feature = "frozenkrill"),
        clap(long, conflicts_with = "descriptor")
    )]
    addresses: Option<String>,

    // Backend selection options (mutually exclusive)
    /// Electrum server URL (e.g., ssl://electrum.blockstream.info:50002)
    #[clap(long, conflicts_with_all = ["esplora", "bitcoin_dir", "rpc_url", "rpc_user", "rpc_password"])]
    electrum: Option<String>,
    /// Esplora server URL (e.g., https://blockstream.info/api)
    #[clap(long, conflicts_with_all = ["electrum", "bitcoin_dir", "rpc_url", "rpc_user", "rpc_password"])]
    esplora: Option<String>,

    // Bitcoin Core RPC options (default backend)
    /// Bitcoin Core RPC URL (default: http://127.0.0.1:8332)
    #[clap(long, default_value = DEFAULT_BITCOIN_RPC_URL, conflicts_with_all = ["electrum", "esplora"])]
    rpc_url: String,
    /// Bitcoin directory path (for cookie authentication, default: ~/.bitcoin)
    #[clap(long, conflicts_with_all = ["electrum", "esplora"])]
    bitcoin_dir: Option<String>,
    /// RPC username (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with_all = ["bitcoin_dir", "electrum", "esplora"])]
    rpc_user: Option<String>,
    /// RPC password (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with_all = ["bitcoin_dir", "electrum", "esplora"])]
    rpc_password: Option<String>,

    /// Bitcoin network (mainnet, testnet, signet, regtest)
    #[clap(long, default_value = "mainnet")]
    network: String,
    /// Minimum confirmations (default: 1)
    #[clap(long, default_value = "1")]
    min_conf: u32,
    /// Maximum confirmations (default: 9999999)
    #[clap(long, default_value = "9999999")]
    max_conf: u32,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct CreatePsbtArgs {
    /// frozenkrill wallet export file to use for address derivation
    #[cfg(feature = "frozenkrill")]
    #[clap(long, conflicts_with = "descriptor")]
    wallet_file: Option<std::path::PathBuf>,
    /// Output descriptor (required when using BDK backends)
    #[cfg_attr(feature = "frozenkrill", clap(long, conflicts_with = "wallet_file"))]
    #[cfg_attr(not(feature = "frozenkrill"), clap(long))]
    descriptor: Option<String>,

    // Backend selection options (mutually exclusive)
    /// Electrum server URL (e.g., ssl://electrum.blockstream.info:50002)
    #[clap(long, conflicts_with_all = ["esplora", "bitcoin_dir", "rpc_url", "rpc_user", "rpc_password"])]
    electrum: Option<String>,
    /// Esplora server URL (e.g., https://blockstream.info/api)
    #[clap(long, conflicts_with_all = ["electrum", "bitcoin_dir", "rpc_url", "rpc_user", "rpc_password"])]
    esplora: Option<String>,

    // Bitcoin Core RPC options (default backend)
    /// Bitcoin Core RPC URL (default: http://127.0.0.1:8332)
    #[clap(long, default_value = DEFAULT_BITCOIN_RPC_URL, conflicts_with_all = ["electrum", "esplora"])]
    rpc_url: String,
    /// Bitcoin directory path (for cookie authentication, default: ~/.bitcoin)
    #[clap(long, conflicts_with_all = ["electrum", "esplora"])]
    bitcoin_dir: Option<String>,
    /// RPC username (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with_all = ["bitcoin_dir", "electrum", "esplora"])]
    rpc_user: Option<String>,
    /// RPC password (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with_all = ["bitcoin_dir", "electrum", "esplora"])]
    rpc_password: Option<String>,

    /// Bitcoin network (mainnet, testnet, signet, regtest)
    #[clap(long, default_value = "mainnet")]
    network: String,
    /// Input UTXOs in format txid:vout or output descriptors (can be specified multiple times)
    /// Examples: --inputs txid1:0 --inputs txid2:1 or --inputs "wpkh([fingerprint/84'/0'/0']xpub...)"
    #[clap(long, required = true)]
    inputs: Vec<String>,
    /// Output addresses and amounts (comma-separated).
    /// Format: address:amount where amount supports:
    /// - Plain number (BTC): "0.5"
    /// - BTC with suffix: "0.5btc"
    /// - Satoshis: "50000000sats"
    /// - Millisatoshis: "50000000000msats"
    ///   Example: "bc1qaddr1:0.5,bc1qaddr2:100000sats"
    #[clap(long, required = true)]
    outputs: String,
    /// Fee rate in sats/vB (optional, will use Bitcoin Core's default if not specified) - supports formats like '15', '20.5sats', '15btc'
    #[clap(long)]
    fee_rate: Option<AmountInput>,
    /// Output file path for JSON response
    #[clap(short, long)]
    output: Option<String>,
    /// Output file path for raw PSBT data (base64)
    #[clap(long)]
    psbt_output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct CreateFundedPsbtArgs {
    /// frozenkrill wallet export file to use for address derivation
    #[cfg(feature = "frozenkrill")]
    #[clap(long, conflicts_with = "descriptor")]
    wallet_file: Option<std::path::PathBuf>,
    /// Output descriptor (required when using BDK backends)
    #[cfg_attr(feature = "frozenkrill", clap(long, conflicts_with = "wallet_file"))]
    #[cfg_attr(not(feature = "frozenkrill"), clap(long))]
    descriptor: Option<String>,

    // Backend selection options (mutually exclusive)
    /// Electrum server URL (e.g., ssl://electrum.blockstream.info:50002)
    #[clap(long, conflicts_with_all = ["esplora", "bitcoin_dir", "rpc_url", "rpc_user", "rpc_password"])]
    electrum: Option<String>,
    /// Esplora server URL (e.g., https://blockstream.info/api)
    #[clap(long, conflicts_with_all = ["electrum", "bitcoin_dir", "rpc_url", "rpc_user", "rpc_password"])]
    esplora: Option<String>,

    // Bitcoin Core RPC options (default backend)
    /// Bitcoin Core RPC URL (default: http://127.0.0.1:8332)
    #[clap(long, default_value = DEFAULT_BITCOIN_RPC_URL, conflicts_with_all = ["electrum", "esplora"])]
    rpc_url: String,
    /// Bitcoin directory path (for cookie authentication, default: ~/.bitcoin)
    #[clap(long, conflicts_with_all = ["electrum", "esplora"])]
    bitcoin_dir: Option<String>,
    /// RPC username (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with_all = ["bitcoin_dir", "electrum", "esplora"])]
    rpc_user: Option<String>,
    /// RPC password (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with_all = ["bitcoin_dir", "electrum", "esplora"])]
    rpc_password: Option<String>,

    /// Bitcoin network (mainnet, testnet, signet, regtest)
    #[clap(long, default_value = "mainnet")]
    network: String,
    /// Input UTXOs in format txid:vout (can be specified multiple times). Leave empty for automatic input selection.
    /// Examples: --inputs txid1:0 --inputs txid2:1
    /// Note: For automatic selection from a descriptor, use --descriptor instead
    #[clap(long)]
    inputs: Vec<String>,
    /// Output addresses and amounts (comma-separated).
    /// Format: address:amount where amount supports:
    /// - Plain number (BTC): "0.5"
    /// - BTC with suffix: "0.5btc"
    /// - Satoshis: "50000000sats"
    /// - Millisatoshis: "50000000000msats"
    ///   Example: "bc1qaddr1:0.5,bc1qaddr2:100000sats"
    #[clap(long, required = true)]
    outputs: String,
    /// Confirmation target in blocks (1-1008)
    #[clap(long)]
    conf_target: Option<u32>,
    /// Fee estimation mode: UNSET, ECONOMICAL, CONSERVATIVE
    #[clap(long)]
    estimate_mode: Option<String>,
    /// Fee rate in sats/vB (overrides conf_target and estimate_mode) - supports formats like '15', '20.5sats', '15btc'
    #[clap(long)]
    fee_rate: Option<AmountInput>,
    /// Output file path for JSON response
    #[clap(short, long)]
    output: Option<String>,
    /// Output file path for raw PSBT data (base64)
    #[clap(long)]
    psbt_output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct MoveUtxosArgs {
    /// frozenkrill wallet export file to use for UTXO discovery
    #[cfg(feature = "frozenkrill")]
    #[clap(long, conflicts_with = "descriptor")]
    wallet_file: Option<std::path::PathBuf>,
    /// Output descriptor (required when using BDK backends)
    #[cfg_attr(feature = "frozenkrill", clap(long, conflicts_with = "wallet_file"))]
    #[cfg_attr(not(feature = "frozenkrill"), clap(long))]
    descriptor: Option<String>,

    // Backend selection options (mutually exclusive)
    /// Electrum server URL (e.g., ssl://electrum.blockstream.info:50002)
    #[clap(long, conflicts_with_all = ["esplora", "bitcoin_dir", "rpc_url", "rpc_user", "rpc_password"])]
    electrum: Option<String>,
    /// Esplora server URL (e.g., https://blockstream.info/api)
    #[clap(long, conflicts_with_all = ["electrum", "bitcoin_dir", "rpc_url", "rpc_user", "rpc_password"])]
    esplora: Option<String>,

    // Bitcoin Core RPC options (default backend)
    /// Bitcoin Core RPC URL (default: http://127.0.0.1:8332)
    #[clap(long, default_value = DEFAULT_BITCOIN_RPC_URL, conflicts_with_all = ["electrum", "esplora"])]
    rpc_url: String,
    /// Bitcoin directory path (for cookie authentication, default: ~/.bitcoin)
    #[clap(long, conflicts_with_all = ["electrum", "esplora"])]
    bitcoin_dir: Option<String>,
    /// RPC username (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with_all = ["bitcoin_dir", "electrum", "esplora"])]
    rpc_user: Option<String>,
    /// RPC password (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with_all = ["bitcoin_dir", "electrum", "esplora"])]
    rpc_password: Option<String>,

    /// Bitcoin network (mainnet, testnet, signet, regtest)
    #[clap(long, default_value = "mainnet")]
    network: String,
    /// Input UTXOs to consolidate in format txid:vout or output descriptors (can be specified multiple times)
    /// Examples: --inputs txid1:0 --inputs txid2:1 or --inputs "wpkh([fingerprint/84'/0'/0']xpub...)"
    #[clap(long, required = true)]
    inputs: Vec<String>,
    /// Destination address for consolidated output
    #[clap(long, required = true)]
    destination: String,
    /// Fee rate in sats/vB (conflicts with fee) - supports formats like '15', '20.5sats', '15btc'
    #[clap(long, conflicts_with = "fee")]
    fee_rate: Option<AmountInput>,
    /// Fee amount (conflicts with fee_rate) - supports formats like '1000sats', '0.00001btc', '1000'
    #[clap(long, conflicts_with = "fee_rate")]
    fee: Option<AmountInput>,
    /// Maximum amount to move (supports formats: '123sats', '0.666btc', or '0.666' for BTC)
    #[clap(long)]
    max_amount: Option<AmountInput>,
    /// Output file path for JSON response
    #[clap(short, long)]
    output: Option<String>,
    /// Output file path for raw PSBT data (base64)
    #[clap(long)]
    psbt_output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct DecodePsbtArgs {
    /// PSBT string (base64 encoded) or file path containing PSBT
    input: Option<String>,

    /// Path to output file (default: stdout)
    #[clap(short, long)]
    output: Option<String>,

    /// Network (mainnet, testnet, signet, regtest)
    #[clap(long, default_value = "mainnet")]
    network: String,
}

#[derive(clap::Args, Debug)]
struct DcaReportArgs {
    /// Output descriptor to analyze
    #[clap(long)]
    descriptor: String,

    /// Bitcoin Core data directory (for RPC backend)
    #[clap(long, value_hint = clap::ValueHint::DirPath, conflicts_with_all = &["electrum", "esplora"])]
    bitcoin_dir: Option<std::path::PathBuf>,

    /// Electrum server URL (e.g., ssl://electrum.blockstream.info:50002)
    #[clap(long, conflicts_with_all = &["bitcoin_dir", "esplora"])]
    electrum: Option<String>,

    /// Esplora server URL (e.g., https://blockstream.info/api)
    #[clap(long, conflicts_with_all = &["bitcoin_dir", "electrum"])]
    esplora: Option<String>,

    /// Fiat currency for price data
    #[clap(long, default_value = "usd")]
    currency: String,

    /// Directory for caching price data
    #[clap(long, value_hint = clap::ValueHint::DirPath)]
    cache_dir: Option<std::path::PathBuf>,

    /// Path to output file (default: stdout)
    #[clap(short, long)]
    output: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber with RUST_LOG environment variable, output to stderr
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    // Initialize rustls crypto provider for TLS connections (required for Electrum)
    if rustls::crypto::ring::default_provider()
        .install_default()
        .is_err()
    {
        bail!("Failed to initialize rustls crypto provider");
    }

    let args: Cli = Cli::parse();
    match args.command {
        // Lightning Network Operations
        Commands::LnDecodeInvoice(args) => decode_invoice(args)?,
        Commands::LnDecodeLnurl(args) => decode_lnurl(args)?,
        Commands::LnEncodeInvoice(args) => encode_invoice(args)?,
        Commands::LnGenerateInvoice(args) => generate_invoice(args).await?,

        // Fedimint Operations
        Commands::FmDecodeInvite(args) => decode_fedimint_invite(args)?,
        Commands::FmEncodeInvite(args) => encode_fedimint_invite(args)?,
        Commands::FmFetchConfig(args) => fedimint_config(args).await?,

        // Hardware Wallet Operations
        #[cfg(feature = "smartcards")]
        Commands::HwTapsignerAddress(args) => tapsigner_address(args).await?,
        #[cfg(feature = "smartcards")]
        Commands::HwTapsignerInit(args) => tapsigner_init(args).await?,
        #[cfg(feature = "smartcards")]
        Commands::HwSatscardAddress(args) => satscard_address(args).await?,

        // Coldcard Operations
        #[cfg(feature = "coldcard")]
        Commands::HwColdcardAddress(args) => coldcard_address(args).await?,
        #[cfg(feature = "coldcard")]
        Commands::HwColdcardSignPsbt(args) => coldcard_sign_psbt(args).await?,
        #[cfg(feature = "coldcard")]
        Commands::HwColdcardExportPsbt(args) => coldcard_export_psbt(args).await?,
        #[cfg(feature = "trezor")]
        Commands::HwTrezorAddress(args) => trezor_address(args).await?,
        #[cfg(feature = "trezor")]
        Commands::HwTrezorSignPsbt(args) => trezor_sign_psbt(args).await?,

        // Jade Hardware Wallet Operations
        #[cfg(feature = "jade")]
        Commands::HwJadeAddress(args) => jade_address(args).await?,
        #[cfg(feature = "jade")]
        Commands::HwJadeXpub(args) => jade_xpub(args).await?,
        #[cfg(feature = "jade")]
        Commands::HwJadeSignPsbt(args) => jade_sign_psbt(args).await?,

        // Bitcoin Onchain Operations
        Commands::OnchainListUtxos(args) => bitcoin_list_utxos(args).await?,
        Commands::OnchainCreatePsbt(args) => bitcoin_create_psbt(args).await?,
        Commands::OnchainCreateFundedPsbt(args) => bitcoin_create_funded_psbt(args).await?,
        Commands::OnchainMoveUtxos(args) => bitcoin_move_utxos(args).await?,
        Commands::OnchainDecodePsbt(args) => decode_psbt(args)?,
        Commands::OnchainDcaReport(args) => dca_report(args).await?,

        // Utility Commands
        Commands::Version => {
            // Version output should be JSON for consistency
            let version = serde_json::json!({
                "version": env!("CARGO_PKG_VERSION")
            });
            let version_str = serde_json::to_string_pretty(&version)?;
            println!("{version_str}");
        }
        Commands::GenerateMnemonic(args) => generate_mnemonic(args)?,

        // MCP Server
        Commands::McpServer(args) => mcp_server(args).await?,
    }
    Ok(())
}

fn decode_lnurl(args: DecodeLnurlArgs) -> anyhow::Result<()> {
    let input = match args.input {
        Some(input) => input,
        None => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::stdout()),
    };

    let output = cyberkrill_core::decode_lnurl(&input)?;
    serde_json::to_writer_pretty(writer, &output)?;
    Ok(())
}

fn decode_invoice(args: DecodeInvoiceArgs) -> anyhow::Result<()> {
    let input = match args.input {
        Some(input) => input,
        None => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    let writer: Box<dyn std::io::Write> = match args.output {
        Some(output) => Box::new(BufWriter::new(std::fs::File::create(output)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let output = cyberkrill_core::decode_invoice(&input)?;
    serde_json::to_writer_pretty(writer, &output)?;
    Ok(())
}

fn encode_invoice(args: EncodeInvoiceArgs) -> anyhow::Result<()> {
    use bitcoin::secp256k1::SecretKey;
    use cyberkrill_core::InvoiceOutput;

    // Read input JSON
    let json_str = match args.input.as_deref() {
        Some("-") | None => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
        Some(path) => std::fs::read_to_string(path)?,
    };

    // Parse JSON to InvoiceOutput
    let invoice_data: InvoiceOutput = serde_json::from_str(&json_str)?;

    // Parse the private key from hex
    let private_key_bytes = hex::decode(&args.private_key)
        .map_err(|e| anyhow::anyhow!("Invalid private key hex: {e}"))?;
    let private_key = SecretKey::from_slice(&private_key_bytes)
        .map_err(|e| anyhow::anyhow!("Invalid private key format: {e}"))?;

    // Encode the invoice
    let encoded_invoice = cyberkrill_core::encode_invoice(&invoice_data, &private_key)?;

    // Write output
    match args.output {
        Some(path) => std::fs::write(path, encoded_invoice)?,
        None => println!("{encoded_invoice}"),
    }

    Ok(())
}

fn decode_fedimint_invite(args: DecodeFedimintInviteArgs) -> anyhow::Result<()> {
    let input = match args.input {
        Some(input) => input,
        None => {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer.trim().to_string()
        }
    };

    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::stdout()),
    };

    let output = fedimint_lite::decode_invite(&input)?;
    serde_json::to_writer_pretty(writer, &output)?;
    Ok(())
}

async fn generate_invoice(args: GenerateInvoiceArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    // Parse amount with flexible format support
    let amount = AmountInput::from_str(&args.amount)
        .map_err(|e| anyhow::anyhow!("Invalid amount format: {e}"))?;

    let invoice = cyberkrill_core::generate_invoice_from_address(
        &args.address,
        &amount,
        args.comment.as_deref(),
    )
    .await?;

    serde_json::to_writer_pretty(writer, &invoice)?;
    Ok(())
}

#[cfg(feature = "smartcards")]
async fn tapsigner_address(args: TapsignerAddressArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let address_info = cyberkrill_core::generate_tapsigner_address(&args.path).await?;

    serde_json::to_writer_pretty(writer, &address_info)?;
    Ok(())
}

#[cfg(feature = "smartcards")]
async fn tapsigner_init(args: TapsignerInitArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let init_info = cyberkrill_core::initialize_tapsigner(args.chain_code).await?;

    serde_json::to_writer_pretty(writer, &init_info)?;
    Ok(())
}

#[cfg(feature = "smartcards")]
async fn satscard_address(args: SatscardAddressArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let address_info = cyberkrill_core::generate_satscard_address(args.slot).await?;

    serde_json::to_writer_pretty(writer, &address_info)?;
    Ok(())
}

async fn bitcoin_list_utxos(args: ListUtxosArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    // Parse network
    let network = match args.network.to_lowercase().as_str() {
        "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
        "testnet" => cyberkrill_core::Network::Testnet,
        "signet" => cyberkrill_core::Network::Signet,
        "regtest" => cyberkrill_core::Network::Regtest,
        _ => bail!(
            "Invalid network: {network}. Expected one of: mainnet, testnet, signet, regtest",
            network = args.network
        ),
    };

    // Check if we're using BDK backends
    if args.electrum.is_some()
        || args.esplora.is_some()
        || (args.descriptor.is_some() && args.bitcoin_dir.is_some())
    {
        // BDK path: require descriptor
        let descriptor = args
            .descriptor
            .ok_or_else(|| anyhow::anyhow!("--descriptor is required when using BDK backends"))?;

        let result = if let Some(electrum_url) = args.electrum {
            // Use Electrum backend
            cyberkrill_core::scan_and_list_utxos_electrum(
                &descriptor,
                network,
                &electrum_url,
                200, // default stop_gap
            )
            .await?
        } else if let Some(esplora_url) = args.esplora {
            // Use Esplora backend
            cyberkrill_core::scan_and_list_utxos_esplora(
                &descriptor,
                network,
                &esplora_url,
                200, // default stop_gap
            )
            .await?
        } else if let Some(bitcoin_dir) = args.bitcoin_dir {
            // Use Bitcoin Core backend with BDK
            let bitcoin_path = std::path::Path::new(&bitcoin_dir);
            cyberkrill_core::scan_and_list_utxos_bitcoind(&descriptor, network, bitcoin_path)
                .await?
        } else {
            // Use local BDK wallet (no blockchain connection)
            cyberkrill_core::list_utxos_bdk(&descriptor, network)?
        };

        // Create summary for BDK results
        let summary = cyberkrill_core::get_utxo_summary(result);
        serde_json::to_writer_pretty(writer, &summary)?;
    } else {
        // Bitcoin Core RPC path (original behavior)
        let bitcoin_dir = args.bitcoin_dir.as_ref().map(Path::new);
        let client = cyberkrill_core::BitcoinRpcClient::new_auto(
            args.rpc_url,
            bitcoin_dir,
            args.rpc_user,
            args.rpc_password,
        )?;

        let result = if let Some(descriptor) = args.descriptor {
            client.list_utxos_for_descriptor(&descriptor).await?
        } else if let Some(addresses_str) = args.addresses {
            let addresses: Vec<String> = addresses_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            client.list_utxos_for_addresses(addresses).await?
        } else {
            #[cfg(feature = "frozenkrill")]
            if let Some(wallet_file) = args.wallet_file {
                client.list_utxos_from_wallet_file(&wallet_file).await?
            } else {
                bail!("Either --descriptor, --addresses, or --wallet-file must be provided");
            }
            #[cfg(not(feature = "frozenkrill"))]
            bail!("Either --descriptor or --addresses must be provided");
        };

        serde_json::to_writer_pretty(writer, &result)?;
    }

    Ok(())
}

async fn bitcoin_create_psbt(args: CreatePsbtArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    // Parse network
    let network = match args.network.to_lowercase().as_str() {
        "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
        "testnet" => cyberkrill_core::Network::Testnet,
        "signet" => cyberkrill_core::Network::Signet,
        "regtest" => cyberkrill_core::Network::Regtest,
        _ => bail!(
            "Invalid network: {network}. Expected one of: mainnet, testnet, signet, regtest",
            network = args.network
        ),
    };

    // Get descriptor from wallet file or direct input
    #[cfg(feature = "frozenkrill")]
    let descriptor = if let Some(wallet_file) = &args.wallet_file {
        let (receiving_desc, _change_desc) =
            cyberkrill_core::BitcoinRpcClient::get_descriptors_from_wallet_file(wallet_file)?;
        Some(receiving_desc)
    } else {
        args.descriptor.clone()
    };
    #[cfg(not(feature = "frozenkrill"))]
    let descriptor = args.descriptor.clone();

    // Check if we're using BDK backends
    if args.electrum.is_some()
        || args.esplora.is_some()
        || (descriptor.is_some() && args.bitcoin_dir.is_some())
    {
        // BDK path: require descriptor
        let descriptor = descriptor.ok_or_else(|| {
            anyhow::anyhow!("--descriptor or --wallet-file is required when using BDK backends")
        })?;

        // Parse outputs
        let outputs = parse_outputs(&args.outputs)?;

        // Convert fee rate if provided
        let fee_rate_sat_vb = args.fee_rate.map(|rate| {
            // Convert AmountInput to sats/vB
            rate.as_fractional_sats()
        });

        // Determine backend URL
        let backend = if let Some(electrum_url) = args.electrum {
            format!("electrum://{electrum_url}")
        } else if let Some(esplora_url) = args.esplora {
            format!("esplora://{esplora_url}")
        } else if let Some(bitcoin_dir) = args.bitcoin_dir {
            format!("bitcoind://{bitcoin_dir}")
        } else {
            bail!("No backend specified. Use --electrum, --esplora, or --bitcoin-dir")
        };

        let result = cyberkrill_core::create_psbt_bdk(
            &args.inputs,
            &outputs,
            fee_rate_sat_vb,
            &descriptor,
            network,
            &backend,
        )
        .await?;

        // Write PSBT to separate file if requested
        if let Some(psbt_path) = args.psbt_output {
            std::fs::write(psbt_path, &result.psbt)?;
        }

        serde_json::to_writer_pretty(writer, &result)?;
    } else {
        // Bitcoin Core RPC path (original behavior)
        let bitcoin_dir = args.bitcoin_dir.as_ref().map(Path::new);
        let client = cyberkrill_core::BitcoinRpcClient::new_auto(
            args.rpc_url,
            bitcoin_dir,
            args.rpc_user,
            args.rpc_password,
        )?;

        let result = client
            .create_psbt(&args.inputs, &args.outputs, args.fee_rate)
            .await?;

        // Write PSBT to separate file if requested
        if let Some(psbt_path) = args.psbt_output {
            std::fs::write(psbt_path, &result.psbt)?;
        }

        serde_json::to_writer_pretty(writer, &result)?;
    }

    Ok(())
}

async fn bitcoin_create_funded_psbt(args: CreateFundedPsbtArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    // Parse network
    let network = match args.network.to_lowercase().as_str() {
        "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
        "testnet" => cyberkrill_core::Network::Testnet,
        "signet" => cyberkrill_core::Network::Signet,
        "regtest" => cyberkrill_core::Network::Regtest,
        _ => bail!(
            "Invalid network: {network}. Expected one of: mainnet, testnet, signet, regtest",
            network = args.network
        ),
    };

    // Get descriptor from wallet file or direct input
    #[cfg(feature = "frozenkrill")]
    let descriptor = if let Some(wallet_file) = &args.wallet_file {
        let (receiving_desc, _change_desc) =
            cyberkrill_core::BitcoinRpcClient::get_descriptors_from_wallet_file(wallet_file)?;
        Some(receiving_desc)
    } else {
        args.descriptor.clone()
    };
    #[cfg(not(feature = "frozenkrill"))]
    let descriptor = args.descriptor.clone();

    // Check if we're using BDK backends
    if args.electrum.is_some()
        || args.esplora.is_some()
        || (descriptor.is_some() && args.bitcoin_dir.is_some())
    {
        // BDK path: require descriptor
        let descriptor = descriptor.ok_or_else(|| {
            anyhow::anyhow!("--descriptor or --wallet-file is required when using BDK backends")
        })?;

        // Parse outputs
        let outputs = parse_outputs(&args.outputs)?;

        // Convert fee rate if provided
        let fee_rate_sat_vb = args.fee_rate.map(|rate| {
            // Convert AmountInput to sats/vB
            rate.as_fractional_sats()
        });

        // Determine backend URL
        let backend = if let Some(electrum_url) = args.electrum {
            format!("electrum://{electrum_url}")
        } else if let Some(esplora_url) = args.esplora {
            format!("esplora://{esplora_url}")
        } else if let Some(bitcoin_dir) = args.bitcoin_dir {
            format!("bitcoind://{bitcoin_dir}")
        } else {
            bail!("No backend specified. Use --electrum, --esplora, or --bitcoin-dir")
        };

        let result = cyberkrill_core::create_funded_psbt_bdk(
            &outputs,
            args.conf_target,
            fee_rate_sat_vb,
            &descriptor,
            network,
            &backend,
        )
        .await?;

        // Write PSBT to separate file if requested
        if let Some(psbt_path) = args.psbt_output {
            std::fs::write(psbt_path, &result.psbt)?;
        }

        serde_json::to_writer_pretty(writer, &result)?;
    } else {
        // Bitcoin Core RPC path (original behavior)

        // Validate that inputs are not empty
        if args.inputs.is_empty() {
            bail!(
                "Error: --inputs is required for create-funded-psbt.\n\
                 You must provide either:\n\
                 - Specific UTXOs: --inputs \"txid:vout\"\n\
                 - A descriptor: --inputs \"wpkh([fingerprint/path]xpub.../<0;1>/*)\"\n\n\
                 For automatic selection with BDK backends, use --descriptor with --electrum or --esplora"
            );
        }

        let bitcoin_dir = args.bitcoin_dir.as_ref().map(Path::new);
        let client = cyberkrill_core::BitcoinRpcClient::new_auto(
            args.rpc_url,
            bitcoin_dir,
            args.rpc_user,
            args.rpc_password,
        )?;

        let result = client
            .wallet_create_funded_psbt(
                &args.inputs,
                &args.outputs,
                args.conf_target,
                args.estimate_mode.as_deref(),
                args.fee_rate,
            )
            .await?;

        // Write PSBT to separate file if requested
        if let Some(psbt_path) = args.psbt_output {
            std::fs::write(psbt_path, &result.psbt)?;
        }

        serde_json::to_writer_pretty(writer, &result)?;
    }

    Ok(())
}

async fn bitcoin_move_utxos(args: MoveUtxosArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    // Validate that exactly one fee method is provided
    match (&args.fee_rate, &args.fee) {
        (None, None) => bail!("Must specify either --fee-rate or --fee"),
        (Some(_), Some(_)) => bail!("Cannot specify both --fee-rate and --fee"),
        _ => {}
    }

    // Parse network
    let network = match args.network.to_lowercase().as_str() {
        "mainnet" | "bitcoin" => cyberkrill_core::Network::Bitcoin,
        "testnet" => cyberkrill_core::Network::Testnet,
        "signet" => cyberkrill_core::Network::Signet,
        "regtest" => cyberkrill_core::Network::Regtest,
        _ => bail!(
            "Invalid network: {network}. Expected one of: mainnet, testnet, signet, regtest",
            network = args.network
        ),
    };

    // Get descriptor from wallet file or direct input
    #[cfg(feature = "frozenkrill")]
    let descriptor = if let Some(wallet_file) = &args.wallet_file {
        let (receiving_desc, _change_desc) =
            cyberkrill_core::BitcoinRpcClient::get_descriptors_from_wallet_file(wallet_file)?;
        Some(receiving_desc)
    } else {
        args.descriptor.clone()
    };
    #[cfg(not(feature = "frozenkrill"))]
    let descriptor = args.descriptor.clone();

    // Check if we're using BDK backends
    if args.electrum.is_some()
        || args.esplora.is_some()
        || (descriptor.is_some() && args.bitcoin_dir.is_some())
    {
        // BDK path: require descriptor
        let descriptor = descriptor.ok_or_else(|| {
            anyhow::anyhow!("--descriptor or --wallet-file is required when using BDK backends")
        })?;

        // Convert fee rate if provided
        let fee_rate_sat_vb = args.fee_rate.map(|rate| {
            // Convert AmountInput to sats/vB
            rate.as_fractional_sats()
        });

        // Convert fee to satoshis if provided
        let fee_sats = args.fee.map(|fee| fee.as_sat());

        // Convert max amount to bitcoin::Amount if provided
        let max_amount = args
            .max_amount
            .map(|amt| cyberkrill_core::bitcoin::Amount::from_sat(amt.as_sat()));

        // Determine backend URL
        let backend = if let Some(electrum_url) = args.electrum {
            format!("electrum://{electrum_url}")
        } else if let Some(esplora_url) = args.esplora {
            format!("esplora://{esplora_url}")
        } else if let Some(bitcoin_dir) = args.bitcoin_dir {
            format!("bitcoind://{bitcoin_dir}")
        } else {
            bail!("No backend specified. Use --electrum, --esplora, or --bitcoin-dir")
        };

        let result = cyberkrill_core::move_utxos_bdk(
            &args.inputs,
            &args.destination,
            fee_rate_sat_vb,
            fee_sats,
            max_amount,
            &descriptor,
            network,
            &backend,
        )
        .await?;

        // Write PSBT to separate file if requested
        if let Some(psbt_path) = args.psbt_output {
            std::fs::write(psbt_path, &result.psbt)?;
        }

        serde_json::to_writer_pretty(writer, &result)?;
    } else {
        // Bitcoin Core RPC path (original behavior)
        let bitcoin_dir = args.bitcoin_dir.as_ref().map(Path::new);
        let client = cyberkrill_core::BitcoinRpcClient::new_auto(
            args.rpc_url,
            bitcoin_dir,
            args.rpc_user,
            args.rpc_password,
        )?;

        let result = client
            .move_utxos(
                &args.inputs,
                &args.destination,
                args.fee_rate,
                args.fee,
                args.max_amount,
            )
            .await?;

        // Write PSBT to separate file if requested
        if let Some(psbt_path) = args.psbt_output {
            std::fs::write(psbt_path, &result.psbt)?;
        }

        serde_json::to_writer_pretty(writer, &result)?;
    }

    Ok(())
}

async fn fedimint_config(args: FedimintConfigArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::stdout()),
    };

    let config = fedimint_lite::fetch_config(&args.invite_code).await?;
    serde_json::to_writer_pretty(writer, &config)?;
    Ok(())
}

fn encode_fedimint_invite(args: EncodeFedimintInviteArgs) -> anyhow::Result<()> {
    // Read input (JSON)
    let input_content = if args.input == "-" {
        let mut buffer = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer)?;
        buffer
    } else {
        std::fs::read_to_string(&args.input)?
    };

    // Parse JSON into FedimintInviteOutput
    let mut invite: fedimint_lite::InviteCode =
        serde_json::from_str(&input_content).context("Failed to parse JSON input")?;

    // Skip API secret if requested for compatibility
    if args.skip_api_secret {
        invite.api_secret = None;
    }

    // Encode to invite code
    let encoded_invite = fedimint_lite::encode_invite(&invite)?;

    // Write output
    let mut writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::stdout()),
    };

    writeln!(writer, "{encoded_invite}")?;
    Ok(())
}

/// Parse output string in format "address:amount,address:amount" into Vec<(String, Amount)>
/// Supports flexible amount formats: "0.5", "0.5btc", "50000000sats", "50000000000msats"
fn parse_outputs(
    outputs_str: &str,
) -> anyhow::Result<Vec<(String, cyberkrill_core::bitcoin::Amount)>> {
    let mut outputs = Vec::new();
    for output in outputs_str.split(',') {
        let parts: Vec<&str> = output.trim().split(':').collect();
        if parts.len() != 2 {
            bail!("Invalid output format: '{output}'. Expected 'address:amount'");
        }

        let address = parts[0].trim().to_string();
        let amount_str = parts[1].trim();

        // Parse amount using AmountInput for flexible format support
        let amount_input = AmountInput::from_str(amount_str).with_context(|| {
            format!("Failed to parse amount '{amount_str}' in output '{output}'")
        })?;

        // Convert to bitcoin::Amount (loses millisat precision)
        let amount = amount_input.as_amount();

        outputs.push((address, amount));
    }
    Ok(outputs)
}

// Jade Hardware Wallet Functions

#[cfg(feature = "jade")]
async fn jade_address(args: JadeAddressArgs) -> anyhow::Result<()> {
    use cyberkrill_core::generate_jade_address;

    let result = generate_jade_address(&args.path, &args.network).await?;

    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writeln!(&mut writer)?;

    Ok(())
}

#[cfg(feature = "jade")]
async fn jade_xpub(args: JadeXpubArgs) -> anyhow::Result<()> {
    use cyberkrill_core::generate_jade_xpub;

    let result = generate_jade_xpub(&args.path, &args.network).await?;

    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writeln!(&mut writer)?;

    Ok(())
}

#[cfg(feature = "jade")]
async fn jade_sign_psbt(args: JadeSignPsbtArgs) -> anyhow::Result<()> {
    use cyberkrill_core::sign_psbt_with_jade;
    use std::path::Path;

    // Read PSBT data from file or parse as base64/hex
    let psbt_input = if Path::new(&args.input).exists() {
        std::fs::read_to_string(&args.input)
            .with_context(|| format!("Failed to read PSBT file: {input}", input = args.input))?
    } else {
        args.input.clone()
    };

    let result = sign_psbt_with_jade(&psbt_input, &args.network).await?;

    // Save JSON output
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writeln!(&mut writer)?;

    // Optionally save raw PSBT
    if let Some(psbt_path) = args.psbt_output {
        let psbt_bytes = hex::decode(&result.psbt_hex)?;
        std::fs::write(psbt_path, psbt_bytes)?;
    }

    Ok(())
}

fn decode_psbt(args: DecodePsbtArgs) -> anyhow::Result<()> {
    use cyberkrill_core::bitcoin::{Network, psbt::Psbt};
    use std::str::FromStr;

    // Parse network
    let network = match args.network.to_lowercase().as_str() {
        "mainnet" | "bitcoin" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => bail!(
            "Invalid network: {network}. Expected one of: mainnet, testnet, signet, regtest",
            network = args.network
        ),
    };

    // Get PSBT string from input or stdin
    let psbt_string = match args.input {
        Some(input) => {
            // Check if it's a file path
            if std::path::Path::new(&input).exists() {
                std::fs::read_to_string(&input)?
            } else {
                // Assume it's the PSBT string directly
                input
            }
        }
        None => {
            // Read from stdin
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer
        }
    };

    // Parse PSBT
    let psbt = Psbt::from_str(psbt_string.trim())?;

    // Create output structure
    let mut output = serde_json::json!({
        "network": network.to_string(),
        "version": psbt.unsigned_tx.version.0,
        "locktime": psbt.unsigned_tx.lock_time.to_consensus_u32(),
        "input_count": psbt.unsigned_tx.input.len(),
        "output_count": psbt.unsigned_tx.output.len(),
        "inputs": [],
        "outputs": [],
        "total_input_value": null,
        "total_output_value": 0u64,
        "fee": null,
    });

    // Process inputs
    let mut total_input_value = 0u64;
    let mut all_inputs_have_value = true;
    let inputs_array = output["inputs"].as_array_mut().unwrap();

    for (i, (input, psbt_input)) in psbt
        .unsigned_tx
        .input
        .iter()
        .zip(psbt.inputs.iter())
        .enumerate()
    {
        let mut input_json = serde_json::json!({
            "index": i,
            "txid": input.previous_output.txid.to_string(),
            "vout": input.previous_output.vout,
            "sequence": input.sequence.0,
        });

        // Try to get witness UTXO for value
        if let Some(witness_utxo) = &psbt_input.witness_utxo {
            input_json["value_sats"] = serde_json::json!(witness_utxo.value.to_sat());
            input_json["value_btc"] = serde_json::json!(witness_utxo.value.to_btc());
            total_input_value += witness_utxo.value.to_sat();
        } else if let Some(non_witness_utxo) = &psbt_input.non_witness_utxo {
            // For non-witness UTXOs, we need to look up the output
            if let Some(output) = non_witness_utxo
                .output
                .get(input.previous_output.vout as usize)
            {
                input_json["value_sats"] = serde_json::json!(output.value.to_sat());
                input_json["value_btc"] = serde_json::json!(output.value.to_btc());
                total_input_value += output.value.to_sat();
            } else {
                all_inputs_have_value = false;
            }
        } else {
            all_inputs_have_value = false;
        }

        // Add signature info
        let num_sigs = psbt_input.partial_sigs.len();
        if num_sigs > 0 {
            input_json["signatures"] = serde_json::json!(num_sigs);
        }

        inputs_array.push(input_json);
    }

    // Process outputs
    let outputs_array = output["outputs"].as_array_mut().unwrap();
    let mut total_output_value = 0u64;

    for (i, tx_output) in psbt.unsigned_tx.output.iter().enumerate() {
        let output_json = serde_json::json!({
            "index": i,
            "value_sats": tx_output.value.to_sat(),
            "value_btc": tx_output.value.to_btc(),
            "script_pubkey": tx_output.script_pubkey.to_hex_string(),
            "address": cyberkrill_core::bitcoin::Address::from_script(&tx_output.script_pubkey, network)
                .map(|a| a.to_string())
                .ok(),
        });
        outputs_array.push(output_json);
        total_output_value += tx_output.value.to_sat();
    }

    // Update totals
    output["total_output_value"] = serde_json::json!(total_output_value);
    if all_inputs_have_value {
        output["total_input_value"] = serde_json::json!(total_input_value);
        output["fee"] = serde_json::json!(total_input_value.saturating_sub(total_output_value));
    }

    // Write output
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };
    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &output)?;
    writeln!(&mut writer)?;

    Ok(())
}

// Coldcard command implementations

#[cfg(feature = "coldcard")]
async fn coldcard_address(args: ColdcardAddressArgs) -> anyhow::Result<()> {
    use cyberkrill_core::generate_coldcard_address;

    let result = generate_coldcard_address(&args.path).await?;

    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writeln!(&mut writer)?;

    Ok(())
}

#[cfg(feature = "coldcard")]
async fn coldcard_sign_psbt(args: ColdcardSignPsbtArgs) -> anyhow::Result<()> {
    use cyberkrill_core::sign_psbt_with_coldcard;

    // Read PSBT data from file or parse as base64/hex
    let psbt_data = if Path::new(&args.input).exists() {
        std::fs::read(&args.input)
            .with_context(|| format!("Failed to read PSBT file: {input}", input = args.input))?
    } else if args.input.starts_with("cHNidP") {
        // Looks like base64
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &args.input)
            .with_context(|| "Failed to decode PSBT from base64")?
    } else {
        // Try as hex
        hex::decode(&args.input).with_context(|| "Failed to decode PSBT from hex")?
    };

    let result = sign_psbt_with_coldcard(&psbt_data).await?;

    // Save JSON output
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writeln!(&mut writer)?;

    // Optionally save raw PSBT
    if let Some(psbt_path) = args.psbt_output {
        let psbt_bytes = hex::decode(&result.psbt_hex)?;
        std::fs::write(psbt_path, psbt_bytes)?;
    }

    Ok(())
}

#[cfg(feature = "coldcard")]
async fn coldcard_export_psbt(args: ColdcardExportPsbtArgs) -> anyhow::Result<()> {
    use cyberkrill_core::export_psbt_to_coldcard;

    // Read PSBT data from file or parse as base64/hex
    let psbt_data = if Path::new(&args.input).exists() {
        std::fs::read(&args.input)
            .with_context(|| format!("Failed to read PSBT file: {input}", input = args.input))?
    } else if args.input.starts_with("cHNidP") {
        // Looks like base64
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &args.input)
            .with_context(|| "Failed to decode PSBT from base64")?
    } else {
        // Try as hex
        hex::decode(&args.input).with_context(|| "Failed to decode PSBT from hex")?
    };

    let message = export_psbt_to_coldcard(&psbt_data, &args.filename).await?;

    // Output as JSON for consistency
    let result = serde_json::json!({
        "message": message,
        "filename": args.filename
    });
    let result_str = serde_json::to_string_pretty(&result)?;
    println!("{result_str}");

    Ok(())
}

#[cfg(feature = "trezor")]
async fn trezor_address(args: TrezorAddressArgs) -> anyhow::Result<()> {
    use cyberkrill_core::{Network, generate_trezor_address};

    let network = args
        .network
        .parse::<Network>()
        .with_context(|| format!("Invalid network: {network}", network = args.network))?;

    let result = generate_trezor_address(&args.path, network).await?;

    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writeln!(&mut writer)?;

    Ok(())
}

#[cfg(feature = "trezor")]
async fn trezor_sign_psbt(args: TrezorSignPsbtArgs) -> anyhow::Result<()> {
    use cyberkrill_core::{Network, sign_psbt_with_trezor};

    let network = args
        .network
        .parse::<Network>()
        .with_context(|| format!("Invalid network: {network}", network = args.network))?;

    // Read PSBT data from file or parse as base64/hex
    let psbt_data = if Path::new(&args.input).exists() {
        std::fs::read(&args.input)
            .with_context(|| format!("Failed to read PSBT file: {input}", input = args.input))?
    } else if args.input.starts_with("cHNidP") {
        // Looks like base64
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &args.input)
            .context("Failed to decode base64 PSBT")?
    } else {
        // Try as hex
        hex::decode(&args.input).context("Failed to decode hex PSBT")?
    };

    let result = sign_psbt_with_trezor(&psbt_data, network).await?;

    // Save JSON output
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &result)?;
    writeln!(&mut writer)?;

    // Optionally save raw PSBT
    if let Some(psbt_path) = args.psbt_output {
        let psbt_bytes = hex::decode(&result.psbt_hex)?;
        std::fs::write(psbt_path, psbt_bytes)?;
    }

    Ok(())
}

async fn dca_report(args: DcaReportArgs) -> anyhow::Result<()> {
    use cyberkrill_core::{Backend, generate_dca_report};

    // Determine backend based on arguments
    let backend = if let Some(bitcoin_dir) = args.bitcoin_dir {
        Backend::BitcoinCore { bitcoin_dir }
    } else if let Some(electrum_url) = args.electrum {
        Backend::Electrum { url: electrum_url }
    } else if let Some(esplora_url) = args.esplora {
        Backend::Esplora { url: esplora_url }
    } else {
        // Default to Bitcoin Core with default directory
        let default_dir = std::path::Path::new(&std::env::var("HOME")?).join(".bitcoin");
        Backend::BitcoinCore {
            bitcoin_dir: default_dir,
        }
    };

    // Generate the report
    let report = generate_dca_report(
        &args.descriptor,
        backend,
        &args.currency,
        args.cache_dir.as_deref(),
    )
    .await?;

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&report)?;

    // Output
    if let Some(output_path) = args.output {
        std::fs::write(output_path, json)?;
    } else {
        println!("{json}");
    }

    Ok(())
}

async fn mcp_server(args: McpServerArgs) -> anyhow::Result<()> {
    use mcp_server::{CyberkrillMcpServer, McpServerConfig, Transport};

    let transport = match args.transport.to_lowercase().as_str() {
        "stdio" => Transport::Stdio,
        "sse" => Transport::Sse,
        _ => bail!(
            "Invalid transport: {transport}. Expected 'stdio' or 'sse'",
            transport = args.transport
        ),
    };

    let config = McpServerConfig {
        transport,
        host: args.host,
        port: args.port,
    };

    let server = CyberkrillMcpServer::new(config);
    server.run().await?;

    Ok(())
}

fn generate_mnemonic(args: GenerateMnemonicArgs) -> anyhow::Result<()> {
    use bip39::{Language, Mnemonic};
    use rand::{Rng, thread_rng};

    // Map word count to entropy length in bytes
    let entropy_bytes = match args.words {
        12 => 16, // 128 bits
        15 => 20, // 160 bits
        18 => 24, // 192 bits
        21 => 28, // 224 bits
        24 => 32, // 256 bits
        _ => bail!(
            "Invalid word count: {}. Must be 12, 15, 18, 21, or 24",
            args.words
        ),
    };

    // Generate random entropy
    let mut rng = thread_rng();
    let mut entropy = vec![0u8; entropy_bytes];
    rng.fill(&mut entropy[..]);

    // Generate mnemonic from entropy
    let mnemonic = Mnemonic::from_entropy_in(Language::English, &entropy)
        .map_err(|e| anyhow::anyhow!("Failed to generate mnemonic: {}", e))?;

    // Get the mnemonic phrase
    let phrase = mnemonic.to_string();

    // Create output JSON
    let output = serde_json::json!({
        "mnemonic": phrase,
        "words": args.words,
        "entropy_bits": entropy_bytes * 8,
    });

    // Write output
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let mut writer = writer;
    serde_json::to_writer_pretty(&mut writer, &output)?;
    writeln!(&mut writer)?;

    Ok(())
}
