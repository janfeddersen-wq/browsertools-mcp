//! CLI configuration — maps all ~30 flags via clap derive.

use std::path::PathBuf;

use clap::Parser;

/// BrowserTools MCP server — control and inspect Chrome from AI assistants.
#[derive(Parser, Debug, Clone)]
#[command(name = "browsertools-mcp", version, about)]
pub struct Config {
    // === Browser Launch Options ===
    /// Run Chrome in headless mode.
    #[arg(long, default_value_t = true)]
    pub headless: bool,

    /// Path to Chrome executable.
    #[arg(long)]
    pub executable_path: Option<PathBuf>,

    /// Chrome release channel.
    #[arg(long, default_value = "stable")]
    pub channel: String,

    /// Use a temporary isolated user profile.
    #[arg(long)]
    pub isolated: bool,

    /// Custom Chrome user data directory.
    #[arg(long)]
    pub user_data_dir: Option<PathBuf>,

    /// Initial viewport size (WxH).
    #[arg(long)]
    pub viewport: Option<String>,

    /// HTTP proxy server.
    #[arg(long)]
    pub proxy_server: Option<String>,

    /// Accept insecure SSL certificates.
    #[arg(long)]
    pub accept_insecure_certs: bool,

    /// Additional Chrome arguments.
    #[arg(long = "chrome-arg")]
    pub chrome_args: Vec<String>,

    /// Default Chrome arguments to skip.
    #[arg(long = "ignore-default-chrome-arg")]
    pub ignore_default_chrome_args: Vec<String>,

    // === Connection Options ===
    /// Connect to Chrome at this HTTP debug URL.
    #[arg(long)]
    pub browser_url: Option<String>,

    /// Connect to Chrome via WebSocket endpoint.
    #[arg(long)]
    pub ws_endpoint: Option<String>,

    /// Custom WebSocket headers (JSON).
    #[arg(long)]
    pub ws_headers: Option<String>,

    /// Auto-connect to Chrome 144+.
    #[arg(long)]
    pub auto_connect: bool,

    // === Feature Flags ===
    /// Enable slim mode (3 essential tools only).
    #[arg(long)]
    pub slim: bool,

    /// Enable experimental computer vision tools.
    #[arg(long)]
    pub experimental_vision: bool,

    /// Enable experimental screencast recording.
    #[arg(long)]
    pub experimental_screencast: bool,

    /// Enable experimental page ID routing.
    #[arg(long)]
    pub experimental_page_id_routing: bool,

    /// Enable experimental DevTools debugging.
    #[arg(long)]
    pub experimental_devtools: bool,

    // === Category Toggles ===
    /// Enable emulation tools.
    #[arg(long, default_value_t = true)]
    pub category_emulation: bool,

    /// Enable performance tools.
    #[arg(long, default_value_t = true)]
    pub category_performance: bool,

    /// Enable network tools.
    #[arg(long, default_value_t = true)]
    pub category_network: bool,

    /// Enable extension management tools.
    #[arg(long)]
    pub category_extensions: bool,

    /// Enable in-page tools.
    #[arg(long)]
    pub category_in_page_tools: bool,

    // === Performance ===
    /// Enable CrUX API for field performance data.
    #[arg(long)]
    pub performance_crux: bool,

    // === Lighthouse ===
    /// Path to lighthouse CLI executable.
    #[arg(long)]
    pub lighthouse_path: Option<PathBuf>,

    // === Developer Options ===
    /// Debug log file path.
    #[arg(long)]
    pub log_file: Option<PathBuf>,
}

impl Config {
    /// Parse the viewport string (e.g., "1280x720") into (width, height).
    pub fn viewport_size(&self) -> Option<(u32, u32)> {
        self.viewport.as_ref().and_then(|v| {
            let parts: Vec<&str> = v.split('x').collect();
            if parts.len() == 2 {
                let w = parts[0].parse().ok()?;
                let h = parts[1].parse().ok()?;
                Some((w, h))
            } else {
                None
            }
        })
    }

    /// Parse the Chrome channel string into the cdp_client Channel enum.
    pub fn chrome_channel(&self) -> cdp_client::Channel {
        match self.channel.to_lowercase().as_str() {
            "canary" => cdp_client::Channel::Canary,
            "beta" => cdp_client::Channel::Beta,
            "dev" => cdp_client::Channel::Dev,
            _ => cdp_client::Channel::Stable,
        }
    }
}
