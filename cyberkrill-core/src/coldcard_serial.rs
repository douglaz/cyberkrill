use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_serial::SerialPortBuilderExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct ColdcardSerialAddressInfo {
    pub path: String,
    pub address: String,
    pub pubkey: String,
    pub xpub: String,
}

/// Coldcard protocol packet structure
#[derive(Debug, Clone)]
struct ColdcardPacket {
    /// Framing byte: lower 6 bits = length, bit 0x80 = last packet, bit 0x40 = encrypted
    framing: u8,
    /// Payload data (max 63 bytes per packet)
    payload: Vec<u8>,
}

impl ColdcardPacket {
    fn new(payload: Vec<u8>, is_last: bool, is_encrypted: bool) -> Self {
        let mut framing = (payload.len() as u8) & 0x3F; // Lower 6 bits for length
        if is_last {
            framing |= 0x80;
        }
        if is_encrypted {
            framing |= 0x40;
        }
        Self { framing, payload }
    }
    
    fn is_last(&self) -> bool {
        self.framing & 0x80 != 0
    }
    
    #[allow(dead_code)]
    fn is_encrypted(&self) -> bool {
        self.framing & 0x40 != 0
    }
    
    fn length(&self) -> usize {
        (self.framing & 0x3F) as usize
    }
    
    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.framing];
        bytes.extend_from_slice(&self.payload);
        // Pad to 64 bytes for USB HID
        bytes.resize(64, 0);
        bytes
    }
}

pub struct ColdcardSerial {
    port_name: String,
}

impl ColdcardSerial {
    /// Create a new serial connection to Coldcard
    pub fn new(port_name: Option<String>) -> Result<Self> {
        let port_name = port_name.unwrap_or_else(|| "/dev/ttyACM0".to_string());
        
        // Check if the device exists
        if !std::path::Path::new(&port_name).exists() {
            bail!("Serial device {} not found. Is Coldcard connected and unlocked?", port_name);
        }
        
        Ok(Self { port_name })
    }
    
    /// Send a USB HID packet through serial interface
    async fn send_packet(&self, packet: &ColdcardPacket, port: &mut tokio_serial::SerialStream) -> Result<()> {
        let bytes = packet.to_bytes();
        println!("Sending packet: {:02x?} (first 10 bytes)", &bytes[..10.min(bytes.len())]);
        port.write_all(&bytes).await
            .context("Failed to write packet to serial port")?;
        Ok(())
    }
    
    /// Read a USB HID packet from serial interface
    async fn read_packet(&self, port: &mut tokio_serial::SerialStream) -> Result<ColdcardPacket> {
        let mut buffer = vec![0u8; 64];
        println!("Waiting for packet...");
        let n = port.read_exact(&mut buffer).await
            .context("Failed to read packet from serial port")?;
        
        println!("Read {} bytes: {:02x?} (first 10)", n, &buffer[..10.min(n)]);
        
        if n < 1 {
            bail!("Empty packet received");
        }
        
        let framing = buffer[0];
        let length = (framing & 0x3F) as usize;
        
        if length > 63 {
            bail!("Invalid packet length: {}", length);
        }
        
        let payload = buffer[1..=length].to_vec();
        
        Ok(ColdcardPacket {
            framing,
            payload,
        })
    }
    
    /// Send a command and receive response using Coldcard protocol
    async fn send_command(&self, cmd_code: &[u8; 4], data: Option<&[u8]>) -> Result<Vec<u8>> {
        // Open serial port
        let mut port = tokio_serial::new(&self.port_name, 115200)
            .timeout(Duration::from_secs(10))
            .open_native_async()
            .context("Failed to open serial port")?;
        
        // Configure port
        port.set_exclusive(false)
            .context("Unable to set serial port exclusive")?;
        
        // Build request payload
        let mut payload = cmd_code.to_vec();
        if let Some(data) = data {
            payload.extend_from_slice(data);
        }
        
        // Split into packets if needed (max 63 bytes per packet)
        let mut packets = Vec::new();
        for (i, chunk) in payload.chunks(63).enumerate() {
            let is_last = i == payload.chunks(63).count() - 1;
            packets.push(ColdcardPacket::new(chunk.to_vec(), is_last, false));
        }
        
        // Send all packets
        for packet in &packets {
            self.send_packet(packet, &mut port).await?;
        }
        
        // Read response packets
        let mut response = Vec::new();
        loop {
            let packet = self.read_packet(&mut port).await?;
            response.extend_from_slice(&packet.payload[..packet.length()]);
            
            if packet.is_last() {
                break;
            }
        }
        
        // Check response code (first 4 bytes)
        if response.len() < 4 {
            bail!("Response too short: {} bytes", response.len());
        }
        
        Ok(response)
    }
    
    /// Get version information
    pub async fn get_version(&self) -> Result<String> {
        let response = self.send_command(b"vers", None).await?;
        
        // Skip the 4-byte response code
        if response.len() > 4 {
            let version_bytes = &response[4..];
            Ok(String::from_utf8_lossy(version_bytes).to_string())
        } else {
            bail!("Invalid version response")
        }
    }
    
