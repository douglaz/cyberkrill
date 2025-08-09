//! Example: Get extended public key from Jade

use jade_bitcoin::{JadeClient, Network};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let path = args.get(1).map(|s| s.as_str()).unwrap_or("m/84'/0'/0'");

    println!("Connecting to Jade...");

    // Connect to Jade
    let mut jade = JadeClient::connect()?;
    println!("Connected to Jade");

    // Get version info
    let version = jade.get_version_info()?;
    println!("Jade version: {}", version.jade_version);

    // Unlock the device (xpub works with any network)
    println!("\nUnlocking Jade...");
    println!("Please check your Jade device and confirm the operation");
    jade.unlock(Network::Bitcoin)?;
    println!("Jade unlocked");

    // Get extended public key
    println!("\nGetting xpub for path: {}", path);
    let xpub = jade.get_xpub(path)?;

    println!("\n=== Extended Public Key ===");
    println!("Path: {}", path);
    println!("xpub: {}", xpub);

    // Demonstrate getting multiple xpubs
    println!("\n=== Additional xpubs ===");

    // Account 0 for different purposes
    for (purpose, name) in &[
        (44, "Legacy (P2PKH)"),
        (49, "Nested SegWit (P2SH-P2WPKH)"),
        (84, "Native SegWit (P2WPKH)"),
        (86, "Taproot (P2TR)"),
    ] {
        let account_path = format!("m/{}'/{}'/{}'", purpose, 0, 0);
        match jade.get_xpub(&account_path) {
            Ok(xpub) => {
                println!("\n{} - {}", name, account_path);
                println!("{}", xpub);
            }
            Err(e) => {
                eprintln!("Failed to get xpub for {}: {}", account_path, e);
            }
        }
    }

    // Logout
    jade.logout()?;
    println!("\nLogged out from Jade");

    Ok(())
}
