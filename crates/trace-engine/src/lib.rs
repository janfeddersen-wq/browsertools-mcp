//! # trace-engine
//!
//! Chrome performance trace parsing and Web Vitals extraction.
//!
//! Parses Chrome trace JSON format, extracts key performance metrics
//! (LCP, FCP, CLS, INP, TBT), and generates actionable insights.

pub mod formatter;
pub mod insights;
pub mod model;
pub mod parser;

pub use formatter::format_trace_summary;
pub use insights::{extract_metrics, generate_insights};
pub use model::{TraceEvent, TraceInsight, TraceMetrics};
pub use parser::parse_trace;