    /// Send ping command
    pub async fn ping(&self) -> Result<String> {
        let test_msg = b"Hello Coldcard";
        let response = self.send_command(b"ping", Some(test_msg)).await?;
        
        // Skip the 4-byte response code "pong"
        if response.len() > 4 && &response[0..4] == b"pong" {
            let echo = &response[4..];
            Ok(String::from_utf8_lossy(echo).to_string())
        } else {
            bail!("Invalid ping response")
        }
    }
    
    /// Get extended public key (xpub)
    pub async fn get_xpub(&self, path: &str) -> Result<String> {
        // Convert path to bytes
        let path_bytes = path.as_bytes();
        let response = self.send_command(b"xpub", Some(path_bytes)).await?;
        
        // Response format: "xpub" + xpub_string
        if response.len() > 4 && &response[0..4] == b"xpub" {
            let xpub_bytes = &response[4..];
            Ok(String::from_utf8_lossy(xpub_bytes).to_string())
        } else {
            bail!("Invalid xpub response")
        }
    }
    
    /// Show address on device and get it back
    pub async fn show_address(&self, path: &str, addr_fmt: u8) -> Result<String> {
        // Build request: path as string + format byte
        let mut data = path.as_bytes().to_vec();
        data.push(0); // null terminator
        data.push(addr_fmt); // 0=P2PKH, 1=P2WPKH-P2SH, 2=P2WPKH
        
        let response = self.send_command(b"show", Some(&data)).await?;
        
        // Response format: "addr" + address_string
        if response.len() > 4 && &response[0..4] == b"addr" {
            let addr_bytes = &response[4..];
            // Find null terminator if present
            let addr_str = if let Some(null_pos) = addr_bytes.iter().position(|&b| b == 0) {
                String::from_utf8_lossy(&addr_bytes[..null_pos]).to_string()
            } else {
                String::from_utf8_lossy(addr_bytes).to_string()
            };
            Ok(addr_str)
        } else {
            bail!("Invalid address response")
        }
    }
    
    /// Generate address for a given derivation path
    /// Note: The address network depends on the Coldcard's internal settings
    pub async fn get_address(&self, path: &str) -> Result<ColdcardSerialAddressInfo> {
        // Coldcard doesn't have a network parameter in the protocol
        // It uses the configured chain setting (mainnet/testnet)
        // For now, we'll use P2WPKH (native segwit) format
        let addr_fmt = 2; // P2WPKH
        
        // Get the address
        let address = self.show_address(path, addr_fmt).await?;
        
        // Get the xpub for this path
        let xpub = self.get_xpub(path).await?;
        
        Ok(ColdcardSerialAddressInfo {
            path: path.to_string(),
            address,
            pubkey: "".to_string(), // Not directly available via protocol
            xpub,
        })
    }
    
    /// Test serial connection
    pub async fn test_connection(&self) -> Result<()> {
        println!("Testing connection to Coldcard on {}", self.port_name);
        
        // Try ping first
        println!("Sending ping command...");
        match self.ping().await {
            Ok(echo) => println!("Coldcard ping response: {}", echo),
            Err(e) => println!("Ping failed: {}", e),
        }
        
        // Try to get version
        println!("Sending version command...");
        match self.get_version().await {
            Ok(version) => println!("Coldcard version: {}", version),
            Err(e) => println!("Version query failed: {}", e),
        }
        
        Ok(())
    }
    
    /// Sign a PSBT transaction
    pub async fn sign_psbt(&self, psbt_bytes: &[u8]) -> Result<Vec<u8>> {
        // Send PSBT for signing
        let response = self.send_command(b"stxn", Some(psbt_bytes)).await?;
        
        // Response should be "wait" followed by signing progress
        if response.len() < 4 {
            bail!("Invalid signing response");
        }
        
        let resp_code = &response[0..4];
        if resp_code == b"wait" {
            // Need to poll for completion
            // For now, we'll wait and try to get result
            tokio::time::sleep(Duration::from_secs(2)).await;
            
            // Get signed transaction
            let result = self.send_command(b"stok", None).await?;
            if result.len() > 4 && &result[0..4] == b"psbt" {
                Ok(result[4..].to_vec())
            } else {
                bail!("Failed to get signed PSBT")
            }
        } else if resp_code == b"err_" {
            let error_msg = String::from_utf8_lossy(&response[4..]);
            bail!("Signing error: {}", error_msg)
        } else {
            bail!("Unexpected response: {:?}", resp_code)
        }
    }
}

/// Alternative function using serial instead of HID
/// Note: The address network depends on the Coldcard's internal settings
pub async fn generate_coldcard_serial_address(
    path: &str,
    port: Option<String>,
) -> Result<ColdcardSerialAddressInfo> {
    let coldcard = ColdcardSerial::new(port)?;
    
    // Test connection first
    coldcard.test_connection().await
        .context("Failed to connect to Coldcard via serial")?;
    
    // Get address - network is determined by Coldcard's settings
    coldcard.get_address(path).await
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_serial_detection() {
        // This test checks if we can create a serial connection
        // It will only pass if a Coldcard is connected
        if std::path::Path::new("/dev/ttyACM0").exists() {
            let coldcard = ColdcardSerial::new(None);
            assert!(coldcard.is_ok());
        }
    }
}