use thiserror::Error;

/// Errors that can occur in the CDP client.
#[derive(Debug, Error)]
pub enum CdpError {
    #[error("WebSocket connection failed: {0}")]
    ConnectionFailed(String),

    #[error("WebSocket connection closed unexpectedly")]
    ConnectionClosed,

    #[error("Failed to send CDP command: {0}")]
    SendFailed(String),

    #[error("CDP command timed out after {timeout_ms}ms: {method}")]
    Timeout { method: String, timeout_ms: u64 },

    #[error("CDP protocol error (code {code}): {message}")]
    ProtocolError { code: i64, message: String },

    #[error("Failed to parse CDP response: {0}")]
    ParseError(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Page not found: {0}")]
    PageNotFound(String),

    #[error("Browser launch failed: {0}")]
    BrowserLaunchFailed(String),

    #[error("Chrome executable not found. Provide --executable-path or install Chrome")]
    ChromeNotFound,

    #[error("Target not found: {0}")]
    TargetNotFound(String),

    #[error("Navigation failed: {0}")]
    NavigationFailed(String),

    #[error("JavaScript evaluation failed: {0}")]
    EvaluationFailed(String),

    #[error("Screenshot failed: {0}")]
    ScreenshotFailed(String),

    #[error("Element not found for uid: {0}")]
    ElementNotFound(String),

    #[error("{0}")]
    Other(String),
}

impl From<tokio_tungstenite::tungstenite::Error> for CdpError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        CdpError::ConnectionFailed(err.to_string())
    }
}

impl From<serde_json::Error> for CdpError {
    fn from(err: serde_json::Error) -> Self {
        CdpError::ParseError(err.to_string())
    }
}

impl From<url::ParseError> for CdpError {
    fn from(err: url::ParseError) -> Self {
        CdpError::ConnectionFailed(format!("Invalid URL: {err}"))
    }
}

pub type CdpResult<T> = Result<T, CdpError>;
