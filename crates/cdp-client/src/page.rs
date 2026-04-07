//! CDP Page abstraction — navigation, lifecycle, and page-level operations.

use crate::error::{CdpError, CdpResult};
use crate::session::CdpSession;

/// A handle to a browser page (tab) via its CDP session.
#[derive(Clone)]
pub struct CdpPage {
    session: CdpSession,
    target_id: String,
    url: String,
}

impl CdpPage {
    pub fn new(session: CdpSession, target_id: String, url: String) -> Self {
        Self {
            session,
            target_id,
            url,
        }
    }

    pub fn target_id(&self) -> &str {
        &self.target_id
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn session(&self) -> &CdpSession {
        &self.session
    }

    /// Navigate to a URL.
    pub async fn navigate(&mut self, url: &str) -> CdpResult<()> {
        let result = self
            .session
            .send_command("Page.navigate", serde_json::json!({ "url": url }))
            .await?;

        if let Some(error_text) = result.get("errorText").and_then(|v| v.as_str())
            && !error_text.is_empty()
        {
            return Err(CdpError::NavigationFailed(error_text.to_string()));
        }

        self.url = url.to_string();
        Ok(())
    }

    /// Reload the current page.
    pub async fn reload(&self) -> CdpResult<()> {
        self.session
            .send_command("Page.reload", serde_json::json!({}))
            .await?;
        Ok(())
    }

    /// Go back in history.
    pub async fn go_back(&self) -> CdpResult<()> {
        let history = self
            .session
            .send_command("Page.getNavigationHistory", serde_json::json!({}))
            .await?;

        let current_index = history
            .get("currentIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        if current_index > 0 {
            let entries = history
                .get("entries")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if let Some(entry) = entries.get((current_index - 1) as usize)
                && let Some(entry_id) = entry.get("id").and_then(|v| v.as_i64())
            {
                self.session
                    .send_command(
                        "Page.navigateToHistoryEntry",
                        serde_json::json!({ "entryId": entry_id }),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    /// Go forward in history.
    pub async fn go_forward(&self) -> CdpResult<()> {
        let history = self
            .session
            .send_command("Page.getNavigationHistory", serde_json::json!({}))
            .await?;

        let current_index = history
            .get("currentIndex")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let entries = history
            .get("entries")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        if let Some(entry) = entries.get((current_index + 1) as usize)
            && let Some(entry_id) = entry.get("id").and_then(|v| v.as_i64())
        {
            self.session
                .send_command(
                    "Page.navigateToHistoryEntry",
                    serde_json::json!({ "entryId": entry_id }),
                )
                .await?;
        }
        Ok(())
    }

    /// Capture a screenshot and return the base64-encoded image data.
    pub async fn capture_screenshot(
        &self,
        format: &str,
        quality: Option<u32>,
        clip: Option<serde_json::Value>,
        full_page: bool,
    ) -> CdpResult<String> {
        let mut params = serde_json::json!({
            "format": format,
            "captureBeyondViewport": full_page,
        });

        if let Some(q) = quality {
            params["quality"] = serde_json::Value::Number(q.into());
        }
        if let Some(c) = clip {
            params["clip"] = c;
        }

        let result = self
            .session
            .send_command("Page.captureScreenshot", params)
            .await?;

        result
            .get("data")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| CdpError::ScreenshotFailed("No data in response".into()))
    }

    /// Enable the Page domain events.
    pub async fn enable(&self) -> CdpResult<()> {
        self.session
            .send_command("Page.enable", serde_json::json!({}))
            .await?;
        Ok(())
    }

    /// Evaluate JavaScript in the page context.
    pub async fn evaluate(&self, expression: &str) -> CdpResult<serde_json::Value> {
        let result = self
            .session
            .send_command(
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true,
                }),
            )
            .await?;

        if let Some(exception) = result.get("exceptionDetails") {
            let text = exception
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown error");
            return Err(CdpError::EvaluationFailed(text.to_string()));
        }

        Ok(result
            .get("result")
            .and_then(|r| r.get("value"))
            .cloned()
            .unwrap_or(serde_json::Value::Null))
    }

    /// Set the default timeout for navigation.
    pub fn set_default_timeout(&mut self, _timeout_ms: u64) {
        // Stored for future use in navigation waits.
    }
}
