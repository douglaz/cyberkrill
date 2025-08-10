//! Example: Get extended public key from Jade (assumes already unlocked)

use jade_bitcoin::{JadeClient, Network};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("m/84'/0'/0'");

    println!("Connecting to Jade...");

    // Connect to Jade
    let mut jade = JadeClient::connect().await?;
    println!("Connected to Jade");

    // Try to unlock for testnet (or check if already unlocked)
    println!("Checking device status...");
    match jade.unlock(Network::Testnet).await {
        Ok(_) => println!("Device unlocked"),
        Err(e) => {
            // If already unlocked, this might succeed anyway
            println!("Note: {e}");
        }
    }

    // Get extended public key
    println!("\nGetting xpub for path: {path}");
    let xpub = jade.get_xpub(path).await?;

    println!("\n=== Extended Public Key ===");
    println!("Path: {path}");
    println!("xpub: {xpub}");

    Ok(())
}
