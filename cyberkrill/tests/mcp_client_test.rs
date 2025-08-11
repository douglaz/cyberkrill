use anyhow::Result;
use rmcp::{
    model::CallToolRequestParam,
    transport::{ConfigureCommandExt, TokioChildProcess},
    ServiceExt,
};
use serde_json::json;

/// Helper function to start the MCP server and connect as a client
async fn connect_to_server() -> Result<rmcp::service::RunningService<rmcp::RoleClient, ()>> {
    // Start our MCP server as a subprocess and connect to it using rmcp client
    let transport =
        TokioChildProcess::new(tokio::process::Command::new("cargo").configure(|cmd| {
            cmd.args(["run", "--quiet", "--", "mcp-server", "-t", "stdio"])
                .env("RUST_LOG", "error"); // Suppress logs for cleaner test output
        }))?;

    let client = ().serve(transport).await?;
    Ok(client)
}

#[tokio::test]
async fn test_mcp_server_connects() -> Result<()> {
    let _client = connect_to_server().await?;
    // If we get here, the server started and connected successfully
    Ok(())
}

#[tokio::test]
async fn test_list_all_tools() -> Result<()> {
    let client = connect_to_server().await?;

    // List all available tools
    let tools = client.list_all_tools().await?;

    // Expected tools from our MCP server
    let expected_tools = vec![
        "decode_invoice",
        "decode_lnurl",
        "generate_invoice",
        "decode_fedimint_invite",
        "encode_fedimint_invite",
        "list_utxos",
        "decode_psbt",
        "create_psbt",
        "create_funded_psbt",
        "move_utxos",
        "dca_report",
    ];

    // Verify all expected tools are present
    for expected_tool in &expected_tools {
        let found = tools.iter().any(|t| t.name == *expected_tool);
        assert!(
            found,
            "Tool '{}' not found in tools list. Available tools: {:?}",
            expected_tool,
            tools.iter().map(|t| &t.name).collect::<Vec<_>>()
        );
    }

    // Verify we have exactly 11 tools
    assert_eq!(
        tools.len(),
        11,
        "Expected 11 tools, found {}. Tools: {:?}",
        tools.len(),
        tools.iter().map(|t| &t.name).collect::<Vec<_>>()
    );

    Ok(())
}

