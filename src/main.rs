mod decoder;

use clap::{Parser, Subcommand};
use std::io::{BufWriter, Read};

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Decode(DecodeArgs),
    Generate(GenerateArgs),
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Cli = Cli::parse();
    match args.command {
        Commands::Decode(args) => decode(args)?,
        Commands::Generate(args) => generate(args).await?,
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
