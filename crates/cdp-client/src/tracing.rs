//! CDP Tracing domain — performance trace recording.

use crate::error::CdpResult;
use crate::session::CdpSession;

/// Start recording a performance trace.
pub async fn start(
    session: &CdpSession,
    categories: Option<&str>,
    buffer_usage_reporting_interval: Option<f64>,
) -> CdpResult<()> {
    let mut params = serde_json::json!({});

    if let Some(cats) = categories {
        params["categories"] = serde_json::Value::String(cats.to_string());
    }
    if let Some(interval) = buffer_usage_reporting_interval {
        params["bufferUsageReportingInterval"] = serde_json::json!(interval);
    }

    session.send_command("Tracing.start", params).await?;
    Ok(())
}

/// Stop the current trace recording.
pub async fn stop(session: &CdpSession) -> CdpResult<()> {
    session
        .send_command("Tracing.end", serde_json::json!({}))
        .await?;
    Ok(())
}
