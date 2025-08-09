//! Test unlocking Jade with better logging

use jade_bitcoin::{JadeClient, Network};
use std::io;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Jade Unlock Test");
    println!("================");

    // Connect to Jade
    println!("Connecting to Jade...");
    let mut client = JadeClient::connect().await?;
    println!("✓ Connected");

    // Check version
    println!("\nGetting version info...");
    let version = client.get_version_info().await?;
    println!("✓ Version: {}", version.jade_version);
    println!("  State: {}", version.jade_state);

    // Check if already unlocked
    if client.is_unlocked().await {
        println!("\n✓ Device is already unlocked!");
    } else {
        println!("\n⚠ Device is locked. Starting unlock process...");
        println!("Please be ready to enter your PIN on the Jade device when prompted.");
        println!("Press Enter to continue...");

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        println!("Unlocking for mainnet...");
        match client.unlock(Network::Bitcoin).await {
            Ok(()) => {
                println!("✓ Device unlocked successfully!");
            }
            Err(e) => {
                println!("✗ Failed to unlock: {}", e);
                return Err(e.into());
            }
        }
    }

    // Test getting xpub
    println!("\nTesting xpub retrieval...");
    match client.get_xpub("m/84'/0'/0'").await {
        Ok(xpub) => {
            println!("✓ Got xpub: {}", &xpub[..20]);
        }
        Err(e) => {
            println!("✗ Failed to get xpub: {}", e);
        }
    }

    println!("\n✓ Test completed successfully!");
    Ok(())
}
