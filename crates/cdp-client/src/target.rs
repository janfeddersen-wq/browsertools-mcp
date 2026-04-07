//! CDP Target domain — page/target discovery and management.

use serde::{Deserialize, Serialize};

use crate::error::CdpResult;
use crate::session::BrowserSession;

/// Information about a browser target (page, iframe, service worker, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    pub target_id: String,
    #[serde(rename = "type")]
    pub target_type: String,
    pub title: String,
    pub url: String,
    pub attached: bool,
    pub browser_context_id: Option<String>,
}

/// Manages target discovery and lifecycle.
pub struct TargetManager;

impl TargetManager {
    /// Get all targets from the browser.
    pub async fn get_targets(session: &BrowserSession) -> CdpResult<Vec<TargetInfo>> {
        let result = session
            .send_command("Target.getTargets", serde_json::json!({}))
            .await?;

        let targets: Vec<TargetInfo> = serde_json::from_value(
            result
                .get("targetInfos")
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![])),
        )
        .map_err(|e| crate::error::CdpError::ParseError(e.to_string()))?;

        Ok(targets)
    }

    /// Create a new page target.
    pub async fn create_target(
        session: &BrowserSession,
        url: &str,
        background: bool,
    ) -> CdpResult<String> {
        let result = session
            .send_command(
                "Target.createTarget",
                serde_json::json!({
                    "url": url,
                    "background": background,
                }),
            )
            .await?;

        result
            .get("targetId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                crate::error::CdpError::TargetNotFound(
                    "No targetId in createTarget response".into(),
                )
            })
    }

    /// Close a target.
    pub async fn close_target(session: &BrowserSession, target_id: &str) -> CdpResult<()> {
        session
            .send_command(
                "Target.closeTarget",
                serde_json::json!({ "targetId": target_id }),
            )
            .await?;
        Ok(())
    }
}
