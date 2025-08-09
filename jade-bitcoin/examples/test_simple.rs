//! Simple test to diagnose connection issues

use jade_bitcoin::{JadeClient, Network};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Starting simple Jade test...");

    // Just try to connect
    println!("Attempting to connect to Jade...");
    let mut client = JadeClient::connect().await?;
    println!("Connected!");

    // Try to get version
    println!("Getting version info...");
    let version = client.get_version_info().await?;
    println!("Version: {:?}", version);

    // Check if unlocked
    println!("Checking unlock status...");
    let unlocked = client.is_unlocked().await;
    println!("Unlocked: {}", unlocked);

    if !unlocked {
        println!("Device is locked. Attempting to unlock for mainnet...");
        client.unlock(Network::Bitcoin).await?;
        println!("Unlock successful!");
    }

    // Try to get xpub
    println!("Getting xpub...");
    let xpub = client.get_xpub("m/84'/0'/0'").await?;
    println!("xpub: {}", xpub);

    println!("Test completed successfully!");
    Ok(())
}
