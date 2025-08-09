//! Simple test to get version info from Jade

use jade_bitcoin::JadeClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    println!("Connecting to Jade...");

    // List available devices
    let devices = JadeClient::list_devices();
    println!("Found {} device(s): {:?}", devices.len(), devices);

    if devices.is_empty() {
        eprintln!("No Jade devices found!");
        return Ok(());
    }

    // Connect to Jade
    let mut jade = JadeClient::connect()?;
    println!("Connected to Jade");

    // Just try to get version info - this should work without unlock
    println!("\nGetting version info...");
    match jade.get_version_info() {
        Ok(version) => {
            println!("Success! Jade version: {}", version.jade_version);
            println!("Board type: {}", version.board_type);
            println!("Config: {}", version.jade_config);
            println!("Has PIN: {}", version.jade_has_pin);
        }
        Err(e) => {
            eprintln!("Failed to get version info: {}", e);
        }
    }

    Ok(())
}
