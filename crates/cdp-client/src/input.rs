//! CDP Input domain — mouse, keyboard, and touch input dispatching.

use crate::error::CdpResult;
use crate::session::CdpSession;

/// Dispatch a mouse click at given coordinates.
pub async fn click(session: &CdpSession, x: f64, y: f64, button: &str) -> CdpResult<()> {
    // Mouse pressed
    session
        .send_command(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mousePressed",
                "x": x,
                "y": y,
                "button": button,
                "clickCount": 1,
            }),
        )
        .await?;

    // Mouse released
    session
        .send_command(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mouseReleased",
                "x": x,
                "y": y,
                "button": button,
                "clickCount": 1,
            }),
        )
        .await?;

    Ok(())
}

/// Move the mouse to given coordinates (hover).
pub async fn hover(session: &CdpSession, x: f64, y: f64) -> CdpResult<()> {
    session
        .send_command(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mouseMoved",
                "x": x,
                "y": y,
            }),
        )
        .await?;
    Ok(())
}

/// Dispatch a key press event.
pub async fn press_key(session: &CdpSession, key: &str, modifiers: u32) -> CdpResult<()> {
    session
        .send_command(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": "keyDown",
                "key": key,
                "modifiers": modifiers,
            }),
        )
        .await?;

    session
        .send_command(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": "keyUp",
                "key": key,
                "modifiers": modifiers,
            }),
        )
        .await?;

    Ok(())
}

/// Type text character by character using insertText events.
pub async fn type_text(session: &CdpSession, text: &str) -> CdpResult<()> {
    for ch in text.chars() {
        session
            .send_command(
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyDown",
                    "text": ch.to_string(),
                }),
            )
            .await?;

        session
            .send_command(
                "Input.insertText",
                serde_json::json!({
                    "text": ch.to_string(),
                }),
            )
            .await?;

        session
            .send_command(
                "Input.dispatchKeyEvent",
                serde_json::json!({
                    "type": "keyUp",
                    "text": ch.to_string(),
                }),
            )
            .await?;
    }
    Ok(())
}

/// Dispatch a drag event from one point to another.
pub async fn drag(
    session: &CdpSession,
    from_x: f64,
    from_y: f64,
    to_x: f64,
    to_y: f64,
) -> CdpResult<()> {
    // Mouse down at source
    session
        .send_command(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mousePressed",
                "x": from_x,
                "y": from_y,
                "button": "left",
                "clickCount": 1,
            }),
        )
        .await?;

    // Mouse move to target
    session
        .send_command(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mouseMoved",
                "x": to_x,
                "y": to_y,
                "button": "left",
            }),
        )
        .await?;

    // Mouse up at target
    session
        .send_command(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mouseReleased",
                "x": to_x,
                "y": to_y,
                "button": "left",
                "clickCount": 1,
            }),
        )
        .await?;

    Ok(())
}

/// Scroll into view and get the center coordinates of an element by its backend node ID.
pub async fn get_element_center(
    session: &CdpSession,
    backend_node_id: i64,
) -> CdpResult<(f64, f64)> {
    // Resolve the node to get an object ID.
    let resolve_result = session
        .send_command(
            "DOM.resolveNode",
            serde_json::json!({ "backendNodeId": backend_node_id }),
        )
        .await?;

    let object_id = resolve_result
        .get("object")
        .and_then(|o| o.get("objectId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            crate::error::CdpError::ElementNotFound(format!(
                "Could not resolve node {backend_node_id}"
            ))
        })?
        .to_string();

    // Scroll the element into view.
    let _ = session
        .send_command(
            "DOM.scrollIntoViewIfNeeded",
            serde_json::json!({ "backendNodeId": backend_node_id }),
        )
        .await;

    // Get the box model to compute center coordinates.
    let box_result = session
        .send_command(
            "DOM.getBoxModel",
            serde_json::json!({ "objectId": object_id }),
        )
        .await?;

    let content = box_result
        .get("model")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .ok_or_else(|| {
            crate::error::CdpError::ElementNotFound("No box model for element".into())
        })?;

    // content quad is [x1, y1, x2, y2, x3, y3, x4, y4]
    if content.len() >= 8 {
        let xs: Vec<f64> = content
            .iter()
            .step_by(2)
            .filter_map(|v| v.as_f64())
            .collect();
        let ys: Vec<f64> = content
            .iter()
            .skip(1)
            .step_by(2)
            .filter_map(|v| v.as_f64())
            .collect();

        let center_x = xs.iter().sum::<f64>() / xs.len() as f64;
        let center_y = ys.iter().sum::<f64>() / ys.len() as f64;

        Ok((center_x, center_y))
    } else {
        Err(crate::error::CdpError::ElementNotFound(
            "Invalid box model coordinates".into(),
        ))
    }
}

/// Focus an element by its backend node ID.
pub async fn focus_element(session: &CdpSession, backend_node_id: i64) -> CdpResult<()> {
    session
        .send_command(
            "DOM.focus",
            serde_json::json!({ "backendNodeId": backend_node_id }),
        )
        .await?;
    Ok(())
}
