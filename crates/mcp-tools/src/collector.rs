//! Event collectors -- network requests and console messages.
//!
//! Collects CDP events per page, with navigation-based splitting
//! to preserve data from the last N navigations.
//!
//! Collectors use `Arc<Mutex<>>` internally so they can be shared
//! across async tasks (event listener tasks and tool handlers).

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

/// Inner data for the network collector.
#[derive(Debug, Default)]
struct NetworkCollectorInner {
    /// Page target ID -> list of navigation buckets -> requests in that navigation.
    data: HashMap<String, Vec<Vec<serde_json::Value>>>,
    /// Stable ID counter for requests.
    next_id: u32,
    /// Request -> stable ID mapping.
    id_map: HashMap<String, u32>,
}

/// Collects network requests per page.
///
/// Cloneable and safe to share across async tasks.
#[derive(Debug, Clone, Default)]
pub struct NetworkCollector {
    inner: Arc<Mutex<NetworkCollectorInner>>,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start collecting for a page.
    pub async fn add_page(&self, target_id: &str) {
        let mut inner = self.inner.lock().await;
        inner
            .data
            .entry(target_id.to_string())
            .or_insert_with(|| vec![Vec::new()]);
    }

    /// Record a new navigation for a page (starts a new bucket).
    pub async fn on_navigation(&self, target_id: &str) {
        let mut inner = self.inner.lock().await;
        if let Some(buckets) = inner.data.get_mut(target_id) {
            buckets.push(Vec::new());
            // Keep last 3 navigations.
            while buckets.len() > 3 {
                buckets.remove(0);
            }
        }
    }

    /// Add a network request event.
    pub async fn add_request(&self, target_id: &str, request: serde_json::Value) {
        let mut inner = self.inner.lock().await;
        // Assign stable ID before borrowing data mutably.
        if let Some(req_id) = request.get("requestId").and_then(|v| v.as_str()) {
            let id = inner.next_id;
            inner.next_id += 1;
            inner.id_map.insert(req_id.to_string(), id);
        }
        if let Some(buckets) = inner.data.get_mut(target_id)
            && let Some(current) = buckets.last_mut()
        {
            current.push(request);
        }
    }

    /// Get all requests for a page (current navigation only).
    pub async fn get_requests(&self, target_id: &str) -> Vec<serde_json::Value> {
        let inner = self.inner.lock().await;
        inner
            .data
            .get(target_id)
            .and_then(|buckets| buckets.last())
            .cloned()
            .unwrap_or_default()
    }

    /// Get the stable ID for a request.
    pub async fn get_request_id(&self, cdp_request_id: &str) -> Option<u32> {
        let inner = self.inner.lock().await;
        inner.id_map.get(cdp_request_id).copied()
    }
}

/// Inner data for the console collector.
#[derive(Debug, Default)]
struct ConsoleCollectorInner {
    data: HashMap<String, Vec<Vec<serde_json::Value>>>,
    next_id: u32,
}

/// Collects console messages per page.
///
/// Cloneable and safe to share across async tasks.
#[derive(Debug, Clone, Default)]
pub struct ConsoleCollector {
    inner: Arc<Mutex<ConsoleCollectorInner>>,
}

impl ConsoleCollector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start collecting for a page.
    pub async fn add_page(&self, target_id: &str) {
        let mut inner = self.inner.lock().await;
        inner
            .data
            .entry(target_id.to_string())
            .or_insert_with(|| vec![Vec::new()]);
    }

    /// Record a new navigation.
    pub async fn on_navigation(&self, target_id: &str) {
        let mut inner = self.inner.lock().await;
        if let Some(buckets) = inner.data.get_mut(target_id) {
            buckets.push(Vec::new());
            while buckets.len() > 3 {
                buckets.remove(0);
            }
        }
    }

    /// Add a console message event.
    pub async fn add_message(&self, target_id: &str, message: serde_json::Value) {
        let mut inner = self.inner.lock().await;
        if let Some(buckets) = inner.data.get_mut(target_id)
            && let Some(current) = buckets.last_mut()
        {
            current.push(message);
        }
    }

    /// Get all messages for a page (current + optionally preserved navigations).
    pub async fn get_messages(
        &self,
        target_id: &str,
        include_preserved: bool,
    ) -> Vec<serde_json::Value> {
        let inner = self.inner.lock().await;
        inner
            .data
            .get(target_id)
            .map(|buckets| {
                if include_preserved {
                    buckets.iter().flat_map(|b| b.iter()).cloned().collect()
                } else {
                    buckets.last().map(|b| b.to_vec()).unwrap_or_default()
                }
            })
            .unwrap_or_default()
    }

    /// Get the next message ID and increment.
    pub async fn next_id(&self) -> u32 {
        let mut inner = self.inner.lock().await;
        let id = inner.next_id;
        inner.next_id += 1;
        id
    }
}
