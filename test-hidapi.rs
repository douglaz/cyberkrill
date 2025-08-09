// Simple test program to check hidapi functionality
use hidapi::HidApi;

fn main() {
    println!("Testing hidapi...");
    
    // Try to create HidApi instance
    match HidApi::new() {
        Ok(api) => {
            println!("HidApi created successfully");
            
            // List all devices
            for device in api.device_list() {
                println!("Device: {:04x}:{:04x} - {} - {}",
                    device.vendor_id(),
                    device.product_id(),
                    device.manufacturer_string().unwrap_or("Unknown"),
                    device.product_string().unwrap_or("Unknown")
                );
                
                // Check for Coldcard
                if device.vendor_id() == 0xd13e {
                    println!("Found Coldcard device!");
                }
            }
        }
        Err(e) => {
            println!("Failed to create HidApi: {:?}", e);
        }
    }
    
    println!("Test completed");
}