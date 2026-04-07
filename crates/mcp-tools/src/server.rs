//! ChromeDevToolsServer — the MCP server struct with all 29 tool implementations.

use std::sync::Arc;
use std::time::Duration;

use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, Content, ServerInfo};
use rmcp::{ServerHandler, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::Deserialize;
use tokio::sync::Mutex;

use crate::context::McpContext;
use crate::formatters::snapshot::format_snapshot;
use crate::utils::keyboard::parse_key_combination;
use crate::wait_for::wait_for_text;

/// Thread-safe shared context.
pub type SharedContext = Arc<Mutex<McpContext>>;

// ---------------------------------------------------------------------------
// Parameter types for each tool (derive JsonSchema + Deserialize)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListPagesParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SelectPageParams {
    /// The MCP page ID to select.
    pub page_id: u32,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct NewPageParams {
    /// Optional URL to open in the new tab.
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct NavigatePageParams {
    /// URL to navigate to, or "back", "forward", "reload".
    pub url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClosePageParams {
    /// The MCP page ID to close.
    pub page_id: u32,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WaitForParams {
    /// Text strings to wait for on the page.
    pub texts: Vec<String>,
    /// Timeout in milliseconds (default 30000).
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_timeout() -> u64 {
    30000
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClickParams {
    /// UID of the element from the accessibility snapshot.
    pub uid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ClickAtParams {
    /// X coordinate.
    pub x: f64,
    /// Y coordinate.
    pub y: f64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DragParams {
    /// UID of the source element.
    pub from_uid: String,
    /// UID of the target element.
    pub to_uid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FillParams {
    /// UID of the input/textarea/select element.
    pub uid: String,
    /// Value to fill.
    pub value: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FillFormParams {
    /// Map of UID -> value pairs.
    pub fields: Vec<FormField>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FormField {
    /// UID of the form field element.
    pub uid: String,
    /// Value to fill.
    pub value: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HoverParams {
    /// UID of the element to hover over.
    pub uid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PressKeyParams {
    /// Key combination, e.g. "Enter", "Control+a", "Shift+Tab".
    pub key: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TypeTextParams {
    /// Text to type into the focused element.
    pub text: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UploadFileParams {
    /// UID of the file input element.
    pub uid: String,
    /// Absolute path(s) to file(s) to upload.
    pub files: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct EvaluateScriptParams {
    /// JavaScript expression to evaluate.
    pub expression: String,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct TakeScreenshotParams {
    /// Image format: "png", "jpeg", or "webp".
    #[serde(default = "default_format")]
    pub format: String,
    /// Image quality (0-100) for JPEG/WebP.
    #[serde(default)]
    pub quality: Option<u32>,
    /// Capture full page.
    #[serde(default)]
    pub full_page: bool,
    /// UID of element to screenshot (element scope).
    #[serde(default)]
    pub uid: Option<String>,
}

fn default_format() -> String {
    "png".to_string()
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct TakeSnapshotParams {
    /// Include verbose details.
    #[serde(default)]
    pub verbose: bool,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListConsoleMessagesParams {
    /// Page number (1-indexed).
    #[serde(default = "default_page_num")]
    pub page: usize,
    /// Items per page.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

fn default_page_num() -> usize {
    1
}
fn default_page_size() -> usize {
    25
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetConsoleMessageParams {
    /// Console message index.
    pub index: usize,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct LighthouseAuditParams {
    /// URL to audit (defaults to current page URL).
    #[serde(default)]
    pub url: Option<String>,
    /// Categories to audit (e.g., "performance", "accessibility").
    #[serde(default)]
    pub categories: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListNetworkRequestsParams {
    /// Filter by URL pattern.
    #[serde(default)]
    pub url_filter: Option<String>,
    /// Filter by resource type.
    #[serde(default)]
    pub resource_type: Option<String>,
    /// Page number (1-indexed).
    #[serde(default = "default_page_num")]
    pub page: usize,
    /// Items per page.
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetNetworkRequestParams {
    /// Request index in the current list.
    pub index: usize,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct EmulateParams {
    /// Device name for built-in presets (e.g., "iPhone 14").
    #[serde(default)]
    pub device: Option<String>,
    /// Network conditions preset (e.g., "Slow 3G", "Fast 3G", "Offline").
    #[serde(default)]
    pub network: Option<String>,
    /// CPU throttling rate (1 = no throttle, 4 = 4x slower).
    #[serde(default)]
    pub cpu_throttling_rate: Option<f64>,
    /// Geolocation latitude.
    #[serde(default)]
    pub latitude: Option<f64>,
    /// Geolocation longitude.
    #[serde(default)]
    pub longitude: Option<f64>,
    /// Viewport width.
    #[serde(default)]
    pub viewport_width: Option<u32>,
    /// Viewport height.
    #[serde(default)]
    pub viewport_height: Option<u32>,
    /// Color scheme: "light" or "dark".
    #[serde(default)]
    pub color_scheme: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ResizePageParams {
    /// New width in pixels.
    pub width: u32,
    /// New height in pixels.
    pub height: u32,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct PerformanceStartTraceParams {
    /// Trace categories (comma-separated).
    #[serde(default)]
    pub categories: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct PerformanceStopTraceParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PerformanceAnalyzeInsightParams {
    /// Index of the insight to analyze from the last trace.
    pub index: usize,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct TakeMemorySnapshotParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InstallExtensionParams {
    /// Path to unpacked extension directory.
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UninstallExtensionParams {
    /// Extension ID to uninstall.
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema, Default)]
pub struct ListExtensionsParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReloadExtensionParams {
    /// Extension ID to reload.
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TriggerExtensionActionParams {
    /// Extension ID.
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HandleDialogParams {
    /// Whether to accept the dialog.
    pub accept: bool,
    /// Optional prompt text to enter.
    #[serde(default)]
    pub text: Option<String>,
}

// ---------------------------------------------------------------------------
// Server struct
// ---------------------------------------------------------------------------

/// The Chrome DevTools MCP server.
#[derive(Clone)]
pub struct ChromeDevToolsServer {
    ctx: SharedContext,
    tool_router: ToolRouter<Self>,
}

impl ChromeDevToolsServer {
    /// Create a new server with the given shared context.
    pub fn new(ctx: SharedContext) -> Self {
        Self {
            ctx,
            tool_router: Self::tool_router(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helper: resolve element backend_node_id from UID
// ---------------------------------------------------------------------------

/// Resolve a UID from the accessibility snapshot to a backend_node_id.
fn resolve_uid(ctx: &McpContext, uid: &str) -> Result<i64, String> {
    let page_state = ctx
        .selected_page()
        .ok_or_else(|| "No page selected".to_string())?;
    let snapshot = page_state
        .snapshot()
        .ok_or_else(|| "No snapshot taken yet. Call take_snapshot first.".to_string())?;
    let node = snapshot
        .id_to_node
        .get(uid)
        .ok_or_else(|| format!("Element with UID '{uid}' not found in snapshot"))?;
    node.backend_node_id
        .ok_or_else(|| format!("Element '{uid}' has no backendNodeId"))
}

// ---------------------------------------------------------------------------
// Tool implementations (29 tools)
// ---------------------------------------------------------------------------

#[tool_router]
impl ChromeDevToolsServer {
    // =======================================================================
    // PAGE TOOLS (6)
    // =======================================================================

    #[tool(
        name = "list_pages",
        description = "List all open pages/tabs with their URLs and titles"
    )]
    async fn list_pages(
        &self,
        #[allow(unused_variables)] params: Parameters<ListPagesParams>,
    ) -> String {
        let ctx = self.ctx.lock().await;
        let pages = ctx.list_pages();
        if pages.is_empty() {
            return "No pages open.".to_string();
        }
        let mut out = String::new();
        for p in &pages {
            let selected = if ctx.selected_page().map(|s| s.id()) == Some(p.id()) {
                " (selected)"
            } else {
                ""
            };
            out.push_str(&format!("Page {}: {}{}\n", p.id(), p.url(), selected,));
        }
        out
    }

    #[tool(
        name = "select_page",
        description = "Switch the active page by its MCP page ID"
    )]
    async fn select_page(&self, params: Parameters<SelectPageParams>) -> String {
        let mut ctx = self.ctx.lock().await;
        match ctx.select_page(params.0.page_id) {
            Ok(()) => format!("Selected page {}", params.0.page_id),
            Err(e) => format!("Error: {e}"),
        }
    }

    #[tool(
        name = "new_page",
        description = "Open a new browser tab, optionally navigating to a URL"
    )]
    async fn new_page(&self, params: Parameters<NewPageParams>) -> String {
        let url = params.0.url.as_deref().unwrap_or("about:blank");
        let ctx = self.ctx.lock().await;
        let browser = ctx.browser().clone();
        let session = browser.session();

        match cdp_client::target::TargetManager::create_target(session, url, false).await {
            Ok(target_id) => {
                match session.attach_to_target(&target_id).await {
                    Ok(cdp_session) => {
                        let page = cdp_client::CdpPage::new(
                            cdp_session,
                            target_id.clone(),
                            url.to_string(),
                        );
                        // Drop ctx to re-lock mutably
                        drop(ctx);
                        let mut ctx = self.ctx.lock().await;
                        let page_id = ctx.register_page(page);
                        format!("Opened new page {page_id} at {url}")
                    }
                    Err(e) => format!("Error attaching to new page: {e}"),
                }
            }
            Err(e) => format!("Error creating new page: {e}"),
        }
    }

    #[tool(
        name = "navigate_page",
        description = "Navigate the active page to a URL, or pass 'back', 'forward', 'reload'"
    )]
    async fn navigate_page(&self, params: Parameters<NavigatePageParams>) -> String {
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

    #[tool(name = "close_page", description = "Close a page by its MCP page ID")]
    async fn close_page(&self, params: Parameters<ClosePageParams>) -> String {
        let mut ctx = self.ctx.lock().await;
        let page_state = match ctx.get_page(params.0.page_id) {
            Some(p) => p,
            None => return format!("Page {} not found", params.0.page_id),
        };
        let target_id = page_state.cdp_page().target_id().to_string();
        let browser = ctx.browser().clone();
        match cdp_client::target::TargetManager::close_target(browser.session(), &target_id).await {
            Ok(()) => {
                ctx.remove_page(&target_id);
                format!("Closed page {}", params.0.page_id)
            }
            Err(e) => format!("Error closing page: {e}"),
        }
    }

    #[tool(
        name = "wait_for",
        description = "Wait for specific text to appear on the page"
    )]
    async fn wait_for(&self, params: Parameters<WaitForParams>) -> String {
        let ctx = self.ctx.lock().await;
        let page_state = match ctx.selected_page() {
            Some(p) => p,
            None => return "No page selected".to_string(),
        };
        let cdp = page_state.cdp_page();
        let timeout = Duration::from_millis(params.0.timeout_ms);
        match wait_for_text(cdp, &params.0.texts, timeout).await {
            Ok(()) => format!("Found text {:?} on the page", params.0.texts),
            Err(e) => format!("Wait failed: {e}"),
        }
    }

    // =======================================================================
    // INPUT TOOLS (9)
    // =======================================================================

    #[tool(
        name = "click",
        description = "Click an element by its UID from the accessibility snapshot"
    )]
    async fn click(&self, params: Parameters<ClickParams>) -> String {
        let ctx = self.ctx.lock().await;
        let backend_node_id = match resolve_uid(&ctx, &params.0.uid) {
            Ok(id) => id,
            Err(e) => return e,
        };
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        match cdp_client::input::get_element_center(&session, backend_node_id).await {
            Ok((x, y)) => match cdp_client::input::click(&session, x, y, "left").await {
                Ok(()) => format!("Clicked element '{}'", params.0.uid),
                Err(e) => format!("Click error: {e}"),
            },
            Err(e) => format!("Could not locate element: {e}"),
        }
    }

    #[tool(
        name = "click_at",
        description = "Click at specific x,y coordinates (experimental vision)"
    )]
    async fn click_at(&self, params: Parameters<ClickAtParams>) -> String {
        let ctx = self.ctx.lock().await;
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        match cdp_client::input::click(&session, params.0.x, params.0.y, "left").await {
            Ok(()) => format!("Clicked at ({}, {})", params.0.x, params.0.y),
            Err(e) => format!("Click error: {e}"),
        }
    }

    #[tool(name = "drag", description = "Drag from one element UID to another")]
    async fn drag(&self, params: Parameters<DragParams>) -> String {
        let ctx = self.ctx.lock().await;
        let from_id = match resolve_uid(&ctx, &params.0.from_uid) {
            Ok(id) => id,
            Err(e) => return format!("Source: {e}"),
        };
        let to_id = match resolve_uid(&ctx, &params.0.to_uid) {
            Ok(id) => id,
            Err(e) => return format!("Target: {e}"),
        };
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        let from = match cdp_client::input::get_element_center(&session, from_id).await {
            Ok(c) => c,
            Err(e) => return format!("Cannot locate source: {e}"),
        };
        let to = match cdp_client::input::get_element_center(&session, to_id).await {
            Ok(c) => c,
            Err(e) => return format!("Cannot locate target: {e}"),
        };

        match cdp_client::input::drag(&session, from.0, from.1, to.0, to.1).await {
            Ok(()) => format!(
                "Dragged from '{}' to '{}'",
                params.0.from_uid, params.0.to_uid
            ),
            Err(e) => format!("Drag error: {e}"),
        }
    }

    #[tool(
        name = "fill",
        description = "Fill an input, textarea, or select element identified by UID"
    )]
    async fn fill(&self, params: Parameters<FillParams>) -> String {
        let ctx = self.ctx.lock().await;
        let backend_node_id = match resolve_uid(&ctx, &params.0.uid) {
            Ok(id) => id,
            Err(e) => return e,
        };
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        // Focus the element, clear, then type the new value.
        if let Err(e) = cdp_client::input::focus_element(&session, backend_node_id).await {
            return format!("Focus error: {e}");
        }
        // Select all and delete existing content.
        if let Err(e) = cdp_client::input::press_key(&session, "a", 2).await {
            return format!("Select all error: {e}");
        }
        if let Err(e) = cdp_client::input::press_key(&session, "Backspace", 0).await {
            return format!("Delete error: {e}");
        }
        match cdp_client::input::type_text(&session, &params.0.value).await {
            Ok(()) => format!("Filled '{}' with '{}'", params.0.uid, params.0.value),
            Err(e) => format!("Type error: {e}"),
        }
    }

    #[tool(name = "fill_form", description = "Fill multiple form fields at once")]
    async fn fill_form(&self, params: Parameters<FillFormParams>) -> String {
        let mut results = Vec::new();
        for field in &params.0.fields {
            let ctx = self.ctx.lock().await;
            let backend_node_id = match resolve_uid(&ctx, &field.uid) {
                Ok(id) => id,
                Err(e) => {
                    results.push(format!("'{}': {e}", field.uid));
                    continue;
                }
            };
            let session = match ctx.selected_page() {
                Some(p) => p.cdp_page().session().clone(),
                None => {
                    results.push("No page selected".to_string());
                    break;
                }
            };
            drop(ctx);

            if let Err(e) = cdp_client::input::focus_element(&session, backend_node_id).await {
                results.push(format!("'{}' focus error: {e}", field.uid));
                continue;
            }
            let _ = cdp_client::input::press_key(&session, "a", 2).await;
            let _ = cdp_client::input::press_key(&session, "Backspace", 0).await;
            match cdp_client::input::type_text(&session, &field.value).await {
                Ok(()) => results.push(format!("'{}': filled", field.uid)),
                Err(e) => results.push(format!("'{}': type error: {e}", field.uid)),
            }
        }
        results.join("\n")
    }

    #[tool(
        name = "hover",
        description = "Hover over an element identified by UID"
    )]
    async fn hover(&self, params: Parameters<HoverParams>) -> String {
        let ctx = self.ctx.lock().await;
        let backend_node_id = match resolve_uid(&ctx, &params.0.uid) {
            Ok(id) => id,
            Err(e) => return e,
        };
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        match cdp_client::input::get_element_center(&session, backend_node_id).await {
            Ok((x, y)) => match cdp_client::input::hover(&session, x, y).await {
                Ok(()) => format!("Hovered over '{}'", params.0.uid),
                Err(e) => format!("Hover error: {e}"),
            },
            Err(e) => format!("Could not locate element: {e}"),
        }
    }

    #[tool(
        name = "press_key",
        description = "Press a key combination (e.g. 'Enter', 'Control+a', 'Shift+Tab')"
    )]
    async fn press_key(&self, params: Parameters<PressKeyParams>) -> String {
        let ctx = self.ctx.lock().await;
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        let (key, modifiers) = parse_key_combination(&params.0.key);
        match cdp_client::input::press_key(&session, &key, modifiers).await {
            Ok(()) => format!("Pressed '{}'", params.0.key),
            Err(e) => format!("Key press error: {e}"),
        }
    }

    #[tool(
        name = "type_text",
        description = "Type text into the currently focused element"
    )]
    async fn type_text(&self, params: Parameters<TypeTextParams>) -> String {
        let ctx = self.ctx.lock().await;
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        match cdp_client::input::type_text(&session, &params.0.text).await {
            Ok(()) => format!("Typed '{}' characters", params.0.text.len()),
            Err(e) => format!("Type error: {e}"),
        }
    }

    #[tool(
        name = "upload_file",
        description = "Upload file(s) through a file input element identified by UID"
    )]
    async fn upload_file(&self, params: Parameters<UploadFileParams>) -> String {
        let ctx = self.ctx.lock().await;
        let backend_node_id = match resolve_uid(&ctx, &params.0.uid) {
            Ok(id) => id,
            Err(e) => return e,
        };
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        match cdp_client::dom::set_file_input_files(&session, backend_node_id, &params.0.files)
            .await
        {
            Ok(()) => format!("Uploaded {} file(s)", params.0.files.len()),
            Err(e) => format!("Upload error: {e}"),
        }
    }

    // =======================================================================
    // DEBUGGING TOOLS (6)
    // =======================================================================

    #[tool(
        name = "evaluate_script",
        description = "Execute JavaScript in the page context and return the result"
    )]
    async fn evaluate_script(&self, params: Parameters<EvaluateScriptParams>) -> String {
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
        description = "Capture a screenshot of the page (PNG/JPEG/WebP, viewport/full-page/element)"
    )]
    async fn take_screenshot(
        &self,
        params: Parameters<TakeScreenshotParams>,
    ) -> Result<CallToolResult, rmcp::ErrorData> {
        let p = params.0;
        let format = if p.format.is_empty() {
            "png"
        } else {
            &p.format
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

        // Build clip region if element UID is provided.
        let clip = if let Some(ref uid) = p.uid {
            let ctx = self.ctx.lock().await;
            match resolve_uid(&ctx, uid) {
                Ok(backend_node_id) => {
                    let session = match ctx.selected_page() {
                        Some(ps) => ps.cdp_page().session().clone(),
                        None => {
                            return Ok(CallToolResult::error(vec![Content::text(
                                "No page selected",
                            )]));
                        }
                    };
                    drop(ctx);
                    match cdp_client::dom::get_box_model(&session, backend_node_id).await {
                        Ok(model) => {
                            let content = model
                                .get("content")
                                .and_then(|c| c.as_array())
                                .cloned()
                                .unwrap_or_default();
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
                                let min_x = xs.iter().cloned().fold(f64::INFINITY, f64::min);
                                let min_y = ys.iter().cloned().fold(f64::INFINITY, f64::min);
                                let max_x = xs.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                                let max_y = ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                                Some(serde_json::json!({
                                    "x": min_x,
                                    "y": min_y,
                                    "width": max_x - min_x,
                                    "height": max_y - min_y,
                                    "scale": 1
                                }))
                            } else {
                                None
                            }
                        }
                        Err(_) => None,
                    }
                }
                Err(_) => None,
            }
        } else {
            None
        };

        match page
            .capture_screenshot(format, p.quality, clip, p.full_page)
            .await
        {
            Ok(data) => Ok(CallToolResult::success(vec![Content::image(data, mime)])),
            Err(e) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Screenshot error: {e}"
            ))])),
        }
    }

    #[tool(
        name = "take_snapshot",
        description = "Get the accessibility tree snapshot of the page with element UIDs"
    )]
    async fn take_snapshot(&self, params: Parameters<TakeSnapshotParams>) -> String {
        let ctx = self.ctx.lock().await;
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        // Fetch the accessibility tree.
        let ax_nodes = match cdp_client::accessibility::get_full_ax_tree(&session).await {
            Ok(nodes) => nodes,
            Err(e) => return format!("Snapshot error: {e}"),
        };

        let tree = match cdp_client::accessibility::build_accessibility_tree(&ax_nodes) {
            Some(tree) => tree,
            None => return "Empty accessibility tree".to_string(),
        };

        // Build snapshot text with UIDs.
        let mut ctx = self.ctx.lock().await;
        let snapshot_id = ctx.next_snapshot_id();
        let verbose = params.0.verbose;

        // Build uid-annotated snapshot.
        let text = format_snapshot(&tree, verbose);

        // Store the snapshot in page state.
        if let Some(page_state) = ctx.selected_page_mut() {
            use std::collections::HashMap;
            let mut id_to_node = HashMap::new();
            assign_uids(&tree, &mut id_to_node, &mut 0);
            let text_snapshot = crate::page_state::TextSnapshot {
                snapshot_id: snapshot_id.to_string(),
                root: to_text_node(&tree, &id_to_node),
                id_to_node,
                verbose,
            };
            page_state.set_snapshot(text_snapshot);
        }

        text
    }

    #[tool(
        name = "list_console_messages",
        description = "List console messages from the active page with pagination"
    )]
    async fn list_console_messages(&self, params: Parameters<ListConsoleMessagesParams>) -> String {
        let ctx = self.ctx.lock().await;
        let target_id = match ctx.selected_page() {
            Some(p) => p.cdp_page().target_id().to_string(),
            None => return "No page selected".to_string(),
        };
        let collector = ctx.console_collector().clone();
        drop(ctx);

        let messages = collector.get_messages(&target_id, true).await;
        if messages.is_empty() {
            return "No console messages.".to_string();
        }
        crate::formatters::console::format_console_messages(
            &messages,
            params.0.page,
            params.0.page_size,
        )
    }

    #[tool(
        name = "get_console_message",
        description = "Get a specific console message by index, including its stack trace"
    )]
    async fn get_console_message(&self, params: Parameters<GetConsoleMessageParams>) -> String {
        let ctx = self.ctx.lock().await;
        let target_id = match ctx.selected_page() {
            Some(p) => p.cdp_page().target_id().to_string(),
            None => return "No page selected".to_string(),
        };
        let collector = ctx.console_collector().clone();
        drop(ctx);

        let messages = collector.get_messages(&target_id, true).await;
        match messages.get(params.0.index) {
            Some(msg) => crate::formatters::console::format_console_message(msg, params.0.index),
            None => format!("Message at index {} not found", params.0.index),
        }
    }

    #[tool(
        name = "lighthouse_audit",
        description = "Run a Lighthouse audit (shells out to the lighthouse CLI)"
    )]
    async fn lighthouse_audit(&self, params: Parameters<LighthouseAuditParams>) -> String {
        let ctx = self.ctx.lock().await;
        let url = params.0.url.clone().unwrap_or_else(|| {
            ctx.selected_page()
                .map(|p| p.url().to_string())
                .unwrap_or_else(|| "http://localhost".to_string())
        });
        drop(ctx);

        let lighthouse_bin = which::which("lighthouse").unwrap_or_else(|_| "lighthouse".into());
        let mut cmd = tokio::process::Command::new(lighthouse_bin);
        cmd.arg(&url)
            .arg("--output=json")
            .arg("--quiet")
            .arg("--chrome-flags=--headless");
        if !params.0.categories.is_empty() {
            cmd.arg(format!(
                "--only-categories={}",
                params.0.categories.join(",")
            ));
        }
        match cmd.output().await {
            Ok(output) => {
                if output.status.success() {
                    let json_str = String::from_utf8_lossy(&output.stdout);
                    // Parse and extract summary.
                    match serde_json::from_str::<serde_json::Value>(&json_str) {
                        Ok(report) => {
                            let categories = report.get("categories").and_then(|c| c.as_object());
                            let mut summary = String::from("Lighthouse Results:\n");
                            if let Some(cats) = categories {
                                for (name, cat) in cats {
                                    let score =
                                        cat.get("score").and_then(|s| s.as_f64()).unwrap_or(0.0);
                                    summary.push_str(&format!("  {name}: {:.0}%\n", score * 100.0));
                                }
                            }
                            summary
                        }
                        Err(_) => json_str.into_owned(),
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    format!("Lighthouse failed: {stderr}")
                }
            }
            Err(e) => format!("Failed to run lighthouse: {e}"),
        }
    }

    // =======================================================================
    // NETWORK TOOLS (2)
    // =======================================================================

    #[tool(
        name = "list_network_requests",
        description = "List network requests from the active page with optional filtering"
    )]
    async fn list_network_requests(&self, params: Parameters<ListNetworkRequestsParams>) -> String {
        let ctx = self.ctx.lock().await;
        let target_id = match ctx.selected_page() {
            Some(p) => p.cdp_page().target_id().to_string(),
            None => return "No page selected".to_string(),
        };
        let collector = ctx.network_collector().clone();
        drop(ctx);

        let requests = collector.get_requests(&target_id).await;
        crate::formatters::network::format_network_requests(
            &requests,
            params.0.url_filter.as_deref(),
            params.0.resource_type.as_deref(),
            params.0.page,
            params.0.page_size,
        )
    }

    #[tool(
        name = "get_network_request",
        description = "Get detailed information about a specific network request by index"
    )]
    async fn get_network_request(&self, params: Parameters<GetNetworkRequestParams>) -> String {
        let ctx = self.ctx.lock().await;
        let target_id = match ctx.selected_page() {
            Some(p) => p.cdp_page().target_id().to_string(),
            None => return "No page selected".to_string(),
        };
        let collector = ctx.network_collector().clone();
        drop(ctx);

        let requests = collector.get_requests(&target_id).await;
        match requests.get(params.0.index) {
            Some(req) => crate::formatters::network::format_network_request_detail(req),
            None => format!("Request at index {} not found", params.0.index),
        }
    }

    // =======================================================================
    // EMULATION TOOLS (2)
    // =======================================================================

    #[tool(
        name = "emulate",
        description = "Emulate device, network, CPU, geolocation, viewport, or color scheme"
    )]
    async fn emulate(&self, params: Parameters<EmulateParams>) -> String {
        let p = params.0;
        let ctx = self.ctx.lock().await;
        let session = match ctx.selected_page() {
            Some(ps) => ps.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        let mut applied = Vec::new();

        // Network conditions.
        if let Some(ref network) = p.network {
            let (offline, latency, download, upload) = match network.as_str() {
                "Offline" => (true, 0.0, 0.0, 0.0),
                "Slow 3G" => (false, 2000.0, 50000.0, 50000.0),
                "Fast 3G" => (false, 563.0, 180000.0, 84375.0),
                _ => (false, 0.0, -1.0, -1.0),
            };
            match cdp_client::network::emulate_network_conditions(
                &session, offline, latency, download, upload,
            )
            .await
            {
                Ok(()) => applied.push(format!("Network: {network}")),
                Err(e) => applied.push(format!("Network error: {e}")),
            }
        }

        // CPU throttling.
        if let Some(rate) = p.cpu_throttling_rate {
            match cdp_client::emulation::set_cpu_throttling_rate(&session, rate).await {
                Ok(()) => applied.push(format!("CPU throttling: {rate}x")),
                Err(e) => applied.push(format!("CPU error: {e}")),
            }
        }

        // Geolocation.
        if let (Some(lat), Some(lng)) = (p.latitude, p.longitude) {
            match cdp_client::emulation::set_geolocation_override(&session, lat, lng, None).await {
                Ok(()) => applied.push(format!("Geolocation: ({lat}, {lng})")),
                Err(e) => applied.push(format!("Geolocation error: {e}")),
            }
        }

        // Viewport.
        if let (Some(w), Some(h)) = (p.viewport_width, p.viewport_height) {
            match cdp_client::emulation::set_device_metrics_override(&session, w, h, 1.0, false)
                .await
            {
                Ok(()) => applied.push(format!("Viewport: {w}x{h}")),
                Err(e) => applied.push(format!("Viewport error: {e}")),
            }
        }

        // Color scheme.
        if let Some(ref scheme) = p.color_scheme {
            let features = vec![("prefers-color-scheme".to_string(), scheme.clone())];
            match cdp_client::emulation::set_emulated_media(&session, &features).await {
                Ok(()) => applied.push(format!("Color scheme: {scheme}")),
                Err(e) => applied.push(format!("Color scheme error: {e}")),
            }
        }

        if applied.is_empty() {
            "No emulation parameters provided.".to_string()
        } else {
            format!("Applied emulation:\n{}", applied.join("\n"))
        }
    }

    #[tool(
        name = "resize_page",
        description = "Change the active page dimensions"
    )]
    async fn resize_page(&self, params: Parameters<ResizePageParams>) -> String {
        let ctx = self.ctx.lock().await;
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        match cdp_client::emulation::set_device_metrics_override(
            &session,
            params.0.width,
            params.0.height,
            1.0,
            false,
        )
        .await
        {
            Ok(()) => format!("Resized to {}x{}", params.0.width, params.0.height),
            Err(e) => format!("Resize error: {e}"),
        }
    }

    // =======================================================================
    // PERFORMANCE TOOLS (4)
    // =======================================================================

    #[tool(
        name = "performance_start_trace",
        description = "Start recording a performance trace"
    )]
    async fn performance_start_trace(
        &self,
        params: Parameters<PerformanceStartTraceParams>,
    ) -> String {
        let mut ctx = self.ctx.lock().await;
        if ctx.is_running_trace() {
            return "A trace is already running. Stop it first.".to_string();
        }
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };

        match cdp_client::tracing::start(&session, params.0.categories.as_deref(), Some(500.0))
            .await
        {
            Ok(()) => {
                ctx.set_running_trace(true);
                "Performance trace started.".to_string()
            }
            Err(e) => format!("Trace start error: {e}"),
        }
    }

    #[tool(
        name = "performance_stop_trace",
        description = "Stop recording the performance trace and return a summary"
    )]
    async fn performance_stop_trace(
        &self,
        #[allow(unused_variables)] params: Parameters<PerformanceStopTraceParams>,
    ) -> String {
        let mut ctx = self.ctx.lock().await;
        if !ctx.is_running_trace() {
            return "No trace is running.".to_string();
        }
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };

        // Subscribe to trace data events BEFORE sending Tracing.end.
        let mut data_rx = session.subscribe_events("Tracing.dataCollected").await;
        let mut complete_rx = session.subscribe_events("Tracing.tracingComplete").await;

        // Send Tracing.end to stop the trace.
        if let Err(e) = cdp_client::tracing::stop(&session).await {
            ctx.set_running_trace(false);
            return format!("Trace stop error: {e}");
        }
        ctx.set_running_trace(false);
        drop(ctx);

        // Collect all trace event chunks until Tracing.tracingComplete fires.
        let mut all_trace_events: Vec<serde_json::Value> = Vec::new();
        let timeout = tokio::time::Duration::from_secs(30);
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            tokio::select! {
                Some(event) = data_rx.recv() => {
                    // Tracing.dataCollected delivers chunks of trace events in params.value.
                    let params = event.get("params").cloned().unwrap_or(event);
                    if let Some(value) = params.get("value").and_then(|v| v.as_array()) {
                        all_trace_events.extend(value.iter().cloned());
                    }
                }
                Some(_) = complete_rx.recv() => {
                    // Tracing.tracingComplete signals all data has been delivered.
                    break;
                }
                _ = tokio::time::sleep_until(deadline) => {
                    tracing::warn!("Timed out waiting for Tracing.tracingComplete");
                    break;
                }
            }
        }

        if all_trace_events.is_empty() {
            return "Performance trace stopped but no trace events were collected.".to_string();
        }

        // Parse the raw trace events into TraceEvent structs.
        let trace_json = serde_json::Value::Array(all_trace_events);
        let trace_bytes = serde_json::to_vec(&trace_json).unwrap_or_default();
        let events = match trace_engine::parse_trace(&trace_bytes) {
            Ok(events) => events,
            Err(e) => {
                return format!("Trace stopped but failed to parse events: {e}");
            }
        };

        // Extract metrics and generate insights.
        let metrics = trace_engine::extract_metrics(&events);
        let insights = trace_engine::generate_insights(&metrics);
        let summary = trace_engine::format_trace_summary(&metrics, &insights);

        // Store the trace result in context for later analysis.
        let trace_result = serde_json::json!({
            "metrics": metrics,
            "insights": insights,
            "event_count": events.len(),
        });
        let mut ctx = self.ctx.lock().await;
        ctx.store_trace_result(trace_result);

        summary
    }

    #[tool(
        name = "performance_analyze_insight",
        description = "Analyze a specific insight from the last performance trace"
    )]
    async fn performance_analyze_insight(
        &self,
        params: Parameters<PerformanceAnalyzeInsightParams>,
    ) -> String {
        let ctx = self.ctx.lock().await;
        let results = ctx.trace_results();
        if results.is_empty() {
            return "No trace results available. Run a performance trace first.".to_string();
        }
        // Parse insights from the stored trace data.
        let trace_data = &results[0];
        let insights = trace_data
            .get("insights")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        match insights.get(params.0.index) {
            Some(insight) => {
                serde_json::to_string_pretty(insight).unwrap_or_else(|_| insight.to_string())
            }
            None => format!(
                "Insight at index {} not found ({} available)",
                params.0.index,
                insights.len()
            ),
        }
    }

    #[tool(
        name = "take_memory_snapshot",
        description = "Capture a heap snapshot of the active page"
    )]
    async fn take_memory_snapshot(
        &self,
        #[allow(unused_variables)] params: Parameters<TakeMemorySnapshotParams>,
    ) -> String {
        let ctx = self.ctx.lock().await;
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        // Take a heap snapshot via CDP.
        match session
            .send_command("HeapProfiler.takeHeapSnapshot", serde_json::json!({}))
            .await
        {
            Ok(_) => "Heap snapshot captured.".to_string(),
            Err(e) => format!("Heap snapshot error: {e}"),
        }
    }

    // =======================================================================
    // EXTENSION TOOLS (5)
    // =======================================================================

    #[tool(
        name = "install_extension",
        description = "Install an unpacked Chrome extension from a directory path"
    )]
    async fn install_extension(&self, params: Parameters<InstallExtensionParams>) -> String {
        let ctx = self.ctx.lock().await;
        let browser = ctx.browser().clone();
        drop(ctx);

        match cdp_client::extensions::load_unpacked(browser.session(), &params.0.path).await {
            Ok(ext_id) => {
                let mut ctx = self.ctx.lock().await;
                let path = std::path::PathBuf::from(&params.0.path);
                let _ = ctx
                    .extension_registry_mut()
                    .register(ext_id.clone(), path)
                    .await;
                format!("Extension installed with ID: {ext_id}")
            }
            Err(e) => format!("Install error: {e}"),
        }
    }

    #[tool(
        name = "uninstall_extension",
        description = "Remove an extension by its ID"
    )]
    async fn uninstall_extension(&self, params: Parameters<UninstallExtensionParams>) -> String {
        let ctx = self.ctx.lock().await;
        let browser = ctx.browser().clone();
        drop(ctx);

        match cdp_client::extensions::uninstall(browser.session(), &params.0.id).await {
            Ok(()) => {
                let mut ctx = self.ctx.lock().await;
                ctx.extension_registry_mut().remove(&params.0.id);
                format!("Extension '{}' uninstalled", params.0.id)
            }
            Err(e) => format!("Uninstall error: {e}"),
        }
    }

    #[tool(
        name = "list_extensions",
        description = "List all installed Chrome extensions"
    )]
    async fn list_extensions(
        &self,
        #[allow(unused_variables)] params: Parameters<ListExtensionsParams>,
    ) -> String {
        let ctx = self.ctx.lock().await;
        let exts = ctx.extension_registry().list();
        if exts.is_empty() {
            return "No extensions installed.".to_string();
        }
        let mut out = String::new();
        for ext in exts {
            out.push_str(&format!("- {} (v{}) [{}]\n", ext.name, ext.version, ext.id));
        }
        out
    }

    #[tool(
        name = "reload_extension",
        description = "Reload a Chrome extension by its ID"
    )]
    async fn reload_extension(&self, params: Parameters<ReloadExtensionParams>) -> String {
        let ctx = self.ctx.lock().await;
        let ext = ctx.extension_registry().get(&params.0.id);
        let path = match ext {
            Some(e) => e.path.display().to_string(),
            None => return format!("Extension '{}' not found", params.0.id),
        };
        let browser = ctx.browser().clone();
        drop(ctx);

        // Uninstall and reinstall to reload.
        let _ = cdp_client::extensions::uninstall(browser.session(), &params.0.id).await;
        match cdp_client::extensions::load_unpacked(browser.session(), &path).await {
            Ok(new_id) => {
                let mut ctx = self.ctx.lock().await;
                ctx.extension_registry_mut().remove(&params.0.id);
                let _ = ctx
                    .extension_registry_mut()
                    .register(new_id.clone(), std::path::PathBuf::from(&path))
                    .await;
                format!("Extension reloaded (new ID: {new_id})")
            }
            Err(e) => format!("Reload error: {e}"),
        }
    }

    #[tool(
        name = "trigger_extension_action",
        description = "Trigger a Chrome extension's action on the active page"
    )]
    async fn trigger_extension_action(
        &self,
        params: Parameters<TriggerExtensionActionParams>,
    ) -> String {
        let ctx = self.ctx.lock().await;
        let target_id = match ctx.selected_page() {
            Some(p) => p.cdp_page().target_id().to_string(),
            None => return "No page selected".to_string(),
        };
        let browser = ctx.browser().clone();
        drop(ctx);

        match cdp_client::extensions::trigger_action(browser.session(), &params.0.id, &target_id)
            .await
        {
            Ok(()) => format!("Extension '{}' action triggered", params.0.id),
            Err(e) => format!("Trigger error: {e}"),
        }
    }

    // =======================================================================
    // OTHER TOOLS (1)
    // =======================================================================

    #[tool(
        name = "handle_dialog",
        description = "Accept or dismiss a browser dialog (alert, confirm, prompt)"
    )]
    async fn handle_dialog(&self, params: Parameters<HandleDialogParams>) -> String {
        let ctx = self.ctx.lock().await;
        let session = match ctx.selected_page() {
            Some(p) => p.cdp_page().session().clone(),
            None => return "No page selected".to_string(),
        };
        drop(ctx);

        let mut cmd_params = serde_json::json!({
            "accept": params.0.accept,
        });
        if let Some(ref text) = params.0.text {
            cmd_params["promptText"] = serde_json::Value::String(text.clone());
        }

        match session
            .send_command("Page.handleJavaScriptDialog", cmd_params)
            .await
        {
            Ok(_) => {
                if params.0.accept {
                    "Dialog accepted.".to_string()
                } else {
                    "Dialog dismissed.".to_string()
                }
            }
            Err(e) => format!("Dialog handling error: {e}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Implement ServerHandler via tool_handler macro
// ---------------------------------------------------------------------------

#[tool_handler]
impl ServerHandler for ChromeDevToolsServer {
    fn get_info(&self) -> ServerInfo {
        let mut info = ServerInfo::default();
        info.instructions = Some(
            "Chrome DevTools MCP Server — control and inspect Chrome from AI assistants.".into(),
        );
        info
    }
}

// ---------------------------------------------------------------------------
// Helper: assign UID integers to accessibility tree nodes
// ---------------------------------------------------------------------------

fn assign_uids(
    node: &cdp_client::accessibility::AccessibilityNode,
    map: &mut std::collections::HashMap<String, crate::page_state::TextSnapshotNode>,
    counter: &mut u32,
) {
    let uid = format!("e{counter}");
    *counter += 1;

    let text_node = crate::page_state::TextSnapshotNode {
        id: uid.clone(),
        role: node.role.clone(),
        name: node.name.clone(),
        value: node.value.clone(),
        description: node.description.clone(),
        backend_node_id: node.backend_node_id,
        children: Vec::new(), // Children are tracked via the tree structure, not here.
    };
    map.insert(uid, text_node);

    for child in &node.children {
        assign_uids(child, map, counter);
    }
}

fn to_text_node(
    node: &cdp_client::accessibility::AccessibilityNode,
    _map: &std::collections::HashMap<String, crate::page_state::TextSnapshotNode>,
) -> crate::page_state::TextSnapshotNode {
    crate::page_state::TextSnapshotNode {
        id: String::new(),
        role: node.role.clone(),
        name: node.name.clone(),
        value: node.value.clone(),
        description: node.description.clone(),
        backend_node_id: node.backend_node_id,
        children: node
            .children
            .iter()
            .map(|c| to_text_node(c, _map))
            .collect(),
    }
}
