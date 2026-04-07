//! Browser launch and connection management.
//!
//! Handles finding Chrome, launching it with the right flags,
//! and connecting to its DevTools WebSocket endpoint.

use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;

use tokio::io::AsyncBufReadExt;
use tokio::process::{Child, Command};

use crate::connection::CdpConnection;
use crate::error::{CdpError, CdpResult};
use crate::session::BrowserSession;

/// Chrome release channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Channel {
    Stable,
    Beta,
    Dev,
    Canary,
}

/// Configuration for launching a new Chrome instance.
#[derive(Debug, Clone)]
pub struct LaunchConfig {
    pub executable_path: Option<PathBuf>,
    pub channel: Channel,
    pub headless: bool,
    pub viewport: Option<(u32, u32)>,
    pub user_data_dir: Option<PathBuf>,
    pub isolated: bool,
    pub chrome_args: Vec<String>,
    pub ignore_default_chrome_args: Vec<String>,
    pub accept_insecure_certs: bool,
    pub enable_extensions: bool,
    pub devtools: bool,
    pub proxy_server: Option<String>,
}

impl Default for LaunchConfig {
    fn default() -> Self {
        Self {
            executable_path: None,
            channel: Channel::Stable,
            headless: true,
            viewport: None,
            user_data_dir: None,
            isolated: false,
            chrome_args: Vec::new(),
            ignore_default_chrome_args: Vec::new(),
            accept_insecure_certs: false,
            enable_extensions: false,
            devtools: false,
            proxy_server: None,
        }
    }
}

/// Configuration for connecting to an existing Chrome instance.
#[derive(Debug, Clone)]
pub struct ConnectConfig {
    pub browser_url: Option<String>,
    pub ws_endpoint: Option<String>,
    pub ws_headers: Option<serde_json::Value>,
    pub channel: Option<Channel>,
    pub user_data_dir: Option<PathBuf>,
    pub devtools: bool,
}

/// A managed Chrome browser instance.
pub struct Browser {
    session: BrowserSession,
    _process: Option<Child>,
    _temp_dir: Option<tempfile::TempDir>,
}

impl Browser {
    /// Launch a new Chrome instance.
    pub async fn launch(config: LaunchConfig) -> CdpResult<Self> {
        let chrome_path = find_chrome(&config)?;
        tracing::info!(path = %chrome_path.display(), "Launching Chrome");

        let temp_dir = if config.isolated {
            Some(tempfile::tempdir().map_err(|e| {
                CdpError::BrowserLaunchFailed(format!("Failed to create temp dir: {e}"))
            })?)
        } else {
            None
        };

        let user_data_dir = config
            .user_data_dir
            .clone()
            .or_else(|| temp_dir.as_ref().map(|d| d.path().to_path_buf()));

        let mut args = build_chrome_args(&config, user_data_dir.as_deref());

        // Use remote-debugging-port=0 to let Chrome pick a free port.
        args.push("--remote-debugging-port=0".to_string());

        let mut cmd = Command::new(&chrome_path);
        cmd.args(&args).stderr(Stdio::piped()).stdout(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| {
            CdpError::BrowserLaunchFailed(format!(
                "Failed to start Chrome at {}: {e}",
                chrome_path.display()
            ))
        })?;

        // Read stderr to find the DevTools WebSocket URL.
        let stderr = child.stderr.take().ok_or_else(|| {
            CdpError::BrowserLaunchFailed("Failed to capture Chrome stderr".into())
        })?;

        let ws_url = extract_ws_url(stderr).await?;
        tracing::info!(url = %ws_url, "Chrome DevTools WebSocket URL");

        let connection = Arc::new(CdpConnection::connect(&ws_url).await?);
        let session = BrowserSession::new(connection);

        Ok(Self {
            session,
            _process: Some(child),
            _temp_dir: temp_dir,
        })
    }

    /// Connect to an already-running Chrome instance.
    pub async fn connect(config: ConnectConfig) -> CdpResult<Self> {
        let ws_url = if let Some(ref ws) = config.ws_endpoint {
            ws.clone()
        } else if let Some(ref browser_url) = config.browser_url {
            discover_ws_endpoint(browser_url).await?
        } else {
            return Err(CdpError::ConnectionFailed(
                "No ws-endpoint or browser-url provided".into(),
            ));
        };

        tracing::info!(url = %ws_url, "Connecting to existing Chrome");
        let connection = Arc::new(CdpConnection::connect(&ws_url).await?);
        let session = BrowserSession::new(connection);

        Ok(Self {
            session,
            _process: None,
            _temp_dir: None,
        })
    }

    /// Get the browser-level CDP session.
    pub fn session(&self) -> &BrowserSession {
        &self.session
    }
}

/// Find the Chrome executable on the system.
fn find_chrome(config: &LaunchConfig) -> CdpResult<PathBuf> {
    if let Some(ref path) = config.executable_path {
        return Ok(path.clone());
    }

    let candidates = match config.channel {
        Channel::Canary => chrome_canary_candidates(),
        Channel::Beta => chrome_beta_candidates(),
        Channel::Dev => chrome_dev_candidates(),
        Channel::Stable => chrome_stable_candidates(),
    };

    for candidate in &candidates {
        if let Ok(path) = which::which(candidate) {
            return Ok(path);
        }
    }

    // Try platform-specific known paths.
    for candidate in &candidates {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Ok(path);
        }
    }

    Err(CdpError::ChromeNotFound)
}

