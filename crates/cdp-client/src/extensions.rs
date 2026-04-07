//! CDP Extensions domain — Chrome extension management.

use crate::error::CdpResult;
use crate::session::BrowserSession;

/// Load an unpacked extension from a directory.
pub async fn load_unpacked(session: &BrowserSession, path: &str) -> CdpResult<String> {
    let result = session
        .send_command(
            "Extensions.loadUnpacked",
            serde_json::json!({ "path": path }),
        )
        .await?;

    Ok(result
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string())
}

/// Uninstall an extension by ID.
pub async fn uninstall(session: &BrowserSession, id: &str) -> CdpResult<()> {
    session
        .send_command("Extensions.uninstall", serde_json::json!({ "id": id }))
        .await?;
    Ok(())
}

/// Trigger an extension's action on a specific tab.
pub async fn trigger_action(session: &BrowserSession, id: &str, target_id: &str) -> CdpResult<()> {
    session
        .send_command(
            "Extensions.triggerAction",
            serde_json::json!({
                "id": id,
                "targetId": target_id,
            }),
        )
        .await?;
    Ok(())
}
