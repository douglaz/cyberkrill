//! Example: Generate Bitcoin address with Jade

use jade_bitcoin::{JadeClient, Network};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("m/84'/0'/0'/0/0");
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

    println!("Connecting to Jade...");

    // List available devices
    let devices = JadeClient::list_devices();
    if devices.is_empty() {
        eprintln!("No Jade devices found!");
        eprintln!("Please connect your Jade device and ensure you have proper permissions.");
        return Ok(());
    }

    println!("Found {} Jade device(s): {:?}", devices.len(), devices);

    // Connect to Jade
    let mut jade = JadeClient::connect()?;
    println!("Connected to Jade");

    // Get version info
    let version = jade.get_version_info()?;
    println!("Jade version: {}", version.jade_version);
    println!("Board type: {}", version.board_type);

    // Unlock the device
    println!("\nUnlocking Jade for {:?}...", network);
    println!("Please check your Jade device and confirm the operation");
    jade.unlock(network)?;
    println!("Jade unlocked");

    // Generate address
    println!("\nGenerating address for path: {}", path);
    let address = jade.get_address(path, network)?;

    println!("\n=== Bitcoin Address ===");
    println!("Path:    {}", path);
    println!("Network: {:?}", network);
    println!("Address: {}", address);

    // Logout
    jade.logout()?;
    println!("\nLogged out from Jade");

    Ok(())
}
