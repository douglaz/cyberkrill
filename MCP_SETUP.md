# Cyberkrill MCP Server Setup

This guide explains how to configure and use the cyberkrill MCP (Model Context Protocol) server with Claude Code.

## What is MCP?

MCP (Model Context Protocol) is Anthropic's standard for exposing tools and commands to AI assistants. The cyberkrill MCP server allows Claude to directly interact with Bitcoin, Lightning Network, and Fedimint operations.

## Available Tools

The cyberkrill MCP server exposes 12 tools:

### Lightning Network Tools
- `decode_invoice` - Decode BOLT11 Lightning invoices
- `decode_lnurl` - Decode LNURL strings
- `generate_invoice` - Generate invoices from Lightning addresses

### Fedimint Tools
- `decode_fedimint_invite` - Decode Fedimint federation invite codes
- `encode_fedimint_invite` - Encode Fedimint invite codes from JSON

### Bitcoin Tools
- `list_utxos` - List UTXOs for descriptors or addresses
- `decode_psbt` - Decode Partially Signed Bitcoin Transactions
- `create_psbt` - Create PSBT with manual input/output specification
- `create_funded_psbt` - Create PSBT with automatic input selection
- `move_utxos` - Consolidate/move UTXOs to a single destination
- `dca_report` - Generate Dollar Cost Averaging reports

## Setup Instructions

### 1. Build the Project

First, build cyberkrill with the release profile:

```bash
cd /path/to/cyberkrill
cargo build --release
```

### 2. Configure MCP in Claude Code

There are three ways to configure the MCP server:

#### Option A: Project Scope (Recommended for Teams)

The `.mcp.json` file is already included in this repository. This configuration is shared with your team when you commit it to version control.

```json
{
  "mcpServers": {
    "cyberkrill": {
      "command": "cargo",
      "args": ["run", "--bin", "cyberkrill", "--", "mcp-server"],
      "env": {
        "RUST_LOG": "info"
      }
    }
  }
}
```

#### Option B: Local Scope (Private to You)

Add the server to your local Claude Code configuration:

```bash
claude mcp add cyberkrill /path/to/cyberkrill/target/release/cyberkrill mcp-server
```

#### Option C: User Scope (Available in All Projects)

Add the server globally for all your projects:

```bash
claude mcp add cyberkrill --scope user /path/to/cyberkrill/target/release/cyberkrill mcp-server
```

### 3. Using the Production Binary

For production use, build and use the release binary:

1. Build the release binary:
   ```bash
   cargo build --release
   ```

2. Update `.mcp.json` to use the release binary:
   ```json
   {
     "mcpServers": {
       "cyberkrill": {
         "command": "${HOME}/p/cyberkrill/target/release/cyberkrill",
         "args": ["mcp-server"],
         "env": {
           "RUST_LOG": "info"
         }
       }
     }
   }
   ```

### 4. Verify MCP Server is Running

After configuration, restart Claude Code and check if the MCP server is available:

```bash
# In Claude Code, you should see cyberkrill tools available
# You can ask Claude to list available MCP tools
```

## Usage Examples

Once configured, you can ask Claude to use cyberkrill tools directly:

```
"Decode this Lightning invoice: lnbc1..."
"List UTXOs for descriptor wpkh(...)"
"Create a PSBT to send 0.001 BTC to bc1q..."
"Generate a DCA report for my wallet"
```

## Configuration Options

### Environment Variables

You can pass environment variables to the MCP server:

```json
{
  "mcpServers": {
    "cyberkrill": {
      "command": "cyberkrill",
      "args": ["mcp-server"],
      "env": {
        "RUST_LOG": "debug",  // Set log level
        "BITCOIN_DIR": "/custom/bitcoin/dir",  // Custom Bitcoin directory
        "TAPSIGNER_CVC": "123456"  // For hardware wallet support
      }
    }
  }
}
```

### Transport Options

Currently, the MCP server supports stdio transport (default). SSE transport support is planned for future releases.

## Troubleshooting

### Server Not Starting

1. Check that cyberkrill is built:
   ```bash
   cargo build --release
   ```

2. Verify the path in `.mcp.json` is correct:
   ```bash
   ls -la target/release/cyberkrill
   ```

3. Check logs for errors:
   ```bash
   RUST_LOG=debug cargo run -- mcp-server
   ```

### Tools Not Available

1. Restart Claude Code after updating configuration
2. Check that `.mcp.json` is in the project root
3. Verify the MCP server process is running

### Permission Issues

Make sure the binary has execute permissions:
```bash
chmod +x target/release/cyberkrill
```

## Security Considerations

- The MCP server runs with your local user permissions
- Bitcoin RPC credentials are read from your local Bitcoin configuration
- Never commit sensitive data (API keys, passwords) to `.mcp.json`
- Use environment variables for sensitive configuration

## Development

To run the MCP server in development mode:

```bash
RUST_LOG=debug cargo run -- mcp-server
```

This will start the server with stdio transport and detailed logging.

## Future Enhancements

Planned improvements for the MCP server:
- SSE transport support for network access
- Hardware wallet tools (Tapsigner, Satscard, Coldcard)
- Transaction broadcasting capabilities
- WebSocket transport option
- Enhanced error reporting