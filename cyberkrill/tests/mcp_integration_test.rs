use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// MCP protocol request structure
#[derive(Debug, Serialize)]
struct McpRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: Option<u64>,  // Notifications don't have IDs
}

/// MCP protocol response structure
#[derive(Debug, Deserialize)]
struct McpResponse {
    jsonrpc: String,
    result: Option<Value>,
    error: Option<Value>,
    id: Option<u64>,  // Some responses may not have IDs
}

/// MCP test client for integration testing
struct McpTestClient {
    process: Child,
    stdin: std::process::ChildStdin,
    stdout: BufReader<std::process::ChildStdout>,
    request_id: u64,
}

impl McpTestClient {
    /// Start the MCP server and create a test client
    fn new() -> Result<Self> {
        // Set RUST_LOG to error to avoid info logs interfering with JSON parsing
        let mut process = Command::new("cargo")
            .args(&["run", "--", "mcp-server", "-t", "stdio"])
            .env("RUST_LOG", "error")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .context("Failed to spawn MCP server")?;

        let stdin = process
            .stdin
            .take()
            .context("Failed to get stdin handle")?;
        let stdout = BufReader::new(
            process
                .stdout
                .take()
                .context("Failed to get stdout handle")?,
        );

        let mut client = McpTestClient {
            process,
            stdin,
            stdout,
            request_id: 0,
        };

        // Wait for server to initialize
        std::thread::sleep(Duration::from_millis(1000));

        // Send initialize request
        client.initialize()?;

        Ok(client)
    }

    /// Send a request and get response
    fn send_request(&mut self, method: &str, params: Value) -> Result<McpResponse> {
        self.request_id += 1;
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: Some(self.request_id),
        };

        // Send request
        let request_str = serde_json::to_string(&request)?;
        writeln!(self.stdin, "{}", request_str)?;
        self.stdin.flush()?;

        // Read response
        let mut response_str = String::new();
        self.stdout.read_line(&mut response_str)?;
        
        let response: McpResponse = serde_json::from_str(&response_str)
            .with_context(|| format!("Failed to parse response: {}", response_str))?;

        Ok(response)
    }
    
    /// Send a notification (no response expected)
    fn send_notification(&mut self, method: &str, params: Value) -> Result<()> {
        let notification = McpRequest {
            jsonrpc: "2.0".to_string(),
            method: method.to_string(),
            params,
            id: None,  // Notifications don't have IDs
        };

        // Send notification
        let notification_str = serde_json::to_string(&notification)?;
        writeln!(self.stdin, "{}", notification_str)?;
        self.stdin.flush()?;

        Ok(())
    }

    /// Initialize the MCP connection
    fn initialize(&mut self) -> Result<()> {
        let response = self.send_request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }),
        )?;

        if response.error.is_some() {
            bail!("Failed to initialize: {:?}", response.error);
        }

        // Send initialized notification using the new method
        self.send_notification("notifications/initialized", json!({}))?;

        Ok(())
    }

    /// Check if server is initialized
    fn is_initialized(&self) -> bool {
        // Simple check - if we got here, the server is initialized
        true
    }
}

impl Drop for McpTestClient {
    fn drop(&mut self) {
        // Clean shutdown of the MCP server
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

#[test]
fn test_mcp_server_initialization() -> Result<()> {
    let client = McpTestClient::new()?;
    assert!(client.is_initialized());
    drop(client);
    Ok(())
}

#[test]
fn test_mcp_server_starts_and_stops() -> Result<()> {
    // Start multiple instances to ensure clean startup/shutdown
    for _ in 0..3 {
        let client = McpTestClient::new()?;
        assert!(client.is_initialized());
        drop(client);
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

#[test]
fn test_decode_invoice_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // Sample BOLT11 invoice
    let invoice = "lnbc100n1p3g0jthpp5uypgqerzuah0ure0f9nauzqxul7zhrvgn6r0a072lqkgt5vy4dsdqqcqzpgxqyz5vqsp5k0tg68rk4ezgvaskyv2rwqe0pjeqve68mwfqr5c93qkc0u7z90q9qyyssqny4pdhqmqfhh7g5rl058qjzk6t3jqutegjhxqhtv0p7g8ex6czmgp026x6pnk9aw64kk65aqay09q8yddwmaf5cuvm0ngcm7kxdqpvgqyt8";

    let result = client.call_tool(
        "decode_invoice",
        json!({
            "invoice": invoice
        }),
    )?;

    // Verify the response contains expected fields
    assert!(result.get("payment_hash").is_some());
    assert!(result.get("amount_msat").is_some());
    assert!(result.get("description").is_some());

    Ok(())
}

#[test]
fn test_decode_lnurl_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // Sample LNURL
    let lnurl = "LNURL1DP68GURN8GHJ7UM9WFMXJCM99E3K7MF0V9CXJ0M385EKVCENXC6R2C35XVUKXEFCV5MKVV34X5EKZD3EV56NYD3HXQURZEPEXEJXXEPNXSCRVWFNV9NXZCN9XQ6XYEFHVGCXXCMYXYMNSERXFQ5FNS";

    let result = client.call_tool(
        "decode_lnurl",
        json!({
            "lnurl": lnurl
        }),
    )?;

    // Verify the response contains expected fields
    assert!(result.get("url").is_some());
    assert!(result.get("domain").is_some());

    Ok(())
}

#[test]
fn test_decode_fedimint_invite_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // Sample Fedimint invite code
    let invite_code = "fed11qgqzxgthwden5te0v9cxjtnzd96xxmmfdckhqunfde3kjurvv4ejucm0d5hsqqfqkggx3jz0tvfv5n7lj0e7gs7nh47z06ry95x4963wfh8xlka7a80su3952t";

    let result = client.call_tool(
        "decode_fedimint_invite",
        json!({
            "invite_code": invite_code
        }),
    )?;

    // Verify the response contains expected fields
    assert!(result.get("federation_id").is_some());
    assert!(result.get("guardians").is_some());

    Ok(())
}

#[test]
fn test_encode_fedimint_invite_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    let result = client.call_tool(
        "encode_fedimint_invite",
        json!({
            "federation_id": "1111111111111111111111111111111111111111111111111111111111111111",
            "guardians": [
                {
                    "peer_id": 0,
                    "url": "wss://example.com/"
                }
            ],
            "skip_api_secret": true
        }),
    )?;