fn chrome_stable_candidates() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec![
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "google-chrome",
            "chrome",
        ]
    } else if cfg!(target_os = "windows") {
        vec![
            r"C:\Program Files\Google\Chrome\Application\chrome.exe",
            r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
        ]
    } else {
        vec![
            "google-chrome-stable",
            "google-chrome",
            "chrome",
            "chromium-browser",
            "chromium",
        ]
    }
}

fn chrome_canary_candidates() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec!["/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary"]
    } else if cfg!(target_os = "windows") {
        vec![r"C:\Users\*\AppData\Local\Google\Chrome SxS\Application\chrome.exe"]
    } else {
        vec!["google-chrome-unstable"]
    }
}

fn chrome_beta_candidates() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec!["/Applications/Google Chrome Beta.app/Contents/MacOS/Google Chrome Beta"]
    } else {
        vec!["google-chrome-beta"]
    }
}

fn chrome_dev_candidates() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec!["/Applications/Google Chrome Dev.app/Contents/MacOS/Google Chrome Dev"]
    } else {
        vec!["google-chrome-dev", "google-chrome-unstable"]
    }
}

fn build_chrome_args(
    config: &LaunchConfig,
    user_data_dir: Option<&std::path::Path>,
) -> Vec<String> {
    let mut args = Vec::new();

    if config.headless {
        args.push("--headless=new".to_string());
    }

    if let Some(dir) = user_data_dir {
        args.push(format!("--user-data-dir={}", dir.display()));
    }

    if let Some((w, h)) = config.viewport {
        args.push(format!("--window-size={w},{h}"));
    }

    if config.accept_insecure_certs {
        args.push("--ignore-certificate-errors".to_string());
    }

    if !config.enable_extensions {
        args.push("--disable-extensions".to_string());
    }

    if let Some(ref proxy) = config.proxy_server {
        args.push(format!("--proxy-server={proxy}"));
    }

    // Default args for automation.
    let defaults = [
        "--disable-background-networking",
        "--disable-background-timer-throttling",
        "--disable-backgrounding-occluded-windows",
        "--disable-breakpad",
        "--disable-component-extensions-with-background-pages",
        "--disable-component-update",
        "--disable-default-apps",
        "--disable-dev-shm-usage",
        "--disable-hang-monitor",
        "--disable-ipc-flooding-protection",
        "--disable-popup-blocking",
        "--disable-prompt-on-repost",
        "--disable-renderer-backgrounding",
        "--disable-sync",
        "--enable-features=NetworkService,NetworkServiceInProcess",
        "--force-color-profile=srgb",
        "--metrics-recording-only",
        "--no-first-run",
        "--password-store=basic",
        "--use-mock-keychain",
    ];

    for default in &defaults {
        let flag_name = default.split('=').next().unwrap_or(default);
        if !config
            .ignore_default_chrome_args
            .iter()
            .any(|a| a == flag_name || a == *default)
        {
            args.push((*default).to_string());
        }
    }

    // Append user-specified extra args.
    args.extend(config.chrome_args.iter().cloned());

    args
}

/// Read Chrome's stderr to extract the DevTools WebSocket URL.
async fn extract_ws_url(stderr: tokio::process::ChildStderr) -> CdpResult<String> {
    let reader = tokio::io::BufReader::new(stderr);
    let mut lines = reader.lines();

    let timeout = tokio::time::Duration::from_secs(30);
    let result = tokio::time::timeout(timeout, async {
        while let Ok(Some(line)) = lines.next_line().await {
            tracing::debug!(line = %line, "Chrome stderr");
            if let Some(url) = line.strip_prefix("DevTools listening on ") {
                return Ok(url.trim().to_string());
            }
        }
        Err(CdpError::BrowserLaunchFailed(
            "Chrome closed without providing DevTools URL".into(),
        ))
    })
    .await;

    match result {
        Ok(url) => url,
        Err(_) => Err(CdpError::BrowserLaunchFailed(
            "Timed out waiting for Chrome DevTools URL (30s)".into(),
        )),
    }
}

/// Discover the WebSocket endpoint from a Chrome HTTP debug URL.
async fn discover_ws_endpoint(browser_url: &str) -> CdpResult<String> {
    let version_url = format!("{}/json/version", browser_url.trim_end_matches('/'));
    tracing::debug!(url = %version_url, "Discovering WebSocket endpoint");

    let body = reqwest::get(&version_url)
        .await
        .map_err(|e| {
            CdpError::ConnectionFailed(format!("HTTP request to {version_url} failed: {e}"))
        })?
        .text()
        .await
        .map_err(|e| CdpError::ConnectionFailed(format!("Failed to read response body: {e}")))?;

    let json: serde_json::Value = serde_json::from_str(&body)?;
    json.get("webSocketDebuggerUrl")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            CdpError::ConnectionFailed("No webSocketDebuggerUrl in /json/version response".into())
        })
}
