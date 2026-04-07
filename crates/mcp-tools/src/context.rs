//! McpContext -- central state management for the MCP server.
//!
//! Equivalent to McpContext.ts in the TypeScript implementation.
//! Holds the browser instance, page registry, collectors, and trace results.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use cdp_client::{Browser, CdpPage};

use crate::collector::{ConsoleCollector, NetworkCollector};
use crate::page_state::McpPageState;
use crate::utils::extension_registry::ExtensionRegistry;

/// Central context holding all browser state for the MCP server.
pub struct McpContext {
    /// The browser instance (launched or connected).
    browser: Arc<Browser>,

    /// All known pages, keyed by target ID.
    pages: HashMap<String, McpPageState>,

    /// The currently selected page's target ID.
    selected_page: Option<String>,

    /// Auto-incrementing page ID counter.
    next_page_id: u32,

    /// Network request collector (cloneable, internally synchronized).
    network_collector: NetworkCollector,

    /// Console message collector (cloneable, internally synchronized).
    console_collector: ConsoleCollector,

    /// Extension registry.
    extension_registry: ExtensionRegistry,

    /// Whether a performance trace is currently running.
    is_running_trace: bool,

    /// Stored trace results from completed recordings.
    trace_results: Vec<serde_json::Value>,

    /// Auto-incrementing snapshot ID counter.
    next_snapshot_id: u32,
}

impl McpContext {
    /// Create a new context from a browser instance.
    /// Discovers existing page targets and registers them.
    pub async fn new(browser: Arc<Browser>) -> anyhow::Result<Self> {
        let mut ctx = Self {
            browser,
            pages: HashMap::new(),
            selected_page: None,
            next_page_id: 1,
            network_collector: NetworkCollector::new(),
            console_collector: ConsoleCollector::new(),
            extension_registry: ExtensionRegistry::new(),
            is_running_trace: false,
            trace_results: Vec::new(),
            next_snapshot_id: 1,
        };

        // Discover existing page targets.
        ctx.discover_pages().await;

        Ok(ctx)
    }

    /// Discover existing page targets from the browser and register them.
    pub async fn discover_pages(&mut self) {
        let targets =
            match cdp_client::target::TargetManager::get_targets(self.browser.session()).await {
                Ok(t) => t,
                Err(e) => {
                    tracing::warn!(error = %e, "Failed to discover page targets");
                    return;
                }
            };

        for target in targets {
            if target.target_type != "page" {
                continue;
            }
            // Skip targets we already know about.
            if self.pages.contains_key(&target.target_id) {
                continue;
            }
            // Attach to the target to get a CDP session.
            match self
                .browser
                .session()
                .attach_to_target(&target.target_id)
                .await
            {
                Ok(session) => {
                    // Enable Page + Runtime + Network domains on the new session.
                    let _ = session
                        .send_command("Page.enable", serde_json::json!({}))
                        .await;
                    let _ = session
                        .send_command("Runtime.enable", serde_json::json!({}))
                        .await;
                    let _ = session
                        .send_command("Network.enable", serde_json::json!({}))
                        .await;

                    let page = CdpPage::new(
                        session.clone(),
                        target.target_id.clone(),
                        target.url.clone(),
                    );

                    // Register collectors for this page.
                    self.network_collector.add_page(&target.target_id).await;
                    self.console_collector.add_page(&target.target_id).await;

                    // Spawn background event listener tasks.
                    self.spawn_event_listeners(&session, &target.target_id)
                        .await;

                    self.register_page(page);
                    tracing::info!(
                        target_id = %target.target_id,
                        url = %target.url,
                        "Discovered page"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        target_id = %target.target_id,
                        error = %e,
                        "Failed to attach to page target"
                    );
                }
            }
        }
    }

