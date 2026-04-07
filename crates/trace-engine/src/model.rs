//! Chrome trace event data types.

use serde::{Deserialize, Serialize};

/// A Chrome trace event (from the JSON trace format).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    /// Event name.
    #[serde(default)]
    pub name: String,

    /// Event category.
    #[serde(default, rename = "cat")]
    pub category: String,

    /// Phase: B (begin), E (end), X (complete), I (instant), etc.
    #[serde(default)]
    pub ph: String,

    /// Timestamp in microseconds.
    #[serde(default)]
    pub ts: f64,

    /// Duration in microseconds (for X events).
    #[serde(default)]
    pub dur: Option<f64>,

    /// Process ID.
    #[serde(default)]
    pub pid: i64,

    /// Thread ID.
    #[serde(default)]
    pub tid: i64,

    /// Event arguments.
    #[serde(default)]
    pub args: serde_json::Value,
}

/// Parsed trace data with extracted metrics.
#[derive(Debug, Clone, Default, Serialize)]
pub struct TraceMetrics {
    /// Largest Contentful Paint time in ms.
    pub lcp_ms: Option<f64>,

    /// First Contentful Paint time in ms.
    pub fcp_ms: Option<f64>,

    /// Time to First Byte in ms.
    pub ttfb_ms: Option<f64>,

    /// Cumulative Layout Shift score.
    pub cls: Option<f64>,

    /// Interaction to Next Paint in ms.
    pub inp_ms: Option<f64>,

    /// Total blocking time in ms (sum of long task excess over 50ms).
    pub tbt_ms: Option<f64>,

    /// Number of long tasks (>50ms).
    pub long_task_count: usize,

    /// Network requests found in the trace.
    pub network_requests: Vec<TraceNetworkRequest>,
}

/// A network request extracted from trace events.
#[derive(Debug, Clone, Serialize)]
pub struct TraceNetworkRequest {
    pub url: String,
    pub method: String,
    pub status_code: Option<u32>,
    pub start_time_ms: f64,
    pub end_time_ms: Option<f64>,
    pub transfer_size: Option<u64>,
    pub resource_type: Option<String>,
    pub render_blocking: bool,
}

/// A performance insight derived from trace analysis.
#[derive(Debug, Clone, Serialize)]
pub struct TraceInsight {
    pub name: String,
    pub description: String,
    pub details: String,
    pub severity: InsightSeverity,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InsightSeverity {
    Info,
    Warning,
    Error,
}
