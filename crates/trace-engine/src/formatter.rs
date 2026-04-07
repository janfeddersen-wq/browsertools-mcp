//! Trace summary text formatter.

use crate::model::{TraceInsight, TraceMetrics};

/// Format trace metrics and insights into a human-readable summary.
pub fn format_trace_summary(metrics: &TraceMetrics, insights: &[TraceInsight]) -> String {
    let mut output = String::new();

    output.push_str("## Performance Trace Summary\n\n");

    // Web Vitals
    output.push_str("### Web Vitals\n");
    if let Some(lcp) = metrics.lcp_ms {
        output.push_str(&format!("- LCP: {:.0}ms\n", lcp));
    }
    if let Some(fcp) = metrics.fcp_ms {
        output.push_str(&format!("- FCP: {:.0}ms\n", fcp));
    }
    if let Some(cls) = metrics.cls {
        output.push_str(&format!("- CLS: {:.3}\n", cls));
    }
    if let Some(inp) = metrics.inp_ms {
        output.push_str(&format!("- INP: {:.0}ms\n", inp));
    }
    if let Some(tbt) = metrics.tbt_ms {
        output.push_str(&format!("- TBT: {:.0}ms\n", tbt));
    }
    output.push('\n');

    // Long tasks
    if metrics.long_task_count > 0 {
        output.push_str(&format!(
            "### Main Thread\n- {} long tasks (>50ms)\n\n",
            metrics.long_task_count
        ));
    }

    // Network summary
    if !metrics.network_requests.is_empty() {
        output.push_str(&format!(
            "### Network\n- {} requests\n\n",
            metrics.network_requests.len()
        ));
    }

    // Insights
    if !insights.is_empty() {
        output.push_str("### Insights\n");
        for insight in insights {
            let icon = match insight.severity {
                crate::model::InsightSeverity::Error => "[!]",
                crate::model::InsightSeverity::Warning => "[~]",
                crate::model::InsightSeverity::Info => "[i]",
            };
            output.push_str(&format!(
                "{} **{}**: {}\n  {}\n\n",
                icon, insight.name, insight.description, insight.details
            ));
        }
    }

    output
}
