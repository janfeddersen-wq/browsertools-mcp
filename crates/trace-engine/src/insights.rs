//! Web Vitals and performance insight extraction from trace events.

use crate::model::{InsightSeverity, TraceEvent, TraceInsight, TraceMetrics, TraceNetworkRequest};

/// Extract performance metrics from parsed trace events.
pub fn extract_metrics(events: &[TraceEvent]) -> TraceMetrics {
    let mut metrics = TraceMetrics::default();

    let mut long_task_total_excess = 0.0;
    let mut layout_shifts = Vec::new();

    for event in events {
        match event.name.as_str() {
            "largestContentfulPaint::Candidate" => {
                if let Some(ts) = event.args.get("data").and_then(|d| d.get("candidateIndex")) {
                    // LCP time is in the timestamp.
                    let _ = ts; // Use navigation start to compute relative time.
                    metrics.lcp_ms = Some(event.ts / 1000.0);
                }
            }
            "firstContentfulPaint" => {
                metrics.fcp_ms = Some(event.ts / 1000.0);
            }
            "ResourceSendRequest" => {
                if let Some(data) = event.args.get("data") {
                    let url = data
                        .get("url")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let method = data
                        .get("requestMethod")
                        .and_then(|v| v.as_str())
                        .unwrap_or("GET")
                        .to_string();
                    let render_blocking = data
                        .get("renderBlocking")
                        .and_then(|v| v.as_str())
                        .unwrap_or("non_blocking")
                        != "non_blocking";

                    metrics.network_requests.push(TraceNetworkRequest {
                        url,
                        method,
                        status_code: None,
                        start_time_ms: event.ts / 1000.0,
                        end_time_ms: None,
                        transfer_size: None,
                        resource_type: data
                            .get("resourceType")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string()),
                        render_blocking,
                    });
                }
            }
            "RunTask" => {
                if let Some(dur) = event.dur
                    && dur / 1000.0 > 50.0
                {
                    let dur_ms = dur / 1000.0;
                    metrics.long_task_count += 1;
                    long_task_total_excess += dur_ms - 50.0;
                }
            }
            "LayoutShift" => {
                if let Some(data) = event.args.get("data")
                    && let Some(score) = data.get("weighted_score_delta").and_then(|v| v.as_f64())
                    && !data
                        .get("had_recent_input")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                {
                    layout_shifts.push(score);
                }
            }
            _ => {}
        }
    }

    metrics.tbt_ms = Some(long_task_total_excess);
    metrics.cls = Some(layout_shifts.iter().sum());

    metrics
}

/// Generate performance insights from extracted metrics.
pub fn generate_insights(metrics: &TraceMetrics) -> Vec<TraceInsight> {
    let mut insights = Vec::new();

    // LCP insight
    if let Some(lcp) = metrics.lcp_ms {
        let severity = if lcp > 4000.0 {
            InsightSeverity::Error
        } else if lcp > 2500.0 {
            InsightSeverity::Warning
        } else {
            InsightSeverity::Info
        };

        insights.push(TraceInsight {
            name: "LCP".to_string(),
            description: format!("Largest Contentful Paint: {:.0}ms", lcp),
            details: match severity {
                InsightSeverity::Error => "LCP is poor (>4s). Consider optimizing images, reducing server response time, and eliminating render-blocking resources.".to_string(),
                InsightSeverity::Warning => "LCP needs improvement (>2.5s). Review resource loading priorities and critical rendering path.".to_string(),
                InsightSeverity::Info => "LCP is good (<2.5s).".to_string(),
            },
            severity,
        });
    }

    // CLS insight
    if let Some(cls) = metrics.cls {
        let severity = if cls > 0.25 {
            InsightSeverity::Error
        } else if cls > 0.1 {
            InsightSeverity::Warning
        } else {
            InsightSeverity::Info
        };

        insights.push(TraceInsight {
            name: "CLS".to_string(),
            description: format!("Cumulative Layout Shift: {:.3}", cls),
            details: match severity {
                InsightSeverity::Error => "CLS is poor (>0.25). Set explicit dimensions on images/ads, avoid dynamically injected content.".to_string(),
                InsightSeverity::Warning => "CLS needs improvement (>0.1). Review elements that shift during page load.".to_string(),
                InsightSeverity::Info => "CLS is good (<0.1).".to_string(),
            },
            severity,
        });
    }

    // Long tasks insight
    if metrics.long_task_count > 0 {
        insights.push(TraceInsight {
            name: "Long Tasks".to_string(),
            description: format!(
                "{} long tasks detected, TBT: {:.0}ms",
                metrics.long_task_count,
                metrics.tbt_ms.unwrap_or(0.0)
            ),
            details: "Long tasks (>50ms) block the main thread. Consider code splitting, deferring non-critical work, or moving computation to web workers.".to_string(),
            severity: if metrics.tbt_ms.unwrap_or(0.0) > 600.0 {
                InsightSeverity::Error
            } else if metrics.tbt_ms.unwrap_or(0.0) > 200.0 {
                InsightSeverity::Warning
            } else {
                InsightSeverity::Info
            },
        });
    }

    // Render-blocking resources insight
    let render_blocking: Vec<_> = metrics
        .network_requests
        .iter()
        .filter(|r| r.render_blocking)
        .collect();

    if !render_blocking.is_empty() {
        let urls: Vec<_> = render_blocking.iter().map(|r| r.url.as_str()).collect();
        insights.push(TraceInsight {
            name: "Render Blocking Resources".to_string(),
            description: format!("{} render-blocking resources found", render_blocking.len()),
            details: format!("These resources block rendering:\n{}", urls.join("\n")),
            severity: InsightSeverity::Warning,
        });
    }

    insights
}
