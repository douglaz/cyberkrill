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

### 1. Install cyberkrill

First, install cyberkrill to your system. You can either:

**Option A: Install to ~/bin (Recommended)**
```bash
# Build and copy to ~/bin
cd /path/to/cyberkrill
cargo build --release
cp target/release/cyberkrill ~/bin/
```

**Option B: Install via cargo**
```bash
cargo install --path cyberkrill
```

### 2. Configure MCP with Claude Code CLI

Use the Claude Code CLI to add the cyberkrill MCP server:

#### Global Setup (Recommended - Available in All Projects)

```bash
# Add cyberkrill MCP server globally for all projects
claude mcp add cyberkrill ~/bin/cyberkrill mcp-server -s user -e RUST_LOG=info
```

This makes cyberkrill tools available in every project without additional configuration.

#### Project-Specific Setup (For Team Sharing)

If you want to share the configuration with your team via version control:

```bash
# Add to project configuration
claude mcp add cyberkrill cyberkrill mcp-server -s project -e RUST_LOG=info
```

This creates/updates `.mcp.json` in your project root that can be committed to version control.

### 3. Verify MCP Server Connection

Check that the MCP server is properly connected:

```bash
# List all configured MCP servers and their status
claude mcp list
```

You should see:
```
Checking MCP server health...
cyberkrill: ~/bin/cyberkrill mcp-server - ✓ Connected
```

### 4. Manage MCP Servers

**View configuration:**
```bash
# Show all MCP servers across different scopes
claude mcp list
```

**Remove a server:**
```bash
# Remove from specific scope
claude mcp remove cyberkrill -s user    # Remove from global config
claude mcp remove cyberkrill -s project  # Remove from project config
claude mcp remove cyberkrill -s local    # Remove from local config
```

**Update configuration:**
```bash
# Remove and re-add with new settings
claude mcp remove cyberkrill -s user
claude mcp add cyberkrill ~/bin/cyberkrill mcp-server -s user -e RUST_LOG=debug -e BITCOIN_DIR=/custom/path
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

### Server Not Connecting

1. **Check binary exists and is executable:**
   ```bash
   ls -la ~/bin/cyberkrill
   # Should show executable permissions (x)
   ```

2. **Test the MCP server directly:**
   ```bash
   # This should start without errors
   ~/bin/cyberkrill mcp-server
   # Press Ctrl+C to stop
   ```

3. **Check MCP connection status:**
   ```bash
   claude mcp list
   ```

### Common Issues

**"Failed to connect" error:**
- Make sure you're using the full path to the binary (e.g., `~/bin/cyberkrill` or `/home/user/bin/cyberkrill`)
- The relative path `cyberkrill` only works if it's in your PATH

**Multiple scope conflicts:**
- If you have the same server in multiple scopes, remove duplicates:
  ```bash
  claude mcp remove cyberkrill -s project
  claude mcp remove cyberkrill -s local
  # Keep only the user scope for global access
  ```

**Binary not found:**
- Ensure cyberkrill is built: `cargo build --release`
- Copy to ~/bin: `cp target/release/cyberkrill ~/bin/`
- Make executable: `chmod +x ~/bin/cyberkrill`

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