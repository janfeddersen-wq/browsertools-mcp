//! CDP Network domain — request/response interception and monitoring.

use crate::error::CdpResult;
use crate::session::CdpSession;

/// Enable network event tracking.
pub async fn enable(session: &CdpSession) -> CdpResult<()> {
    session
        .send_command("Network.enable", serde_json::json!({}))
        .await?;
    Ok(())
}

/// Disable network event tracking.
pub async fn disable(session: &CdpSession) -> CdpResult<()> {
    session
        .send_command("Network.disable", serde_json::json!({}))
        .await?;
    Ok(())
}

/// Get the response body for a given request ID.
pub async fn get_response_body(
    session: &CdpSession,
    request_id: &str,
) -> CdpResult<(String, bool)> {
    let result = session
        .send_command(
            "Network.getResponseBody",
            serde_json::json!({ "requestId": request_id }),
        )
        .await?;

    let body = result
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let base64_encoded = result
        .get("base64Encoded")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok((body, base64_encoded))
}

/// Emulate network conditions (throttling, offline).
pub async fn emulate_network_conditions(
    session: &CdpSession,
    offline: bool,
    latency: f64,
    download_throughput: f64,
    upload_throughput: f64,
) -> CdpResult<()> {
    session
        .send_command(
            "Network.emulateNetworkConditions",
            serde_json::json!({
                "offline": offline,
                "latency": latency,
                "downloadThroughput": download_throughput,
                "uploadThroughput": upload_throughput,
            }),
        )
        .await?;
    Ok(())
}
