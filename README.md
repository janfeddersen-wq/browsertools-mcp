# rust-chrome-mcp

A Chrome DevTools MCP server written in Rust. Single binary, no Node.js required.

Control and inspect Chrome browsers from AI assistants (Claude Code, Cursor, etc.) via the [Model Context Protocol](https://modelcontextprotocol.io/).

## Features

- **35 MCP tools** for browser automation, debugging, and performance analysis
- **Single binary** (5.8MB) — no runtime dependencies
- **Headless or headed** Chrome control
- Accessibility tree snapshots with stable element UIDs
- Screenshots (PNG/JPEG/WebP)
- JavaScript evaluation
- Network request inspection with timing
- Console message collection with stack traces
- Device/network/CPU emulation
- Performance trace recording with Web Vitals (LCP, CLS, FCP, TBT)
- Chrome extension management
- Lighthouse audits (optional, shells out to `lighthouse` CLI)
- Slim mode (`--slim`) with 3 essential tools

## Quick Start

```bash
# Build
cargo build --release

# Run with headless Chrome
./target/release/chrome-devtools-mcp --headless

# Connect to an existing Chrome instance
./target/release/chrome-devtools-mcp --ws-endpoint ws://localhost:9222/devtools/browser/...

# Slim mode (3 tools only)
./target/release/chrome-devtools-mcp --headless --slim
```

## MCP Client Configuration

```json
{
  "mcpServers": {
    "chrome-devtools": {
      "command": "/path/to/chrome-devtools-mcp",
      "args": ["--headless", "--isolated"]
    }
  }
}
```

## Tools

### Page Management
`list_pages` `select_page` `new_page` `navigate_page` `close_page` `wait_for`

### Input Automation
`click` `click_at` `drag` `fill` `fill_form` `hover` `press_key` `type_text` `upload_file`

### Debugging
`evaluate_script` `take_screenshot` `take_snapshot` `list_console_messages` `get_console_message` `lighthouse_audit`

### Network
`list_network_requests` `get_network_request`

### Emulation
`emulate` `resize_page`

### Performance
`performance_start_trace` `performance_stop_trace` `performance_analyze_insight` `take_memory_snapshot`

### Extensions
`install_extension` `uninstall_extension` `list_extensions` `reload_extension` `trigger_extension_action`

### Other
`handle_dialog`

## CLI Flags

```
--headless              Run Chrome in headless mode
--executable-path       Path to Chrome executable
--channel               Chrome channel (stable/beta/dev/canary)
--isolated              Use temporary isolated profile
--user-data-dir         Custom Chrome profile directory
--viewport              Initial viewport (WxH, e.g., 1280x720)
--proxy-server          HTTP proxy
--accept-insecure-certs Allow self-signed SSL
--browser-url           Connect via HTTP debug URL
--ws-endpoint           Connect via WebSocket endpoint
--auto-connect          Auto-connect to Chrome 144+
--slim                  Slim mode (3 tools)
--log-file              Debug log file path
```

## Architecture

Cargo workspace with 4 crates:

| Crate | Purpose |
|---|---|
| `cdp-client` | Chrome DevTools Protocol over WebSocket |
| `mcp-tools` | MCP server, 35 tool handlers, formatters, collectors |
| `trace-engine` | Chrome trace parsing, Web Vitals extraction |
| `cli` | Binary entry point, CLI config |

## License

**CC-BY-NC-4.0** — Free for personal and non-commercial use.

Commercial use requires a license. Organizations with >$1M annual revenue must contact the author.

See [LICENSE](LICENSE) for details.
