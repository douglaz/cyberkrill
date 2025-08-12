//! Integration tests for jade-bitcoin
//!
//! These tests require a real Jade device to be connected.
//! Run with: cargo test --features integration-tests -- --nocapture

#[cfg(feature = "integration-tests")]
mod integration_tests {
    use jade_bitcoin::{JadeClient, Network};

    #[test]
    fn test_list_devices() {
        let devices = JadeClient::list_devices();
        println!("Found {} Jade device(s): {:?}", devices.len(), devices);
        // Don't assert on device count as it varies
    }

    #[tokio::test]
    async fn test_connect_and_version() {
        let mut jade = match JadeClient::connect().await {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Skipping test - no Jade device found: {}", e);
                return;
            }
        };

        let version = jade.get_version_info().await.expect("Failed to get version");
        assert!(!version.jade_version.is_empty());
        println!("Jade version: {}", version.jade_version);
    }

    #[tokio::test]
    async fn test_unlock_and_get_address() {
        let mut jade = match JadeClient::connect().await {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Skipping test - no Jade device found: {}", e);
                return;
            }
        };

        // Test with testnet to avoid mainnet operations
        jade.unlock(Network::Testnet)
            .await
            .expect("Failed to unlock Jade");

        let address = jade
            .get_address("m/84'/1'/0'/0/0", Network::Testnet)
            .await
            .expect("Failed to get address");

        // Testnet bech32 addresses start with "tb1"
        assert!(address.starts_with("tb1"));
        println!("Testnet address: {}", address);

        jade.logout().await.expect("Failed to logout");
    }

    #[tokio::test]
    async fn test_get_xpub() {
        let mut jade = match JadeClient::connect().await {
            Ok(j) => j,
            Err(e) => {
                eprintln!("Skipping test - no Jade device found: {}", e);
                return;
            }
        };

        jade.unlock(Network::Testnet)
            .await
            .expect("Failed to unlock Jade");

        let xpub = jade.get_xpub("m/84'/1'/0'").await.expect("Failed to get xpub");

        // Testnet xpubs start with "tpub"
        assert!(xpub.starts_with("tpub") || xpub.starts_with("vpub"));
        println!("xpub: {}", xpub);

        jade.logout().await.expect("Failed to logout");
    }
}

#[cfg(not(feature = "integration-tests"))]
mod unit_tests {
    use jade_bitcoin::JadeClient;

    #[test]
    fn test_list_devices_no_device() {
        // This should work even without a device
        let devices = JadeClient::list_devices();
        // Just ensure it doesn't panic
        println!("Device list returned: {devices:?}");
    }
}
