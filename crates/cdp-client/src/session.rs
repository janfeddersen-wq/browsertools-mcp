//! CDP session management.
//!
//! Each browser target (page, service worker, etc.) gets its own session.
//! Sessions multiplex over the single WebSocket connection.

use std::sync::Arc;

use crate::connection::CdpConnection;
use crate::error::CdpResult;

/// A CDP session attached to a specific target.
///
/// Wraps the connection with a session ID for target-scoped commands.
#[derive(Clone)]
pub struct CdpSession {
    connection: Arc<CdpConnection>,
    session_id: String,
}

impl CdpSession {
    pub fn new(connection: Arc<CdpConnection>, session_id: String) -> Self {
        Self {
            connection,
            session_id,
        }
    }

    /// The session ID for this target.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Send a CDP command within this session's target.
    pub async fn send_command(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> CdpResult<serde_json::Value> {
        self.connection
            .send_command(method, params, Some(&self.session_id))
            .await
    }

    /// Subscribe to events within this session.
    pub async fn subscribe_events(
        &self,
        method: &str,
    ) -> tokio::sync::mpsc::UnboundedReceiver<serde_json::Value> {
        self.connection.subscribe_events(method).await
    }
}

/// The browser-level session (no session ID, commands go to the browser target).
#[derive(Clone)]
pub struct BrowserSession {
    connection: Arc<CdpConnection>,
}

impl BrowserSession {
    pub fn new(connection: Arc<CdpConnection>) -> Self {
        Self { connection }
    }

    /// Send a CDP command at the browser level (no session ID).
    pub async fn send_command(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> CdpResult<serde_json::Value> {
        self.connection.send_command(method, params, None).await
    }

    /// Attach to a target and get a session for it.
    pub async fn attach_to_target(&self, target_id: &str) -> CdpResult<CdpSession> {
        let result = self
            .send_command(
                "Target.attachToTarget",
                serde_json::json!({
                    "targetId": target_id,
                    "flatten": true,
                }),
            )
            .await?;

        let session_id = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                crate::error::CdpError::SessionNotFound(
                    "No sessionId in attachToTarget response".into(),
                )
            })?
            .to_string();

        Ok(CdpSession::new(self.connection.clone(), session_id))
    }

    /// Subscribe to browser-level events.
    pub async fn subscribe_events(
        &self,
        method: &str,
    ) -> tokio::sync::mpsc::UnboundedReceiver<serde_json::Value> {
        self.connection.subscribe_events(method).await
    }

    /// Get a reference to the underlying connection.
    pub fn connection(&self) -> &Arc<CdpConnection> {
        &self.connection
    }
}
