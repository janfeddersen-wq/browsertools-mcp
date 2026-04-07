//! Network request formatter.
//!
//! Formats network requests/responses from CDP events into human-readable output.

/// Format a single network request summary line.
///
/// Output example:
/// ```text
/// [1] GET https://example.com/api -> 200 (application/json, 1.2KB, 150ms)
/// ```
pub fn format_network_request(req: &serde_json::Value, index: usize) -> String {
    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
    let url = req.get("url").and_then(|v| v.as_str()).unwrap_or("?");
    let status = req
        .get("status")
        .and_then(|v| v.as_i64())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "pending".to_string());

    let mut meta = Vec::new();

    // Content type.
    if let Some(content_type) = req
        .get("mimeType")
        .or_else(|| req.get("contentType"))
        .and_then(|v| v.as_str())
        && !content_type.is_empty()
    {
        meta.push(content_type.to_string());
    }

    // Response size.
    if let Some(size) = req
        .get("encodedDataLength")
        .or_else(|| req.get("dataLength"))
        .and_then(|v| v.as_f64())
    {
        meta.push(format_bytes(size));
    }

    // Timing.
    if let Some(duration_ms) = extract_duration_ms(req) {
        meta.push(format_duration(duration_ms));
    }

    let meta_str = if meta.is_empty() {
        String::new()
    } else {
        format!(" ({})", meta.join(", "))
    };

    format!("[{index}] {method} {url} -> {status}{meta_str}")
}

/// Format a paginated, filtered list of network requests.
pub fn format_network_requests(
    requests: &[serde_json::Value],
    url_filter: Option<&str>,
    type_filter: Option<&str>,
    page: usize,
    page_size: usize,
) -> String {
    // Apply filters.
    let filtered: Vec<&serde_json::Value> = requests
        .iter()
        .filter(|req| {
            if let Some(uf) = url_filter {
                let url = req.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if !url.contains(uf) {
                    return false;
                }
            }
            if let Some(tf) = type_filter {
                let rtype = req
                    .get("resourceType")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if !rtype.eq_ignore_ascii_case(tf) {
                    return false;
                }
            }
            true
        })
        .collect();

    if filtered.is_empty() {
        return "No matching network requests.".to_string();
    }

    // Collect into owned values for pagination.
    let owned: Vec<serde_json::Value> = filtered.into_iter().cloned().collect();
    let (page_items, info) = crate::utils::pagination::paginate(&owned, page, page_size);

    let mut out = format!("{info}\n\n");
    for (i, req) in page_items.iter().enumerate() {
        let idx = (info.page.saturating_sub(1)) * info.page_size + i;
        out.push_str(&format_network_request(req, idx));
        out.push('\n');
    }
    out
}

/// Format full details of a single network request.
pub fn format_network_request_detail(req: &serde_json::Value) -> String {
    let mut out = String::new();

    let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
    let url = req.get("url").and_then(|v| v.as_str()).unwrap_or("?");
    let status = req
        .get("status")
        .and_then(|v| v.as_i64())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "pending".to_string());
    let status_text = req.get("statusText").and_then(|v| v.as_str()).unwrap_or("");

    out.push_str(&format!("## {method} {url}\n\n"));
    out.push_str(&format!(
        "**Status:** {status}{}\n",
        if status_text.is_empty() {
            String::new()
        } else {
            format!(" {status_text}")
        }
    ));

    // Resource type.
    if let Some(rt) = req.get("resourceType").and_then(|v| v.as_str()) {
        out.push_str(&format!("**Resource Type:** {rt}\n"));
    }

    // MIME type.
    if let Some(mime) = req.get("mimeType").and_then(|v| v.as_str()) {
        out.push_str(&format!("**MIME Type:** {mime}\n"));
    }

    // Size.
    if let Some(size) = req
        .get("encodedDataLength")
        .or_else(|| req.get("dataLength"))
        .and_then(|v| v.as_f64())
    {
        out.push_str(&format!("**Size:** {}\n", format_bytes(size)));
    }

    // Timing.
    if let Some(duration_ms) = extract_duration_ms(req) {
        out.push_str(&format!("**Duration:** {}\n", format_duration(duration_ms)));
    }

    // Request headers.
    if let Some(headers) = req.get("requestHeaders").and_then(|v| v.as_object()) {
        out.push_str("\n### Request Headers\n");
        for (key, value) in headers {
            let val_owned = value.to_string();
            let val_str = value.as_str().unwrap_or(&val_owned);
            out.push_str(&format!("  {key}: {val_str}\n"));
        }
    }

    // Response headers.
    if let Some(headers) = req.get("responseHeaders").and_then(|v| v.as_object()) {
        out.push_str("\n### Response Headers\n");
        for (key, value) in headers {
            let val_owned = value.to_string();
            let val_str = value.as_str().unwrap_or(&val_owned);
            out.push_str(&format!("  {key}: {val_str}\n"));
        }
    }

    // Timing breakdown.
    if let Some(timing) = req.get("timing").and_then(|v| v.as_object()) {
        out.push_str("\n### Timing Breakdown\n");
        let timing_fields = [
            ("dnsStart", "dnEnd", "DNS Lookup"),
            ("connectStart", "connectEnd", "Connection"),
            ("sslStart", "sslEnd", "TLS/SSL"),
            ("sendStart", "sendEnd", "Request Sent"),
            ("receiveHeadersStart", "receiveHeadersEnd", "Waiting (TTFB)"),
        ];
        for (start_field, end_field, label) in &timing_fields {
            if let (Some(start), Some(end)) = (
                timing.get(*start_field).and_then(|v| v.as_f64()),
                timing.get(*end_field).and_then(|v| v.as_f64()),
            ) && start >= 0.0
                && end >= 0.0
            {
                let duration = end - start;
                out.push_str(&format!("  {label}: {:.1}ms\n", duration));
            }
        }
    }

    // Body preview.
    if let Some(body) = req.get("body").and_then(|v| v.as_str()) {
        out.push_str("\n### Body Preview\n");
        let preview: String = body.chars().take(1000).collect();
        out.push_str(&preview);
        if body.len() > 1000 {
            out.push_str("\n... (truncated)");
        }
        out.push('\n');
    }

    out
}