#[tokio::test]
async fn test_decode_invoice_tool() -> Result<()> {
    let client = connect_to_server().await?;

    // Sample BOLT11 invoice
    let invoice = "lnbc100n1p3g0jthpp5uypgqerzuah0ure0f9nauzqxul7zhrvgn6r0a072lqkgt5vy4dsdqqcqzpgxqyz5vqsp5k0tg68rk4ezgvaskyv2rwqe0pjeqve68mwfqr5c93qkc0u7z90q9qyyssqny4pdhqmqfhh7g5rl058qjzk6t3jqutegjhxqhtv0p7g8ex6czmgp026x6pnk9aw64kk65aqay09q8yddwmaf5cuvm0ngcm7kxdqpvgqyt8";

    // Call the decode_invoice tool
    let result = client
        .call_tool(CallToolRequestParam {
            name: "decode_invoice".into(),
            arguments: Some(
                json!({
                    "invoice": invoice
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Verify the response contains expected fields
    if let Some(content) = &result.content {
        assert!(!content.is_empty(), "Tool should return content");

        // The result should contain text with the decoded invoice
        let content_text = content[0].as_text();
        assert!(content_text.is_some(), "Content should be text");

        let decoded: serde_json::Value = serde_json::from_str(&content_text.unwrap().text)?;
        assert!(decoded.get("payment_hash").is_some());
        assert!(decoded.get("amount_msat").is_some());
        assert!(decoded.get("description").is_some());
    } else {
        panic!("No content in response");
    }

    Ok(())
}

#[tokio::test]
async fn test_decode_lnurl_tool() -> Result<()> {
    let client = connect_to_server().await?;

    // Sample LNURL
    let lnurl = "LNURL1DP68GURN8GHJ7UM9WFMXJCM99E3K7MF0V9CXJ0M385EKVCENXC6R2C35XVUKXEFCV5MKVV34X5EKZD3EV56NYD3HXQURZEPEXEJXXEPNXSCRVWFNV9NXZCN9XQ6XYEFHVGCXXCMYXYMNSERXFQ5FNS";

    // Call the decode_lnurl tool
    let result = client
        .call_tool(CallToolRequestParam {
            name: "decode_lnurl".into(),
            arguments: Some(
                json!({
                    "lnurl": lnurl
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Verify the response
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&content_text.text)?;

        assert!(decoded.get("url").is_some());
        assert!(decoded.get("domain").is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_decode_fedimint_invite_tool() -> Result<()> {
    let client = connect_to_server().await?;

    // Sample Fedimint invite code
    let invite_code = "fed11qgqzxgthwden5te0v9cxjtnzd96xxmmfdckhqunfde3kjurvv4ejucm0d5hsqqfqkggx3jz0tvfv5n7lj0e7gs7nh47z06ry95x4963wfh8xlka7a80su3952t";

    // Call the decode_fedimint_invite tool
    let result = client
        .call_tool(CallToolRequestParam {
            name: "decode_fedimint_invite".into(),
            arguments: Some(
                json!({
                    "invite_code": invite_code
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Verify the response
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&content_text.text)?;

        assert!(decoded.get("federation_id").is_some());
        assert!(decoded.get("guardians").is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_encode_fedimint_invite_tool() -> Result<()> {
    let client = connect_to_server().await?;

    // Call the encode_fedimint_invite tool
    let result = client
        .call_tool(CallToolRequestParam {
            name: "encode_fedimint_invite".into(),
            arguments: Some(json!({
                "federation_id": "1111111111111111111111111111111111111111111111111111111111111111",
                "guardians": [
                    {
                        "peer_id": 0,
                        "url": "wss://example.com/"
                    }
                ],
                "skip_api_secret": true
            }).as_object().unwrap().clone()),
        })
        .await?;

    // Verify we got an invite code back
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&content_text.text)?;

        assert!(decoded.get("invite_code").is_some());
        let invite_code = decoded["invite_code"].as_str().unwrap();
        assert!(invite_code.starts_with("fed1"));
    }

    Ok(())
}

#[tokio::test]
async fn test_decode_psbt_tool() -> Result<()> {
    let client = connect_to_server().await?;

    // Sample PSBT (empty transaction for testing)
    let psbt = "cHNidP8BAHUCAAAAASaBcTce3/KF6Tet7qSze3gADAVmy7OtZGQXE8pCFxv2AAAAAAD+////AtPf9QUAAAAAGXapFNDFmQPFusKGh2DpD9UhpGZap2UgiKwA4fUFAAAAABepFDVF5uM7gyxHBQ8k0+65PJwDlIvHh7MuEwAAAQD9pQEBAAAAAAECiaPHHqtNIOA3G7ukzGmPopXJRjr6Ljl/hTPMti+VZ+UBAAAAFxYAFL4Y0VKpsBIDna89p95PUzSe7LmF/////4b4qkOnHf8USIk6UwpyN+9rRgi7st0tAXHmOuxqSJC0AQAAABcWABT+Pp7xp0XpdNkCxDVZQ6vLNL1TU/////8CAMLrCwAAAAAZdqkUhc/xCX/Z4Ai7NK9wnGIZeziXikiIrHL++E4sAAAAF6kUM5cluiHv1irHU6m80GfWx6ajnQWHAkcwRAIgJxK+IuAnDzlPVoMR3HyppolwuAJf3TskAinwf4pfOiQCIAGLONfc0xTnNMkna9b7QPZzMlvEuqFEyADS8vAtsnZcASED0uFWdJQbrUqZY3LLh+GFbTZSYG2YVi/jnF6efkE/IQUCSDBFAiEA0SuFLYXc2WHS9fSrZgZU327tzHlMDDPOXMMJ/7X85Y0CIGczio4OFyXBl/saiK9Z9R5E5CVbIBZ8hoQDHAXR8lkqASECI7cr7vCWXRC+B3jv7NYfysb3mk6haTkzgHNEZPhPKrMAAAAAAAAA";

    // Call the decode_psbt tool
    let result = client
        .call_tool(CallToolRequestParam {
            name: "decode_psbt".into(),
            arguments: Some(
                json!({
                    "psbt": psbt
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Verify the response
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&content_text.text)?;

        assert!(decoded.get("version").is_some() || decoded.get("unsigned_tx").is_some());
    }

    Ok(())
}

#[tokio::test]
async fn test_list_utxos_tool_error_case() -> Result<()> {
    let client = connect_to_server().await?;

    // This should return an error since we're using a test descriptor without a real backend
    let result = client
        .call_tool(CallToolRequestParam {
            name: "list_utxos".into(),
            arguments: Some(json!({
                "descriptor": "wpkh([fingerprint/84'/0'/0']xpub6CY2xt3vG5BhUS7krcphJpcrNo8GyNZJ)"
            }).as_object().unwrap().clone()),
        })
        .await?;

    // The result should contain an error message
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        assert!(content_text.text.contains("Error"));
    }

    Ok(())
}

#[tokio::test]
async fn test_create_psbt_tool_error_case() -> Result<()> {
    let client = connect_to_server().await?;

    // This will fail without a real backend, but tests the tool is available
    let result = client
        .call_tool(CallToolRequestParam {
            name: "create_psbt".into(),
            arguments: Some(
                json!({
                    "inputs": ["txid:0"],
                    "outputs": "bc1qtest:0.001",
                    "fee_rate": 10.0
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Should return an error without real backend
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        assert!(content_text.text.contains("Error"));
    }

    Ok(())
}

#[tokio::test]
async fn test_create_funded_psbt_tool_error_case() -> Result<()> {
    let client = connect_to_server().await?;

    // This will fail without a real backend, but tests the tool is available
    let result = client
        .call_tool(CallToolRequestParam {
            name: "create_funded_psbt".into(),
            arguments: Some(
                json!({
                    "outputs": "bc1qtest:0.001",
                    "fee_rate": 20.0,
                    "descriptor": "wpkh([fingerprint/84'/0'/0']xpub...)"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Should return an error without real backend
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        assert!(content_text.text.contains("Error"));
    }

    Ok(())
}

#[tokio::test]
async fn test_move_utxos_tool_error_case() -> Result<()> {
    let client = connect_to_server().await?;

    // This will fail without a real backend, but tests the tool is available
    let result = client
        .call_tool(CallToolRequestParam {
            name: "move_utxos".into(),
            arguments: Some(
                json!({
                    "inputs": ["txid:0"],
                    "destination": "bc1qtest",
                    "fee_rate": 10.0
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Should return an error without real backend
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        assert!(content_text.text.contains("Error"));
    }

    Ok(())
}

#[tokio::test]
async fn test_generate_invoice_tool_error_case() -> Result<()> {
    let client = connect_to_server().await?;

    // This will fail because it needs to make an HTTP request to a real Lightning address server
    let result = client
        .call_tool(CallToolRequestParam {
            name: "generate_invoice".into(),
            arguments: Some(
                json!({
                    "address": "test@example.com",
                    "amount_msats": 1000000,
                    "comment": "Test payment"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Should return an error without real Lightning address server
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        assert!(content_text.text.contains("Error"));
    }

    Ok(())
}

#[tokio::test]
async fn test_dca_report_tool_error_case() -> Result<()> {
    let client = connect_to_server().await?;

    // This will fail without a real backend, but tests the tool is available
    let result = client
        .call_tool(CallToolRequestParam {
            name: "dca_report".into(),
            arguments: Some(
                json!({
                    "descriptor": "wpkh([fingerprint/84'/0'/0']xpub...)",
                    "currency": "USD"
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        })
        .await?;

    // Should return an error without real backend
    if let Some(content) = &result.content {
        let content_text = content[0].as_text().unwrap();
        assert!(content_text.text.contains("Error"));
    }

    Ok(())
}