    /// Spawn background tasks that listen for CDP events and feed them to collectors.
    async fn spawn_event_listeners(&self, session: &cdp_client::CdpSession, target_id: &str) {
        let target_id_owned = target_id.to_string();

        // Subscribe to Network.requestWillBeSent events.
        let mut network_rx = session.subscribe_events("Network.requestWillBeSent").await;
        let network_collector = self.network_collector.clone();
        let tid = target_id_owned.clone();
        tokio::spawn(async move {
            while let Some(event) = network_rx.recv().await {
                // Extract the request data from the event params.
                let params = event.get("params").cloned().unwrap_or(event.clone());
                let mut request_data = serde_json::json!({});

                // Extract key fields from Network.requestWillBeSent.
                if let Some(request) = params.get("request") {
                    if let Some(url) = request.get("url") {
                        request_data["url"] = url.clone();
                    }
                    if let Some(method) = request.get("method") {
                        request_data["method"] = method.clone();
                    }
                    if let Some(headers) = request.get("headers") {
                        request_data["requestHeaders"] = headers.clone();
                    }
                }
                if let Some(request_id) = params.get("requestId") {
                    request_data["requestId"] = request_id.clone();
                }
                if let Some(resource_type) = params.get("type") {
                    request_data["resourceType"] = resource_type.clone();
                }
                if let Some(timestamp) = params.get("timestamp") {
                    request_data["startTime"] = timestamp.clone();
                }

                network_collector.add_request(&tid, request_data).await;
            }
        });

        // Subscribe to Network.responseReceived to update requests with response data.
        let mut response_rx = session.subscribe_events("Network.responseReceived").await;
        let network_collector2 = self.network_collector.clone();
        let tid2 = target_id_owned.clone();
        tokio::spawn(async move {
            // We don't have a direct way to update existing requests in the collector,
            // so we add the response data as a separate entry that the formatter can merge.
            // A simpler approach: just add the response as another event; the tool handler
            // can correlate by requestId.
            while let Some(event) = response_rx.recv().await {
                let params = event.get("params").cloned().unwrap_or(event.clone());
                let mut response_data = serde_json::json!({});

                if let Some(request_id) = params.get("requestId") {
                    response_data["requestId"] = request_id.clone();
                }
                if let Some(response) = params.get("response") {
                    if let Some(status) = response.get("status") {
                        response_data["status"] = status.clone();
                    }
                    if let Some(status_text) = response.get("statusText") {
                        response_data["statusText"] = status_text.clone();
                    }
                    if let Some(mime) = response.get("mimeType") {
                        response_data["mimeType"] = mime.clone();
                    }
                    if let Some(headers) = response.get("headers") {
                        response_data["responseHeaders"] = headers.clone();
                    }
                    if let Some(url) = response.get("url") {
                        response_data["url"] = url.clone();
                    }
                    if let Some(timing) = response.get("timing") {
                        response_data["timing"] = timing.clone();
                    }
                    if let Some(encoded_data_length) = response.get("encodedDataLength") {
                        response_data["encodedDataLength"] = encoded_data_length.clone();
                    }
                }
                if let Some(resource_type) = params.get("type") {
                    response_data["resourceType"] = resource_type.clone();
                }
                // Mark this as a response event so we can distinguish it.
                response_data["_isResponse"] = serde_json::json!(true);

                network_collector2.add_request(&tid2, response_data).await;
            }
        });

        // Subscribe to Runtime.consoleAPICalled events.
        let mut console_rx = session.subscribe_events("Runtime.consoleAPICalled").await;
        let console_collector = self.console_collector.clone();
        let tid3 = target_id_owned.clone();
        tokio::spawn(async move {
            while let Some(event) = console_rx.recv().await {
                let params = event.get("params").cloned().unwrap_or(event.clone());
                console_collector.add_message(&tid3, params).await;
            }
        });

        // Subscribe to Page.frameNavigated events.
        let mut nav_rx = session.subscribe_events("Page.frameNavigated").await;
        let network_collector3 = self.network_collector.clone();
        let console_collector2 = self.console_collector.clone();
        let tid4 = target_id_owned;
        tokio::spawn(async move {
            while let Some(event) = nav_rx.recv().await {
                // Only react to top-level frame navigations.
                let params = event.get("params").cloned().unwrap_or(event.clone());
                let is_top_frame = params
                    .get("frame")
                    .and_then(|f| f.get("parentId"))
                    .is_none();
                if is_top_frame {
                    network_collector3.on_navigation(&tid4).await;
                    console_collector2.on_navigation(&tid4).await;
                }
            }
        });
    }

