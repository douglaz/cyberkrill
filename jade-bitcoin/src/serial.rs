//! Serial port communication for Jade (async implementation)

use crate::error::{Error, Result};
use crate::messages::{Request, Response};
use crate::types::{JADE_USB_IDS, SERIAL_BAUD_RATE, SERIAL_TIMEOUT_MS};
use log::debug;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, sleep, timeout};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

/// Async serial connection to Jade device
pub struct SerialConnection {
    port: SerialStream,
    read_buffer: Vec<u8>,
}

impl SerialConnection {
    /// Connect to Jade on any available port
    pub async fn connect() -> Result<Self> {
        let ports = Self::find_jade_ports();
        if ports.is_empty() {
            debug!("No Jade devices found");
            return Err(Error::DeviceNotFound);
        }

        debug!("Found {} potential Jade device(s)", ports.len());
        // Try each port until one works
        for port_info in ports {
            debug!("Attempting to connect to Jade on {}", port_info.port_name);
            match Self::connect_path(&port_info.port_name).await {
                Ok(conn) => {
                    debug!("Successfully connected to Jade on {}", port_info.port_name);
                    return Ok(conn);
                }
                Err(e) => {
                    debug!("Failed to connect to {}: {}", port_info.port_name, e);
                }
            }
        }

        Err(Error::DeviceNotFound)
    }

    /// Connect to Jade on specific port
    pub async fn connect_path(path: &str) -> Result<Self> {
        debug!("Opening serial port: {path}");

        let port = tokio_serial::new(path, SERIAL_BAUD_RATE)
            .data_bits(tokio_serial::DataBits::Eight)
            .stop_bits(tokio_serial::StopBits::One)
            .parity(tokio_serial::Parity::None)
            .flow_control(tokio_serial::FlowControl::None)
            .open_native_async()?;

        // Give device a moment to be ready
        sleep(Duration::from_millis(500)).await;

        Ok(Self {
            port,
            read_buffer: Vec::with_capacity(65536),
        })
    }

    /// Find all potential Jade serial ports
    pub fn find_jade_ports() -> Vec<tokio_serial::SerialPortInfo> {
        let ports = tokio_serial::available_ports().unwrap_or_default();

        ports
            .into_iter()
            .filter(|port| {
                if let tokio_serial::SerialPortType::UsbPort(info) = &port.port_type {
                    JADE_USB_IDS
                        .iter()
                        .any(|(vid, pid)| info.vid == *vid && info.pid == *pid)
                } else {
                    false
                }
            })
            .collect()
    }

    /// List all available Jade devices
    pub fn list_devices() -> Vec<String> {
        Self::find_jade_ports()
            .into_iter()
            .map(|p| p.port_name)
            .collect()
    }

    /// Send a request and receive response
    pub async fn request(&mut self, request: &Request) -> Result<Response> {
        self.send_request(request).await?;
        self.receive_response().await
    }

    /// Send a CBOR-encoded request
    pub async fn send_request(&mut self, request: &Request) -> Result<()> {
        let cbor = serde_cbor::to_vec(request)?;
        debug!("Sending request: {request:?}");
        debug!("CBOR hex: {}", hex::encode(&cbor));

        self.port.write_all(&cbor).await?;
        self.port.flush().await?;

        Ok(())
    }

    /// Receive and decode a CBOR response
    pub async fn receive_response(&mut self) -> Result<Response> {
        // Read CBOR message
        // Jade sends complete CBOR messages, so we need to read until we have a complete one

        debug!("Starting to receive response from Jade...");
        self.read_buffer.clear();
        let mut temp_buffer = [0u8; 4096];
        let mut consecutive_empty_reads = 0;

        // Read data until we have a complete CBOR message
        loop {
            // Use timeout for read operations
            let read_result = timeout(
                Duration::from_millis(SERIAL_TIMEOUT_MS),
                self.port.read(&mut temp_buffer),
            )
            .await;

            match read_result {
                Ok(Ok(0)) => {
                    // No data available right now
                    consecutive_empty_reads += 1;

                    // If we've had several empty reads and have data, try to parse it
                    if consecutive_empty_reads > 3 && !self.read_buffer.is_empty() {
                        match serde_cbor::from_slice::<Response>(&self.read_buffer) {
                            Ok(response) => return Ok(response),
                            Err(e) => {
                                debug!(
                                    "Failed to decode {} bytes after empty reads: {}",
                                    self.read_buffer.len(),
                                    e
                                );
                                // Give it more time
                                sleep(Duration::from_millis(100)).await;
                            }
                        }
                    }

                    if consecutive_empty_reads > 10 {
                        return Err(Error::Timeout);
                    }

                    // Small delay before retry
                    sleep(Duration::from_millis(10)).await;
                }
                Ok(Ok(n)) => {
                    // Got data
                    consecutive_empty_reads = 0;
                    self.read_buffer.extend_from_slice(&temp_buffer[..n]);

                    // Try to decode CBOR
                    match serde_cbor::from_slice::<Response>(&self.read_buffer) {
                        Ok(response) => {
                            debug!("Received response: {response:?}");
                            debug!("Response hex: {}", hex::encode(&self.read_buffer));
                            return Ok(response);
                        }
                        Err(e) => {
                            // Check if this looks like a complete but invalid message
                            if self.read_buffer.len() > 1000 {
                                debug!(
                                    "Failed to decode CBOR after {} bytes: {}",
                                    self.read_buffer.len(),
                                    e
                                );
                                debug!(
                                    "Raw hex (first 200 bytes): {}",
                                    hex::encode(
                                        &self.read_buffer[..200.min(self.read_buffer.len())]
                                    )
                                );
                                return Err(Error::InvalidResponse);
                            }
                            // Not enough data yet, continue reading
                            continue;
                        }
                    }
                }
                Ok(Err(e)) => {
                    return Err(Error::Io(e));
                }
                Err(_) => {
                    // Timeout
                    if !self.read_buffer.is_empty() {
                        match serde_cbor::from_slice::<Response>(&self.read_buffer) {
                            Ok(response) => return Ok(response),
                            Err(decode_err) => {
                                debug!(
                                    "Timeout with {} bytes, decode error: {}",
                                    self.read_buffer.len(),
                                    decode_err
                                );
                            }
                        }
                    }
                    return Err(Error::Timeout);
                }
            }
        }
    }
}
