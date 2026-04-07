//! SlimServer -- a minimal MCP server with 3 essential tools.
//!
//! Provides: take_screenshot, navigate_page, evaluate_script.
//! Used when --slim mode is enabled.

use std::sync::Arc;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::context::McpContext;

/// Thread-safe shared context.
pub type SharedContext = Arc<Mutex<McpContext>>;

// ---------------------------------------------------------------------------
// Parameter types (reuse minimal set)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SlimNavigateParams {
    /// URL to navigate to, or "back", "forward", "reload".
    pub url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SlimEvaluateParams {
    /// JavaScript expression to evaluate.
    pub expression: String,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct SlimScreenshotParams {
    /// Image format: "png", "jpeg", or "webp".
    #[serde(default = "default_format")]
    pub format: String,
}

fn default_format() -> String {
    "png".to_string()
}

// ---------------------------------------------------------------------------
// Slim server struct
// ---------------------------------------------------------------------------

/// The slim Chrome DevTools MCP server (3 tools only).
#[derive(Clone)]
pub struct SlimServer {
    ctx: SharedContext,
    tool_router: ToolRouter<Self>,
}

impl SlimServer {
    pub fn new(ctx: SharedContext) -> Self {
        Self {
            ctx,
            tool_router: Self::tool_router(),
        }
    }
}

// ---------------------------------------------------------------------------
// Tool implementations (3 tools)
// ---------------------------------------------------------------------------

#[tool_router]
impl SlimServer {
    #[tool(
        name = "navigate_page",
        description = "Navigate the active page to a URL, or pass 'back', 'forward', 'reload'"
    )]
    async fn navigate_page(&self, params: Parameters<SlimNavigateParams>) -> String {
        let mut ctx = self.ctx.lock().await;
        let page_state = match ctx.selected_page_mut() {
            Some(p) => p,
            None => return "No page selected".to_string(),
        };
        let cdp = page_state.cdp_page_mut();
        let url = &params.0.url;
        let result = match url.as_str() {
            "back" => cdp.go_back().await,
            "forward" => cdp.go_forward().await,
            "reload" => cdp.reload().await,
            _ => cdp.navigate(url).await,
        };
        match result {
            Ok(()) => format!("Navigated to {url}"),
            Err(e) => format!("Navigation error: {e}"),
        }
    }

    #[tool(
        name = "evaluate_script",
        description = "Execute JavaScript in the page context and return the result"
    )]
    async fn evaluate_script(&self, params: Parameters<SlimEvaluateParams>) -> String {
        let ctx = self.ctx.lock().await;
        let page = match ctx.selected_page() {
            Some(p) => p.cdp_page().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        match page.evaluate(&params.0.expression).await {
            Ok(value) => {
                if value.is_null() {
                    "undefined".to_string()
                } else {
                    serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
                }
            }
            Err(e) => format!("Evaluation error: {e}"),
        }
    }

    #[tool(
        name = "take_screenshot",
        description = "Capture a screenshot of the current page"
    )]
    async fn take_screenshot(
        &self,
        params: Parameters<SlimScreenshotParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let format = if params.0.format.is_empty() {
            "png"
        } else {
            &params.0.format
        };
        let mime = match format {
            "jpeg" | "jpg" => "image/jpeg",
            "webp" => "image/webp",
            _ => "image/png",
        };

        let ctx = self.ctx.lock().await;
        let page = match ctx.selected_page() {
            Some(ps) => ps.cdp_page().clone(),
            None => {
                return Ok(CallToolResult::error(vec![Content::text(
                    "No page selected",
                )]));
            }
        };
        drop(ctx);

        match page.capture_screenshot(format, None, None, false).await {
            Ok(data) => Ok(CallToolResult::success(vec![Content::image(data, mime)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Screenshot error: {e}"
            ))])),
        }
    }
}

// ---------------------------------------------------------------------------
// Implement ServerHandler via tool_handler macro
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for SlimServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.instructions = Some(
            "Chrome DevTools MCP Server (slim mode) -- navigate, evaluate, and screenshot.".into(),
        );
        info
    }
}
