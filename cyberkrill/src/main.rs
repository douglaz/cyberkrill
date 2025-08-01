use anyhow::{bail, Context};
use clap::{Parser, Subcommand};
use cyberkrill_core::AmountInput;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

const DEFAULT_BITCOIN_RPC_URL: &str = "http://127.0.0.1:8332";

#[derive(Parser)]
#[command(name = "cyberkrill")]
#[command(about = "A CLI toolkit for Bitcoin and Lightning Network operations")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    // Lightning Network Operations
    #[command(about = "Decode BOLT11 Lightning invoice")]
    DecodeInvoice(DecodeInvoiceArgs),
    #[command(about = "Decode LNURL string")]
    DecodeLnurl(DecodeLnurlArgs),
    #[command(about = "Generate invoice from Lightning address")]
    GenerateInvoice(GenerateInvoiceArgs),

    // Fedimint Operations
    #[command(about = "Decode Fedimint invite code")]
    DecodeFedimintInvite(DecodeFedimintInviteArgs),
    #[command(about = "Encode Fedimint invite code from JSON")]
    EncodeFedimintInvite(EncodeFedimintInviteArgs),
    #[command(about = "Fetch Fedimint federation configuration")]
    FedimintConfig(FedimintConfigArgs),

    // Hardware Wallet Operations
    #[cfg(feature = "smartcards")]
    #[command(about = "Generate Bitcoin address from Tapsigner")]
    TapsignerAddress(TapsignerAddressArgs),
    #[cfg(feature = "smartcards")]
    #[command(about = "Initialize Tapsigner (one-time setup)")]
    TapsignerInit(TapsignerInitArgs),
    #[cfg(feature = "smartcards")]
    #[command(about = "Generate Bitcoin address from Satscard")]
    SatscardAddress(SatscardAddressArgs),

    // Bitcoin RPC Operations
    #[command(about = "List UTXOs for addresses or descriptors")]
    ListUtxos(ListUtxosArgs),
    #[command(
        about = "Create PSBT with manual input/output specification (you specify exact inputs, outputs, and change)"
    )]
    CreatePsbt(CreatePsbtArgs),
    #[command(
        about = "Create funded PSBT with automatic input selection and change output (wallet handles coin selection)"
    )]
    CreateFundedPsbt(CreateFundedPsbtArgs),
    #[command(
        about = "Consolidate/move UTXOs to a single destination address (output = total inputs - fee)"
    )]
    MoveUtxos(MoveUtxosArgs),
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
    /// Amount in millisatoshis
    amount_msats: u64,
    /// Optional comment
    #[clap(short, long)]
    comment: Option<String>,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
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

#[derive(clap::Args, Debug)]
struct ListUtxosArgs {
    /// Output descriptor to scan for UTXOs (required when using BDK backends)
    #[clap(long, conflicts_with = "addresses")]
    descriptor: Option<String>,
    /// Comma-separated list of addresses to list UTXOs for (only for Bitcoin Core RPC)
    #[clap(long, conflicts_with = "descriptor")]
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
    /// Output descriptor (required when using BDK backends)
    #[clap(long)]
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
    /// Output addresses and amounts in format address:amount_btc (comma-separated)
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
    /// Output descriptor (required when using BDK backends)
    #[clap(long)]
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
    /// Input UTXOs in format txid:vout or output descriptors (can be specified multiple times). Leave empty for automatic input selection.
    /// Examples: --inputs txid1:0 --inputs txid2:1 or --inputs "wpkh([fingerprint/84'/0'/0']xpub...)"
    #[clap(long)]
    inputs: Vec<String>,
    /// Output addresses and amounts in format address:amount_btc (comma-separated)
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
    /// Output descriptor (required when using BDK backends)
    #[clap(long)]
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
        Commands::DecodeInvoice(args) => decode_invoice(args)?,
        Commands::DecodeLnurl(args) => decode_lnurl(args)?,
        Commands::GenerateInvoice(args) => generate_invoice(args).await?,

        // Fedimint Operations
        Commands::DecodeFedimintInvite(args) => decode_fedimint_invite(args)?,
        Commands::EncodeFedimintInvite(args) => encode_fedimint_invite(args)?,
        Commands::FedimintConfig(args) => fedimint_config(args).await?,

        // Hardware Wallet Operations
        #[cfg(feature = "smartcards")]
        Commands::TapsignerAddress(args) => tapsigner_address(args).await?,
        #[cfg(feature = "smartcards")]
        Commands::TapsignerInit(args) => tapsigner_init(args).await?,
        #[cfg(feature = "smartcards")]
        Commands::SatscardAddress(args) => satscard_address(args).await?,

        // Bitcoin RPC Operations
        Commands::ListUtxos(args) => bitcoin_list_utxos(args).await?,
        Commands::CreatePsbt(args) => bitcoin_create_psbt(args).await?,
        Commands::CreateFundedPsbt(args) => bitcoin_create_funded_psbt(args).await?,
        Commands::MoveUtxos(args) => bitcoin_move_utxos(args).await?,
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

    let invoice = cyberkrill_core::generate_invoice_from_address(
        &args.address,
        args.amount_msats,
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
            "Invalid network: {}. Expected one of: mainnet, testnet, signet, regtest",
            args.network
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
            "Invalid network: {}. Expected one of: mainnet, testnet, signet, regtest",
            args.network
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
            "Invalid network: {}. Expected one of: mainnet, testnet, signet, regtest",
            args.network
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
            "Invalid network: {}. Expected one of: mainnet, testnet, signet, regtest",
            args.network
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
fn parse_outputs(
    outputs_str: &str,
) -> anyhow::Result<Vec<(String, cyberkrill_core::bitcoin::Amount)>> {
    let mut outputs = Vec::new();
    for output in outputs_str.split(',') {
        let parts: Vec<&str> = output.trim().split(':').collect();
        if parts.len() != 2 {
            bail!(
                "Invalid output format: '{}'. Expected 'address:amount'",
                output
            );
        }

        let address = parts[0].trim().to_string();
        let amount_str = parts[1].trim();

        // Parse amount as BTC
        let amount_btc: f64 = amount_str
            .parse()
            .context("Failed to parse amount as BTC")?;
        let amount =
            cyberkrill_core::bitcoin::Amount::from_btc(amount_btc).context("Invalid BTC amount")?;

        outputs.push((address, amount));
    }
    Ok(outputs)
}
