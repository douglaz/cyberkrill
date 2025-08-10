//! Example: Sign PSBT with Jade

use jade_bitcoin::{JadeClient, Network};
use std::env;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <psbt_file> [network]", args[0]);
        eprintln!("  psbt_file: Path to PSBT file to sign");
        eprintln!("  network: bitcoin|testnet|regtest|signet (default: bitcoin)");
        return Ok(());
    }

    let psbt_file = &args[1];
    let network = args
        .get(2)
        .and_then(|s| match s.as_str() {
            "mainnet" | "bitcoin" => Some(Network::Bitcoin),
            "testnet" => Some(Network::Testnet),
            "regtest" => Some(Network::Regtest),
            "signet" => Some(Network::Signet),
            _ => None,
        })
        .unwrap_or(Network::Bitcoin);

    // Read PSBT file
    println!("Reading PSBT from: {psbt_file}");
    let psbt_bytes = fs::read(psbt_file)?;
    println!("PSBT size: {} bytes", psbt_bytes.len());

    println!("\nConnecting to Jade...");

    // Connect to Jade
    let mut jade = JadeClient::connect().await?;
    println!("Connected to Jade");

    // Get version info
    let version = jade.get_version_info().await?;
    println!("Jade version: {}", version.jade_version);

    // Unlock the device
    println!("\nUnlocking Jade for {network:?}...");
    println!("Please check your Jade device and confirm the operation");
    jade.unlock(network).await?;
    println!("Jade unlocked");

    // Sign the PSBT
    println!("\nSigning PSBT...");
    println!("Please review and confirm the transaction on your Jade device");

    let signed_psbt = jade.sign_psbt(&psbt_bytes, network).await?;

    println!("PSBT signed successfully!");
    println!("Signed PSBT size: {} bytes", signed_psbt.len());

    // Save signed PSBT
    let output_file = psbt_file.replace(".psbt", "_signed.psbt");
    fs::write(&output_file, signed_psbt)?;
    println!("Signed PSBT saved to: {output_file}");

    // Logout
    jade.logout().await?;
    println!("\nLogged out from Jade");

    Ok(())
}