/// Format byte count into a human-readable string.
fn format_bytes(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{:.0}B", bytes)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.1}KB", bytes / 1024.0)
    } else {
        format!("{:.1}MB", bytes / (1024.0 * 1024.0))
    }
}

/// Format millisecond duration into a human-readable string.
fn format_duration(ms: f64) -> String {
    if ms < 1000.0 {
        format!("{:.0}ms", ms)
    } else {
        format!("{:.2}s", ms / 1000.0)
    }
}

/// Extract request duration in milliseconds from timing data.
fn extract_duration_ms(req: &serde_json::Value) -> Option<f64> {
    // Try explicit duration field first.
    if let Some(dur) = req.get("duration").and_then(|v| v.as_f64()) {
        return Some(dur);
    }

    // Try computing from timing object.
    if let Some(timing) = req.get("timing").and_then(|v| v.as_object()) {
        let request_time = timing.get("requestTime").and_then(|v| v.as_f64())?;
        let receive_end = timing
            .get("receiveHeadersEnd")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        if receive_end > 0.0 {
            // requestTime is in seconds, offsets are in ms.
            return Some(receive_end);
        }
        // Fallback: use the wallTime difference if available.
        let _ = request_time;
    }

    // Try start/end timestamps.
    if let (Some(start), Some(end)) = (
        req.get("startTime").and_then(|v| v.as_f64()),
        req.get("endTime").and_then(|v| v.as_f64()),
    ) {
        return Some(end - start);
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_request_summary() {
        let req = json!({
            "method": "GET",
            "url": "https://example.com/api/data",
            "status": 200,
            "mimeType": "application/json",
            "encodedDataLength": 1234.0,
            "duration": 150.0
        });
        let out = format_network_request(&req, 1);
        assert!(out.contains("[1] GET https://example.com/api/data -> 200"));
        assert!(out.contains("application/json"));
        assert!(out.contains("1.2KB"));
        assert!(out.contains("150ms"));
    }

    #[test]
    fn test_format_pending_request() {
        let req = json!({
            "method": "POST",
            "url": "https://example.com/submit"
        });
        let out = format_network_request(&req, 0);
        assert!(out.contains("POST"));
        assert!(out.contains("pending"));
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512.0), "512B");
        assert_eq!(format_bytes(1536.0), "1.5KB");
        assert_eq!(format_bytes(1048576.0), "1.0MB");
    }

    #[test]
    fn test_filter_by_url() {
        let requests = vec![
            json!({"method": "GET", "url": "https://example.com/api", "status": 200}),
            json!({"method": "GET", "url": "https://example.com/style.css", "status": 200}),
            json!({"method": "GET", "url": "https://cdn.example.com/api/v2", "status": 200}),
        ];
        let out = format_network_requests(&requests, Some("api"), None, 1, 25);
        assert!(out.contains("example.com/api"));
        assert!(!out.contains("style.css"));
    }

    #[test]
    fn test_format_detail() {
        let req = json!({
            "method": "GET",
            "url": "https://example.com/api",
            "status": 200,
            "statusText": "OK",
            "mimeType": "application/json",
            "resourceType": "Fetch",
            "encodedDataLength": 2048.0,
            "requestHeaders": {"Accept": "application/json"},
            "responseHeaders": {"Content-Type": "application/json"},
            "body": "{\"key\": \"value\"}"
        });
        let out = format_network_request_detail(&req);
        assert!(out.contains("## GET https://example.com/api"));
        assert!(out.contains("**Status:** 200 OK"));
        assert!(out.contains("Request Headers"));
        assert!(out.contains("Response Headers"));
        assert!(out.contains("Body Preview"));
    }
}
