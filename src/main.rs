use std::{
    io::{BufWriter, Read},
    str::FromStr,
};

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Decode(DecodeArgs),
}

#[derive(clap::Args, Debug)]
struct DecodeArgs {
    #[clap(subcommand)]
    command: DecodeCommands,
}

#[derive(Subcommand, Debug)]
enum DecodeCommands {
    Invoice(DecodeInvoiceArgs),
}

#[derive(clap::Args, Debug)]
struct DecodeInvoiceArgs {
    input: Option<String>,
    #[clap(short, long)]
    output: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args: Cli = Cli::parse();
    match args.command {
        Commands::Decode(args) => decode(args)?,
    }

    Ok(())
}

fn decode(args: DecodeArgs) -> anyhow::Result<()> {
    match args.command {
        DecodeCommands::Invoice(args) => decode_invoice(args)?,
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
struct InvoiceOutput {
    network: String,
    amount_msats: Option<u64>,
    timestamp_millis: u128,
    payment_hash: String,
    payment_secret: String,
    description: Option<String>,
    description_hash: Option<String>,
    destination: Option<String>,
    expiry_seconds: u64,
    min_final_cltv_expiry: u64,
    fallback_addresses: Vec<String>,
    routes: Vec<Vec<RouteHintHopOutput>>,
}

#[derive(Serialize, Deserialize)]
pub struct RouteHintHopOutput {
    /// The node_id of the non-target end of the route
    pub src_node_id: String,
    /// The short_channel_id of this channel
    pub short_channel_id: u64,
    /// The fees which must be paid to use this channel
    pub fees: RoutingFeesOutput,
    /// The difference in CLTV values between this node and the next node.
    pub cltv_expiry_delta: u16,
    /// The minimum value, in msat, which must be relayed to the next hop.
    pub htlc_minimum_msat: Option<u64>,
    /// The maximum value in msat available for routing with a single HTLC.
    pub htlc_maximum_msat: Option<u64>,
}

impl From<&lightning_invoice::RouteHintHop> for RouteHintHopOutput {
    fn from(hop: &lightning_invoice::RouteHintHop) -> Self {
        Self {
            src_node_id: hop.src_node_id.to_string(),
            short_channel_id: hop.short_channel_id,
            fees: (&hop.fees).into(),
            cltv_expiry_delta: hop.cltv_expiry_delta,
            htlc_minimum_msat: hop.htlc_minimum_msat,
            htlc_maximum_msat: hop.htlc_maximum_msat,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct RoutingFeesOutput {
    /// Flat routing fee in millisatoshis.
    pub base_msat: u32,
    /// Liquidity-based routing fee in millionths of a routed amount.
    /// In other words, 10000 is 1%.
    pub proportional_millionths: u32,
}

impl From<&lightning_invoice::RoutingFees> for RoutingFeesOutput {
    fn from(fees: &lightning_invoice::RoutingFees) -> Self {
        Self {
            base_msat: fees.base_msat,
            proportional_millionths: fees.proportional_millionths,
        }
    }
}

impl From<lightning_invoice::Bolt11Invoice> for InvoiceOutput {
    fn from(invoice: lightning_invoice::Bolt11Invoice) -> Self {
        Self {
            network: invoice.network().to_string(),
            amount_msats: invoice.amount_milli_satoshis(),
            timestamp_millis: invoice.duration_since_epoch().as_millis(),
            payment_hash: invoice.payment_hash().to_string(),
            payment_secret: hex::encode(invoice.payment_secret().0),
            description: match invoice.description() {
                lightning_invoice::Bolt11InvoiceDescriptionRef::Direct(description) => {
                    Some(description.to_string())
                }
                lightning_invoice::Bolt11InvoiceDescriptionRef::Hash(_sha256) => None,
            },
            description_hash: match invoice.description() {
                lightning_invoice::Bolt11InvoiceDescriptionRef::Direct(_description) => None,
                lightning_invoice::Bolt11InvoiceDescriptionRef::Hash(sha256) => {
                    Some(sha256.0.to_string())
                }
            },
            destination: invoice.payee_pub_key().map(|k| k.to_string()),
            expiry_seconds: invoice.expiry_time().as_secs(),
            min_final_cltv_expiry: invoice.min_final_cltv_expiry_delta(),
            fallback_addresses: invoice
                .fallback_addresses()
                .iter()
                .map(|a| a.to_string())
                .collect(),
            routes: invoice
                .route_hints()
                .iter()
                .map(|hints| hints.0.iter().map(|hop| hop.into()).collect())
                .collect(),
        }
    }
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

    let invoice =
        lightning_invoice::Bolt11Invoice::from_str(&input).map_err(|e| anyhow::anyhow!("{e:?}"))?;

    let invoice = InvoiceOutput::from(invoice);

    serde_json::to_writer_pretty(writer, &invoice)?;
    Ok(())
}
