//! McpPageState — per-page state wrapper.
//!
//! Equivalent to McpPage.ts. Holds accessibility snapshot, emulation settings,
//! and UID mapping for each browser page.

use std::collections::HashMap;

use cdp_client::CdpPage;

/// Per-page MCP state, wrapping a CDP page handle.
pub struct McpPageState {
    /// MCP-assigned page ID (stable across snapshots).
    id: u32,

    /// The underlying CDP page handle.
    cdp_page: CdpPage,

    /// Current accessibility snapshot (if taken).
    snapshot: Option<TextSnapshot>,

    /// Emulation settings applied to this page.
    emulation: EmulationSettings,

    /// Maps (loaderId + backendNodeId) -> MCP UID for stable element references.
    unique_backend_node_id_to_mcp_id: HashMap<String, String>,

    /// Isolated context name, if this page belongs to one.
    isolated_context_name: Option<String>,
}

impl McpPageState {
    pub fn new(id: u32, cdp_page: CdpPage) -> Self {
        Self {
            id,
            cdp_page,
            snapshot: None,
            emulation: EmulationSettings::default(),
            unique_backend_node_id_to_mcp_id: HashMap::new(),
            isolated_context_name: None,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn cdp_page(&self) -> &CdpPage {
        &self.cdp_page
    }

    pub fn cdp_page_mut(&mut self) -> &mut CdpPage {
        &mut self.cdp_page
    }

    pub fn url(&self) -> &str {
        self.cdp_page.url()
    }

    pub fn snapshot(&self) -> Option<&TextSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn set_snapshot(&mut self, snapshot: TextSnapshot) {
        self.snapshot = Some(snapshot);
    }

    pub fn emulation(&self) -> &EmulationSettings {
        &self.emulation
    }

    pub fn set_emulation(&mut self, settings: EmulationSettings) {
        self.emulation = settings;
    }

    pub fn uid_map(&self) -> &HashMap<String, String> {
        &self.unique_backend_node_id_to_mcp_id
    }

    pub fn uid_map_mut(&mut self) -> &mut HashMap<String, String> {
        &mut self.unique_backend_node_id_to_mcp_id
    }

    pub fn isolated_context_name(&self) -> Option<&str> {
        self.isolated_context_name.as_deref()
    }

    pub fn set_isolated_context_name(&mut self, name: Option<String>) {
        self.isolated_context_name = name;
    }
}

/// Accessibility tree snapshot with UID-annotated nodes.
#[derive(Debug, Clone)]
pub struct TextSnapshot {
    pub snapshot_id: String,
    pub root: TextSnapshotNode,
    pub id_to_node: HashMap<String, TextSnapshotNode>,
    pub verbose: bool,
}

/// A node in the text snapshot tree with an assigned UID.
#[derive(Debug, Clone)]
pub struct TextSnapshotNode {
    pub id: String,
    pub role: String,
    pub name: String,
    pub value: String,
    pub description: String,
    pub backend_node_id: Option<i64>,
    pub children: Vec<TextSnapshotNode>,
}

/// Emulation settings applied to a page.
#[derive(Debug, Clone, Default)]
pub struct EmulationSettings {
    pub network_conditions: Option<String>,
    pub cpu_throttling_rate: Option<f64>,
    pub geolocation: Option<GeolocationSettings>,
    pub user_agent: Option<String>,
    pub color_scheme: Option<String>,
    pub viewport: Option<ViewportSettings>,
}

#[derive(Debug, Clone)]
pub struct GeolocationSettings {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct ViewportSettings {
    pub width: u32,
    pub height: u32,
    pub device_scale_factor: f64,
    pub is_mobile: bool,
    pub has_touch: bool,
    pub is_landscape: bool,
}
