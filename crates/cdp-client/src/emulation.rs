//! CDP Emulation domain — device, CPU, and media emulation.

use crate::error::CdpResult;
use crate::session::CdpSession;

/// Set CPU throttling rate.
pub async fn set_cpu_throttling_rate(session: &CdpSession, rate: f64) -> CdpResult<()> {
    session
        .send_command(
            "Emulation.setCPUThrottlingRate",
            serde_json::json!({ "rate": rate }),
        )
        .await?;
    Ok(())
}

/// Set geolocation override.
pub async fn set_geolocation_override(
    session: &CdpSession,
    latitude: f64,
    longitude: f64,
    accuracy: Option<f64>,
) -> CdpResult<()> {
    session
        .send_command(
            "Emulation.setGeolocationOverride",
            serde_json::json!({
                "latitude": latitude,
                "longitude": longitude,
                "accuracy": accuracy.unwrap_or(1.0),
            }),
        )
        .await?;
    Ok(())
}

/// Set user agent override.
pub async fn set_user_agent_override(session: &CdpSession, user_agent: &str) -> CdpResult<()> {
    session
        .send_command(
            "Emulation.setUserAgentOverride",
            serde_json::json!({ "userAgent": user_agent }),
        )
        .await?;
    Ok(())
}

/// Set emulated media features (e.g., prefers-color-scheme).
pub async fn set_emulated_media(
    session: &CdpSession,
    features: &[(String, String)],
) -> CdpResult<()> {
    let features_json: Vec<serde_json::Value> = features
        .iter()
        .map(|(name, value)| {
            serde_json::json!({
                "name": name,
                "value": value,
            })
        })
        .collect();

    session
        .send_command(
            "Emulation.setEmulatedMedia",
            serde_json::json!({ "features": features_json }),
        )
        .await?;
    Ok(())
}

/// Set device metrics override (viewport).
pub async fn set_device_metrics_override(
    session: &CdpSession,
    width: u32,
    height: u32,
    device_scale_factor: f64,
    mobile: bool,
) -> CdpResult<()> {
    session
        .send_command(
            "Emulation.setDeviceMetricsOverride",
            serde_json::json!({
                "width": width,
                "height": height,
                "deviceScaleFactor": device_scale_factor,
                "mobile": mobile,
            }),
        )
        .await?;
    Ok(())
}

/// Clear device metrics override.
pub async fn clear_device_metrics_override(session: &CdpSession) -> CdpResult<()> {
    session
        .send_command(
            "Emulation.clearDeviceMetricsOverride",
            serde_json::json!({}),
        )
        .await?;
    Ok(())
}
