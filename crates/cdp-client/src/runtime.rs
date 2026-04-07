//! CDP Runtime domain — JavaScript evaluation and object interaction.

use crate::error::{CdpError, CdpResult};
use crate::session::CdpSession;

/// Evaluate a JavaScript expression in the page context.
pub async fn evaluate(
    session: &CdpSession,
    expression: &str,
    return_by_value: bool,
    await_promise: bool,
) -> CdpResult<serde_json::Value> {
    let result = session
        .send_command(
            "Runtime.evaluate",
            serde_json::json!({
                "expression": expression,
                "returnByValue": return_by_value,
                "awaitPromise": await_promise,
            }),
        )
        .await?;

    if let Some(exception) = result.get("exceptionDetails") {
        let text = exception
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("Evaluation failed");
        return Err(CdpError::EvaluationFailed(text.to_string()));
    }

    Ok(result
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

/// Call a function on a remote object.
pub async fn call_function_on(
    session: &CdpSession,
    object_id: &str,
    function_declaration: &str,
    arguments: &[serde_json::Value],
    return_by_value: bool,
) -> CdpResult<serde_json::Value> {
    let args: Vec<serde_json::Value> = arguments
        .iter()
        .map(|a| serde_json::json!({ "value": a }))
        .collect();

    let result = session
        .send_command(
            "Runtime.callFunctionOn",
            serde_json::json!({
                "objectId": object_id,
                "functionDeclaration": function_declaration,
                "arguments": args,
                "returnByValue": return_by_value,
                "awaitPromise": true,
            }),
        )
        .await?;

    if let Some(exception) = result.get("exceptionDetails") {
        let text = exception
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("Call failed");
        return Err(CdpError::EvaluationFailed(text.to_string()));
    }

    Ok(result
        .get("result")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

/// Enable the Runtime domain.
pub async fn enable(session: &CdpSession) -> CdpResult<()> {
    session
        .send_command("Runtime.enable", serde_json::json!({}))
        .await?;
    Ok(())
}