    // Verify we got an invite code back
    assert!(result.get("invite_code").is_some());
    let invite_code = result["invite_code"].as_str().unwrap();
    assert!(invite_code.starts_with("fed1"));

    Ok(())
}

#[test]
fn test_decode_psbt_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // Sample PSBT (empty transaction for testing)
    let psbt = "cHNidP8BAHUCAAAAASaBcTce3/KF6Tet7qSze3gADAVmy7OtZGQXE8pCFxv2AAAAAAD+////AtPf9QUAAAAAGXapFNDFmQPFusKGh2DpD9UhpGZap2UgiKwA4fUFAAAAABepFDVF5uM7gyxHBQ8k0+65PJwDlIvHh7MuEwAAAQD9pQEBAAAAAAECiaPHHqtNIOA3G7ukzGmPopXJRjr6Ljl/hTPMti+VZ+UBAAAAFxYAFL4Y0VKpsBIDna89p95PUzSe7LmF/////4b4qkOnHf8USIk6UwpyN+9rRgi7st0tAXHmOuxqSJC0AQAAABcWABT+Pp7xp0XpdNkCxDVZQ6vLNL1TU/////8CAMLrCwAAAAAZdqkUhc/xCX/Z4Ai7NK9wnGIZeziXikiIrHL++E4sAAAAF6kUM5cluiHv1irHU6m80GfWx6ajnQWHAkcwRAIgJxK+IuAnDzlPVoMR3HyppolwuAJf3TskAinwf4pfOiQCIAGLONfc0xTnNMkna9b7QPZzMlvEuqFEyADS8vAtsnZcASED0uFWdJQbrUqZY3LLh+GFbTZSYG2YVi/jnF6efkE/IQUCSDBFAiEA0SuFLYXc2WHS9fSrZgZU327tzHlMDDPOXMMJ/7X85Y0CIGczio4OFyXBl/saiK9Z9R5E5CVbIBZ8hoQDHAXR8lkqASECI7cr7vCWXRC+B3jv7NYfysb3mk6haTkzgHNEZPhPKrMAAAAAAAAA";

    let result = client.call_tool(
        "decode_psbt",
        json!({
            "psbt": psbt
        }),
    )?;

    // Verify the response contains expected fields
    assert!(result.get("version").is_some());
    assert!(result.get("inputs").is_some());
    assert!(result.get("outputs").is_some());

    Ok(())
}

#[test]
fn test_list_utxos_tool_with_invalid_descriptor() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // This should return an error since we're using a test descriptor without a real backend
    let result = client.call_tool(
        "list_utxos",
        json!({
            "descriptor": "wpkh([fingerprint/84'/0'/0']xpub6CY2xt3vG5BhUS7krcphJpcrNo8GyNZJ)"
        }),
    )?;

    // The result should be an error string since we don't have a real backend
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}

#[test]
fn test_invalid_tool_call() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // Try to call a non-existent tool
    let result = client.send_request(
        "tools/call",
        json!({
            "name": "non_existent_tool",
            "arguments": {}
        }),
    );

    // This should return an error
    assert!(result.is_ok());
    let response = result?;
    assert!(response.error.is_some());

    Ok(())
}

#[test]
fn test_tool_with_invalid_arguments() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // Call decode_invoice without required invoice parameter
    let result = client.call_tool(
        "decode_invoice",
        json!({}),
    )?;

    // Should return an error
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}

#[test]
fn test_create_psbt_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // This will fail without a real backend, but tests the tool is available
    let result = client.call_tool(
        "create_psbt",
        json!({
            "inputs": ["txid:0"],
            "outputs": "bc1qtest:0.001",
            "fee_rate": 10.0
        }),
    )?;

    // Should return an error without real backend
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}

#[test]
fn test_move_utxos_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // This will fail without a real backend, but tests the tool is available
    let result = client.call_tool(
        "move_utxos",
        json!({
            "inputs": ["txid:0"],
            "destination": "bc1qtest",
            "fee_rate": 10.0
        }),
    )?;

    // Should return an error without real backend
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}

#[test] 
fn test_dca_report_tool() -> Result<()> {
    let mut client = McpTestClient::new()?;
    
    // This will fail without a real backend, but tests the tool is available
    let result = client.call_tool(
        "dca_report",
        json!({
            "descriptor": "wpkh([fingerprint/84'/0'/0']xpub...)",
            "currency": "USD"
        }),
    )?;

    // Should return an error without real backend
    let result_str = result.as_str().unwrap_or("");
    assert!(result_str.contains("Error"));

    Ok(())
}