    /// Get a reference to the browser.
    pub fn browser(&self) -> &Arc<Browser> {
        &self.browser
    }

    /// Register a new page in the context.
    pub fn register_page(&mut self, page: CdpPage) -> u32 {
        let id = self.next_page_id;
        self.next_page_id += 1;

        let target_id = page.target_id().to_string();
        let state = McpPageState::new(id, page);

        self.pages.insert(target_id.clone(), state);

        // Auto-select if no page is selected.
        if self.selected_page.is_none() {
            self.selected_page = Some(target_id);
        }

        id
    }

    /// Get the currently selected page state.
    pub fn selected_page(&self) -> Option<&McpPageState> {
        self.selected_page
            .as_ref()
            .and_then(|id| self.pages.get(id))
    }

    /// Get the currently selected page state mutably.
    pub fn selected_page_mut(&mut self) -> Option<&mut McpPageState> {
        let id = self.selected_page.clone()?;
        self.pages.get_mut(&id)
    }

    /// Select a page by its MCP page ID.
    pub fn select_page(&mut self, page_id: u32) -> anyhow::Result<()> {
        let target_id = self
            .pages
            .iter()
            .find(|(_, state)| state.id() == page_id)
            .map(|(tid, _)| tid.clone())
            .ok_or_else(|| anyhow::anyhow!("No page found with id {page_id}"))?;

        self.selected_page = Some(target_id);
        Ok(())
    }

    /// Get a page by its MCP page ID.
    pub fn get_page(&self, page_id: u32) -> Option<&McpPageState> {
        self.pages.values().find(|state| state.id() == page_id)
    }

    /// Get a page mutably by its MCP page ID.
    pub fn get_page_mut(&mut self, page_id: u32) -> Option<&mut McpPageState> {
        self.pages.values_mut().find(|state| state.id() == page_id)
    }

    /// List all pages.
    pub fn list_pages(&self) -> Vec<&McpPageState> {
        self.pages.values().collect()
    }

    /// Remove a page from the context.
    pub fn remove_page(&mut self, target_id: &str) {
        self.pages.remove(target_id);
        if self.selected_page.as_deref() == Some(target_id) {
            self.selected_page = self.pages.keys().next().cloned();
        }
    }

    /// Get the next snapshot ID and increment.
    pub fn next_snapshot_id(&mut self) -> u32 {
        let id = self.next_snapshot_id;
        self.next_snapshot_id += 1;
        id
    }

    /// Network collector (cloneable handle).
    pub fn network_collector(&self) -> &NetworkCollector {
        &self.network_collector
    }

    /// Console collector (cloneable handle).
    pub fn console_collector(&self) -> &ConsoleCollector {
        &self.console_collector
    }

    /// Extension registry.
    pub fn extension_registry(&self) -> &ExtensionRegistry {
        &self.extension_registry
    }

    pub fn extension_registry_mut(&mut self) -> &mut ExtensionRegistry {
        &mut self.extension_registry
    }

    /// Performance trace state.
    pub fn is_running_trace(&self) -> bool {
        self.is_running_trace
    }

    pub fn set_running_trace(&mut self, running: bool) {
        self.is_running_trace = running;
    }

    pub fn store_trace_result(&mut self, result: serde_json::Value) {
        self.trace_results.clear();
        self.trace_results.push(result);
    }

    pub fn trace_results(&self) -> &[serde_json::Value] {
        &self.trace_results
    }
}

/// Thread-safe shared context handle.
pub type SharedContext = Arc<Mutex<McpContext>>;
