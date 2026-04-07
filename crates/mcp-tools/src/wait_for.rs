//! Wait utilities — navigation detection and DOM stability polling.

use std::time::Duration;

use cdp_client::CdpPage;

/// Wait for specific text to appear on a page.
pub async fn wait_for_text(
    page: &CdpPage,
    texts: &[String],
    timeout: Duration,
) -> anyhow::Result<()> {
    let text_checks: Vec<String> = texts
        .iter()
        .map(|t| {
            format!(
                "document.body && document.body.innerText.includes({})",
                serde_json::to_string(t).unwrap_or_default()
            )
        })
        .collect();

    let check_expression = text_checks.join(" || ");

    let start = std::time::Instant::now();
    loop {
        if start.elapsed() > timeout {
            anyhow::bail!(
                "Timeout waiting for text {:?} after {}ms",
                texts,
                timeout.as_millis()
            );
        }

        match page.evaluate(&check_expression).await {
            Ok(result) => {
                if result.as_bool() == Some(true) {
                    return Ok(());
                }
            }
            Err(_) => {
                // Page might be navigating, retry.
            }
        }

        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

/// Get the network throttling timeout multiplier based on network conditions.
pub fn get_network_multiplier(conditions: Option<&str>) -> f64 {
    match conditions {
        Some("Slow 3G") => 5.0,
        Some("Fast 3G") => 3.0,
        Some("Slow 4G") => 2.0,
        Some("Offline") => 1.0,
        _ => 1.0,
    }
}
