//! Example: Get extended public key from Jade with network selection

use jade_bitcoin::{JadeClient, Network};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("m/84'/0'/0'");
    let network_str = args.get(2).map(|s| s.as_str()).unwrap_or("testnet");

    let network = match network_str {
        "mainnet" | "bitcoin" => Network::Bitcoin,
        "testnet" => Network::Testnet,
        "regtest" => Network::Regtest,
        "signet" => Network::Signet,
        _ => {
            eprintln!(
                "Invalid network: {}. Use mainnet, testnet, regtest, or signet",
                network_str
            );
            return Ok(());
        }
    };

    println!("Connecting to Jade...");

    // Connect to Jade
    let mut jade = JadeClient::connect()?;
    println!("Connected to Jade");

    // Try to unlock for the specified network
    println!("Unlocking device for {:?}...", network);
    jade.unlock(network)?;
    println!("Device unlocked");

    // Get extended public key
    println!("\nGetting xpub for path: {}", path);
    let xpub = jade.get_xpub(path)?;

    println!("\n=== Extended Public Key ===");
    println!("Network: {:?}", network);
    println!("Path: {}", path);
    println!("xpub: {}", xpub);

    Ok(())
}
