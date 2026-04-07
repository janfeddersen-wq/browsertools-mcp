//! BrowserTools MCP server — binary entry point.
//!
//! Parses CLI args, launches/connects to Chrome, starts the MCP server
//! on stdio transport, and handles graceful shutdown.

use std::sync::Arc;

use clap::Parser;
use rmcp::ServiceExt;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;

mod config;

use config::Config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::parse();

    // Initialize logging (always to stderr so stdout is reserved for MCP stdio transport).
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if let Some(ref log_file) = config.log_file {
        // Log to file if specified.
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)?;

        tracing_subscriber::fmt()
            .with_env_filter(
                EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug")),
            )
            .with_writer(file)
            .with_ansi(false)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(std::io::stderr)
            .init();
    }

    // Print disclaimers to stderr.
    print_disclaimers(&config);

    tracing::info!(version = VERSION, "Starting BrowserTools MCP server");

    // Launch or connect to Chrome.
    let browser =
        if config.browser_url.is_some() || config.ws_endpoint.is_some() || config.auto_connect {
            cdp_client::Browser::connect(cdp_client::ConnectConfig {
                browser_url: config.browser_url.clone(),
                ws_endpoint: config.ws_endpoint.clone(),
                ws_headers: config
                    .ws_headers
                    .as_ref()
                    .and_then(|h| serde_json::from_str(h).ok()),
                channel: if config.auto_connect {
                    Some(config.chrome_channel())
                } else {
                    None
                },
                user_data_dir: config.user_data_dir.clone(),
                devtools: config.experimental_devtools,
            })
            .await?
        } else {
            cdp_client::Browser::launch(cdp_client::LaunchConfig {
                executable_path: config.executable_path.clone(),
                channel: config.chrome_channel(),
                headless: config.headless,
                viewport: config.viewport_size(),
                user_data_dir: config.user_data_dir.clone(),
                isolated: config.isolated,
                chrome_args: config.chrome_args.clone(),
                ignore_default_chrome_args: config.ignore_default_chrome_args.clone(),
                accept_insecure_certs: config.accept_insecure_certs,
                enable_extensions: config.category_extensions,
                devtools: config.experimental_devtools,
                proxy_server: config.proxy_server.clone(),
            })
            .await?
        };

    let browser = Arc::new(browser);

    // Create the MCP context.
    let context = mcp_tools::McpContext::new(browser).await?;
    let shared_context = Arc::new(Mutex::new(context));

    tracing::info!("MCP server ready, starting stdio transport");

    // Wire up the rmcp stdio transport.
    let transport = rmcp::transport::io::stdio();

    if config.slim {
        tracing::info!("Starting in slim mode (3 tools)");
        let server = mcp_tools::SlimServer::new(shared_context);
        let running = server.serve(transport).await.inspect_err(|e| {
            tracing::error!(error = %e, "MCP server initialization failed");
        })?;
        tracing::info!("MCP server running on stdio (slim mode)");
        running.waiting().await?;
    } else {
        let server = mcp_tools::BrowserToolsServer::new(shared_context);
        let running = server.serve(transport).await.inspect_err(|e| {
            tracing::error!(error = %e, "MCP server initialization failed");
        })?;
        tracing::info!("MCP server running on stdio");
        running.waiting().await?;
    }

    tracing::info!("BrowserTools MCP server shutting down");
    Ok(())
}

fn print_disclaimers(config: &Config) {
    eprintln!(
        "browsertools-mcp v{VERSION} exposes browser content to MCP clients.\n\
         Avoid sharing sensitive information you do not want to share with MCP clients."
    );

    if !config.slim && config.performance_crux {
        eprintln!(
            "Performance tools may send trace URLs to the Google CrUX API. \
             To disable, run with --no-performance-crux."
        );
    }
}
