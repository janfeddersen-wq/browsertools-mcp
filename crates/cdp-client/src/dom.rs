//! CDP DOM domain — DOM node queries and manipulation.

use crate::error::{CdpError, CdpResult};
use crate::session::CdpSession;

/// Resolve a backend node ID to a remote object.
pub async fn resolve_node(
    session: &CdpSession,
    backend_node_id: i64,
) -> CdpResult<serde_json::Value> {
    let result = session
        .send_command(
            "DOM.resolveNode",
            serde_json::json!({ "backendNodeId": backend_node_id }),
        )
        .await?;

    result
        .get("object")
        .cloned()
        .ok_or_else(|| CdpError::ElementNotFound(format!("Cannot resolve node {backend_node_id}")))
}

/// Get the box model for an element.
pub async fn get_box_model(
    session: &CdpSession,
    backend_node_id: i64,
) -> CdpResult<serde_json::Value> {
    let result = session
        .send_command(
            "DOM.getBoxModel",
            serde_json::json!({ "backendNodeId": backend_node_id }),
        )
        .await?;

    result.get("model").cloned().ok_or_else(|| {
        CdpError::ElementNotFound(format!("No box model for node {backend_node_id}"))
    })
}

/// Scroll an element into view if needed.
pub async fn scroll_into_view_if_needed(
    session: &CdpSession,
    backend_node_id: i64,
) -> CdpResult<()> {
    session
        .send_command(
            "DOM.scrollIntoViewIfNeeded",
            serde_json::json!({ "backendNodeId": backend_node_id }),
        )
        .await?;
    Ok(())
}

/// Focus an element.
pub async fn focus(session: &CdpSession, backend_node_id: i64) -> CdpResult<()> {
    session
        .send_command(
            "DOM.focus",
            serde_json::json!({ "backendNodeId": backend_node_id }),
        )
        .await?;
    Ok(())
}

/// Enable DOM domain.
pub async fn enable(session: &CdpSession) -> CdpResult<()> {
    session
        .send_command("DOM.enable", serde_json::json!({}))
        .await?;
    Ok(())
}

/// Set a file input's files.
pub async fn set_file_input_files(
    session: &CdpSession,
    backend_node_id: i64,
    files: &[String],
) -> CdpResult<()> {
    session
        .send_command(
            "DOM.setFileInputFiles",
            serde_json::json!({
                "backendNodeId": backend_node_id,
                "files": files,
            }),
        )
        .await?;
    Ok(())
}
