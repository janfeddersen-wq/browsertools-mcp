//! Console message formatter.
//!
//! Formats console messages from CDP `Runtime.consoleAPICalled` events
//! into human-readable text with stack traces.

/// Format a single console message from a CDP `Runtime.consoleAPICalled` event.
///
/// Output example:
/// ```text
/// [error] Uncaught TypeError: x is not a function
///   at foo (script.js:12:5)
///   at bar (script.js:24:10)
/// ```
pub fn format_console_message(msg: &serde_json::Value, index: usize) -> String {
    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("log");

    // Extract text from args array (Runtime.consoleAPICalled format).
    let text = extract_message_text(msg);

    let mut out = format!("[{index}] [{msg_type}] {text}");

    // Source location (if present at top level).
    if let Some(url) = msg.get("url").and_then(|v| v.as_str()) {
        let line = msg.get("lineNumber").and_then(|v| v.as_u64()).unwrap_or(0);
        let col = msg
            .get("columnNumber")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if !url.is_empty() {
            out.push_str(&format!("\n  at ({url}:{line}:{col})"));
        }
    }

    // Stack trace frames.
    if let Some(stack_trace) = msg.get("stackTrace")
        && let Some(frames) = stack_trace.get("callFrames").and_then(|v| v.as_array())
    {
        for frame in frames {
            let func = frame
                .get("functionName")
                .and_then(|v| v.as_str())
                .unwrap_or("<anonymous>");
            let frame_url = frame.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let line = frame
                .get("lineNumber")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let col = frame
                .get("columnNumber")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let location = if frame_url.is_empty() {
                format!("<unknown>:{line}:{col}")
            } else {
                // Show just the filename portion for readability.
                let short_url = shorten_url(frame_url);
                format!("{short_url}:{line}:{col}")
            };
            out.push_str(&format!("\n  at {func} ({location})"));
        }
    }

    out
}

/// Format a paginated list of console messages.
pub fn format_console_messages(
    messages: &[serde_json::Value],
    page: usize,
    page_size: usize,
) -> String {
    if messages.is_empty() {
        return "No console messages.".to_string();
    }

    let (page_items, info) = crate::utils::pagination::paginate(messages, page, page_size);

    let mut out = format!("{info}\n\n");
    for (i, msg) in page_items.iter().enumerate() {
        let idx = (info.page.saturating_sub(1)) * info.page_size + i;
        out.push_str(&format_console_message(msg, idx));
        out.push('\n');
    }
    out
}

/// Extract the display text from a console message.
///
/// CDP `Runtime.consoleAPICalled` puts arguments in an `args` array of RemoteObjects.
/// We concatenate their descriptions or values.
fn extract_message_text(msg: &serde_json::Value) -> String {
    // First try the `args` array (Runtime.consoleAPICalled format).
    if let Some(args) = msg.get("args").and_then(|v| v.as_array()) {
        let parts: Vec<String> = args
            .iter()
            .map(|arg| {
                // Prefer "value" for primitives, "description" for objects.
                if let Some(val) = arg.get("value") {
                    match val {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    }
                } else if let Some(desc) = arg.get("description").and_then(|v| v.as_str()) {
                    desc.to_string()
                } else if let Some(unserializable) =
                    arg.get("unserializableValue").and_then(|v| v.as_str())
                {
                    unserializable.to_string()
                } else {
                    arg.get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("undefined")
                        .to_string()
                }
            })
            .collect();
        if !parts.is_empty() {
            return parts.join(" ");
        }
    }

    // Fallback: plain "text" field.
    msg.get("text")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Shorten a URL to just the filename portion for stack traces.
fn shorten_url(url: &str) -> &str {
    url.rsplit('/').next().unwrap_or(url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_simple_log() {
        let msg = json!({
            "type": "log",
            "args": [{"type": "string", "value": "Hello world"}]
        });
        let out = format_console_message(&msg, 0);
        assert!(out.contains("[log]"));
        assert!(out.contains("Hello world"));
    }

    #[test]
    fn test_format_error_with_stack() {
        let msg = json!({
            "type": "error",
            "args": [{"type": "string", "value": "Uncaught TypeError"}],
            "stackTrace": {
                "callFrames": [
                    {
                        "functionName": "foo",
                        "url": "https://example.com/app.js",
                        "lineNumber": 12,
                        "columnNumber": 5
                    },
                    {
                        "functionName": "bar",
                        "url": "https://example.com/app.js",
                        "lineNumber": 24,
                        "columnNumber": 10
                    }
                ]
            }
        });
        let out = format_console_message(&msg, 3);
        assert!(out.contains("[3] [error] Uncaught TypeError"));
        assert!(out.contains("at foo (app.js:12:5)"));
        assert!(out.contains("at bar (app.js:24:10)"));
    }

    #[test]
    fn test_format_multiple_args() {
        let msg = json!({
            "type": "log",
            "args": [
                {"type": "string", "value": "count:"},
                {"type": "number", "value": 42}
            ]
        });
        let out = format_console_message(&msg, 0);
        assert!(out.contains("count: 42"));
    }

    #[test]
    fn test_format_pagination() {
        let messages: Vec<serde_json::Value> = (0..5)
            .map(|i| {
                json!({
                    "type": "log",
                    "args": [{"type": "string", "value": format!("msg {i}")}]
                })
            })
            .collect();
        let out = format_console_messages(&messages, 1, 3);
        assert!(out.contains("Page 1/2"));
        assert!(out.contains("msg 0"));
        assert!(out.contains("msg 2"));
        assert!(!out.contains("msg 3"));
    }
}
