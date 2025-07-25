mod bitcoin_rpc;
mod decoder;
mod satscard;
mod tapsigner;

use anyhow::bail;
use clap::{Parser, Subcommand};
use std::io::{BufWriter, Read};
use std::path::Path;

const DEFAULT_BITCOIN_RPC_URL: &str = "http://127.0.0.1:8332";

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Decode(DecodeArgs),
    Generate(GenerateArgs),
    Tapsigner(TapsignerArgs),
    Satscard(SatscardArgs),
    Bitcoin(BitcoinArgs),
}

#[derive(clap::Args, Debug)]
struct DecodeArgs {
    #[clap(subcommand)]
    command: DecodeCommands,
}

#[derive(Subcommand, Debug)]
enum DecodeCommands {
    Invoice(DecodeInvoiceArgs),
    Lnurl(DecodeLnurlArgs),
}

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
struct GenerateArgs {
    #[clap(subcommand)]
    command: GenerateCommands,
}

#[derive(Subcommand, Debug)]
enum GenerateCommands {
    Invoice(GenerateInvoiceArgs),
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

#[derive(clap::Args, Debug)]
struct TapsignerArgs {
    #[clap(subcommand)]
    command: TapsignerCommands,
}

#[derive(Subcommand, Debug)]
enum TapsignerCommands {
    Address(TapsignerAddressArgs),
    Init(TapsignerInitArgs),
}

#[derive(clap::Args, Debug)]
struct TapsignerAddressArgs {
    /// Derivation path (e.g., m/84'/0'/0'/0/0)
    #[clap(short, long, default_value = "m/84'/0'/0'/0/0")]
    path: String,
    /// Output file path
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct TapsignerInitArgs {
    /// Optional custom chain code (64 hex chars = 32 bytes). If not provided, will generate random.
    #[clap(long)]
    chain_code: Option<String>,
    /// Output file path for initialization details
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct SatscardArgs {
    #[clap(subcommand)]
    command: SatscardCommands,
}

#[derive(Subcommand, Debug)]
enum SatscardCommands {
    Address(SatscardAddressArgs),
}

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
struct BitcoinArgs {
    #[clap(subcommand)]
    command: BitcoinCommands,
}

#[derive(Subcommand, Debug)]
enum BitcoinCommands {
    ListUtxos(ListUtxosArgs),
    CreatePsbt(CreatePsbtArgs),
    CreateFundedPsbt(CreateFundedPsbtArgs),
}

#[derive(clap::Args, Debug)]
struct ListUtxosArgs {
    /// Bitcoin Core RPC URL (default: http://127.0.0.1:8332)
    #[clap(long, default_value = DEFAULT_BITCOIN_RPC_URL)]
    rpc_url: String,
    /// Bitcoin directory path (for cookie authentication, default: ~/.bitcoin)
    #[clap(long)]
    bitcoin_dir: Option<String>,
    /// RPC username (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with = "bitcoin_dir")]
    rpc_user: Option<String>,
    /// RPC password (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with = "bitcoin_dir")]
    rpc_password: Option<String>,
    /// Output descriptor to scan for UTXOs
    #[clap(long, conflicts_with = "addresses")]
    descriptor: Option<String>,
    /// Comma-separated list of addresses to list UTXOs for
    #[clap(long, conflicts_with = "descriptor")]
    addresses: Option<String>,
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
    /// Bitcoin Core RPC URL (default: http://127.0.0.1:8332)
    #[clap(long, default_value = DEFAULT_BITCOIN_RPC_URL)]
    rpc_url: String,
    /// Bitcoin directory path (for cookie authentication, default: ~/.bitcoin)
    #[clap(long)]
    bitcoin_dir: Option<String>,
    /// RPC username (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with = "bitcoin_dir")]
    rpc_user: Option<String>,
    /// RPC password (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with = "bitcoin_dir")]
    rpc_password: Option<String>,
    /// Input UTXOs in format txid:vout (comma-separated)
    #[clap(long, required = true)]
    inputs: String,
    /// Output addresses and amounts in format address:amount_btc (comma-separated)
    #[clap(long, required = true)]
    outputs: String,
    /// Fee rate in sats/vB (optional, will use Bitcoin Core's default if not specified)
    #[clap(long)]
    fee_rate: Option<f64>,
    /// Output file path for JSON response
    #[clap(short, long)]
    output: Option<String>,
    /// Output file path for raw PSBT data (base64)
    #[clap(long)]
    psbt_output: Option<String>,
}

#[derive(clap::Args, Debug)]
struct CreateFundedPsbtArgs {
    /// Bitcoin Core RPC URL (default: http://127.0.0.1:8332)
    #[clap(long, default_value = DEFAULT_BITCOIN_RPC_URL)]
    rpc_url: String,
    /// Bitcoin directory path (for cookie authentication, default: ~/.bitcoin)
    #[clap(long)]
    bitcoin_dir: Option<String>,
    /// RPC username (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with = "bitcoin_dir")]
    rpc_user: Option<String>,
    /// RPC password (conflicts with bitcoin-dir)
    #[clap(long, conflicts_with = "bitcoin_dir")]
    rpc_password: Option<String>,
    /// Input UTXOs in format txid:vout (comma-separated). Leave empty for automatic input selection.
    #[clap(long)]
    inputs: Option<String>,
    /// Output addresses and amounts in format address:amount_btc (comma-separated)
    #[clap(long, required = true)]
    outputs: String,
    /// Confirmation target in blocks (1-1008)
    #[clap(long)]
    conf_target: Option<u32>,
    /// Fee estimation mode: UNSET, ECONOMICAL, CONSERVATIVE
    #[clap(long)]
    estimate_mode: Option<String>,
    /// Fee rate in sats/vB (overrides conf_target and estimate_mode)
    #[clap(long)]
    fee_rate: Option<f64>,
    /// Output file path for JSON response
    #[clap(short, long)]
    output: Option<String>,
    /// Output file path for raw PSBT data (base64)
    #[clap(long)]
    psbt_output: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Cli = Cli::parse();
    match args.command {
        Commands::Decode(args) => decode(args)?,
        Commands::Generate(args) => generate(args).await?,
        Commands::Tapsigner(args) => tapsigner(args).await?,
        Commands::Satscard(args) => satscard(args).await?,
        Commands::Bitcoin(args) => bitcoin(args).await?,
    }
    Ok(())
}

fn decode(args: DecodeArgs) -> anyhow::Result<()> {
    match args.command {
        DecodeCommands::Invoice(args) => decode_invoice(args)?,
        DecodeCommands::Lnurl(args) => decode_lnurl(args)?,
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

    let output = decoder::decode_lnurl(&input)?;
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

    let output = decoder::decode_invoice(&input)?;
    serde_json::to_writer_pretty(writer, &output)?;
    Ok(())
}

async fn generate(args: GenerateArgs) -> anyhow::Result<()> {
    match args.command {
        GenerateCommands::Invoice(args) => generate_invoice(args).await?,
    }
    Ok(())
}

async fn generate_invoice(args: GenerateInvoiceArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let invoice = decoder::generate_invoice_from_address(
        &args.address,
        args.amount_msats,
        args.comment.as_deref(),
    )
    .await?;

    serde_json::to_writer_pretty(writer, &invoice)?;
    Ok(())
}

async fn tapsigner(args: TapsignerArgs) -> anyhow::Result<()> {
    match args.command {
        TapsignerCommands::Address(args) => tapsigner_address(args).await?,
        TapsignerCommands::Init(args) => tapsigner_init(args).await?,
    }
    Ok(())
}

async fn tapsigner_address(args: TapsignerAddressArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let address_info = tapsigner::generate_tapsigner_address(&args.path).await?;

    serde_json::to_writer_pretty(writer, &address_info)?;
    Ok(())
}

async fn tapsigner_init(args: TapsignerInitArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let init_info = tapsigner::initialize_tapsigner(args.chain_code).await?;

    serde_json::to_writer_pretty(writer, &init_info)?;
    Ok(())
}

async fn satscard(args: SatscardArgs) -> anyhow::Result<()> {
    match args.command {
        SatscardCommands::Address(args) => satscard_address(args).await?,
    }
    Ok(())
}

async fn satscard_address(args: SatscardAddressArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let address_info = satscard::generate_satscard_address(args.slot).await?;

    serde_json::to_writer_pretty(writer, &address_info)?;
    Ok(())
}

async fn bitcoin(args: BitcoinArgs) -> anyhow::Result<()> {
    match args.command {
        BitcoinCommands::ListUtxos(args) => bitcoin_list_utxos(args).await?,
        BitcoinCommands::CreatePsbt(args) => bitcoin_create_psbt(args).await?,
        BitcoinCommands::CreateFundedPsbt(args) => bitcoin_create_funded_psbt(args).await?,
    }
    Ok(())
}

async fn bitcoin_list_utxos(args: ListUtxosArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let bitcoin_dir = args.bitcoin_dir.as_ref().map(Path::new);
    let client = bitcoin_rpc::BitcoinRpcClient::new_auto(
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
    Ok(())
}

async fn bitcoin_create_psbt(args: CreatePsbtArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let bitcoin_dir = args.bitcoin_dir.as_ref().map(Path::new);
    let client = bitcoin_rpc::BitcoinRpcClient::new_auto(
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
    Ok(())
}

async fn bitcoin_create_funded_psbt(args: CreateFundedPsbtArgs) -> anyhow::Result<()> {
    let writer: Box<dyn std::io::Write> = match args.output {
        Some(path) => Box::new(BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };

    let bitcoin_dir = args.bitcoin_dir.as_ref().map(Path::new);
    let client = bitcoin_rpc::BitcoinRpcClient::new_auto(
        args.rpc_url,
        bitcoin_dir,
        args.rpc_user,
        args.rpc_password,
    )?;

    let inputs = args.inputs.as_deref().unwrap_or("");
    let result = client
        .wallet_create_funded_psbt(
            inputs,
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
    Ok(())
}
