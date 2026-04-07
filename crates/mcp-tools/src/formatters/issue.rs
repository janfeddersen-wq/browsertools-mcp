//! Browser issue formatter.
//!
//! Formats CDP `Audits.issueAdded` events into human-readable descriptions.

/// Format a browser issue from a CDP `Audits.issueAdded` event.
///
/// Output example:
/// ```text
/// [Warning] Mixed Content: The page loaded over HTTPS but requested an insecure resource...
///   Category: Security
///   Code: MixedContent
///   URL: http://example.com/image.png
/// ```
pub fn format_issue(issue: &serde_json::Value) -> String {
    // CDP issues come wrapped in an `issue` field from the event.
    let issue_data = issue.get("issue").unwrap_or(issue);

    let code = issue_data
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("UnknownIssue");

    let severity = extract_severity(issue_data);
    let severity_label = match severity {
        IssueSeverity::Error => "Error",
        IssueSeverity::Warning => "Warning",
        IssueSeverity::Info => "Info",
    };

    let category = categorize_issue(code);
    let description = describe_issue(code, issue_data);

    let mut out = format!("[{severity_label}] {description}");
    out.push_str(&format!("\n  Category: {category}"));
    out.push_str(&format!("\n  Code: {code}"));

    // Extract affected resource URLs if available.
    if let Some(details) = extract_issue_details(issue_data) {
        for (key, value) in details {
            out.push_str(&format!("\n  {key}: {value}"));
        }
    }

    out
}

/// Format multiple issues into a summary.
pub fn format_issues(issues: &[serde_json::Value]) -> String {
    if issues.is_empty() {
        return "No browser issues detected.".to_string();
    }

    let mut out = format!("{} issue(s) detected:\n\n", issues.len());
    for (i, issue) in issues.iter().enumerate() {
        out.push_str(&format!("{}. {}\n\n", i + 1, format_issue(issue)));
    }
    out
}

#[derive(Debug, Clone, Copy)]
enum IssueSeverity {
    Error,
    Warning,
    Info,
}

/// Extract severity from the issue data.
fn extract_severity(issue: &serde_json::Value) -> IssueSeverity {
    match issue.get("severity").and_then(|v| v.as_str()).unwrap_or("") {
        "Error" => IssueSeverity::Error,
        "Warning" => IssueSeverity::Warning,
        "Information" | "Info" => IssueSeverity::Info,
        _ => {
            // Infer from issue code.
            let code = issue.get("code").and_then(|v| v.as_str()).unwrap_or("");
            if code.contains("Block") || code.contains("Error") {
                IssueSeverity::Error
            } else {
                IssueSeverity::Warning
            }
        }
    }
}

/// Categorize an issue by its code.
fn categorize_issue(code: &str) -> &'static str {
    match code {
        c if c.starts_with("MixedContent") => "Security",
        c if c.starts_with("SameSiteCookie") || c.contains("Cookie") => "Cookie",
        c if c.starts_with("BlockedByResponse") || c.contains("CORS") => "CORS",
        c if c.starts_with("ContentSecurityPolicy") || c.contains("CSP") => "Security",
        c if c.starts_with("Heavy") || c.contains("Performance") => "Performance",
        c if c.starts_with("Deprecation") || c.contains("Deprecated") => "Deprecation",
        c if c.contains("SharedArray") || c.contains("CrossOriginIsolation") => "Security",
        c if c.starts_with("LowTextContrast") || c.contains("Accessibility") => "Accessibility",
        c if c.starts_with("Attribution") => "Attribution Reporting",
        c if c.starts_with("Quirks") => "Quirks Mode",
        c if c.starts_with("Navigator") => "Privacy",
        c if c.contains("Federated") || c.contains("FedCM") => "Privacy",
        _ => "Other",
    }
}

/// Generate a human-readable description for an issue.
fn describe_issue(code: &str, issue: &serde_json::Value) -> String {
    // Try to extract a description from the details.
    if let Some(details) = get_details_object(issue) {
        // Mixed content issues.
        if let Some(mixed) = details.get("mixedContentIssueDetails") {
            let resolution = mixed
                .get("resolutionStatus")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let resource_type = mixed
                .get("resourceType")
                .and_then(|v| v.as_str())
                .unwrap_or("resource");
            let url = mixed
                .get("insecureURL")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            return format!(
                "Mixed Content ({resolution}): The page loaded over HTTPS but requested an insecure {resource_type} '{url}'"
            );
        }

        // Cookie issues.
        if let Some(cookie) = details.get("cookieIssueDetails") {
            let reason = cookie
                .get("cookieWarningReasons")
                .or_else(|| cookie.get("cookieExclusionReasons"))
                .and_then(|v| v.as_array())
                .map(|reasons| {
                    reasons
                        .iter()
                        .filter_map(|r| r.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_else(|| "unspecified reason".to_string());
            return format!("Cookie Issue: {reason}");
        }

        // CORS / blocked by response issues.
        if let Some(blocked) = details.get("blockedByResponseIssueDetails") {
            let reason = blocked
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            return format!("Blocked by Response: {reason}");
        }

        // Content Security Policy issues.
        if let Some(csp) = details.get("contentSecurityPolicyIssueDetails") {
            let directive = csp
                .get("violatedDirective")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            return format!("Content Security Policy violation: directive '{directive}'");
        }

        // Heavy ads.
        if let Some(heavy) = details.get("heavyAdIssueDetails") {
            let reason = heavy
                .get("reason")
                .and_then(|v| v.as_str())
                .unwrap_or("resource limits exceeded");
            return format!("Heavy Ad: {reason}");
        }

        // Deprecation issues.
        if let Some(deprecation) = details.get("deprecationIssueDetails") {
            let feature = deprecation
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("feature");
            return format!("Deprecated: {feature}");
        }

        // Low text contrast.
        if let Some(contrast) = details.get("lowTextContrastIssueDetails") {
            let ratio = contrast
                .get("contrastRatio")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let threshold = contrast
                .get("thresholdAA")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            return format!(
                "Low Text Contrast: ratio {ratio:.2} is below threshold {threshold:.2} (WCAG AA)"
            );
        }
    }

    // Fallback: use the code itself as description.
    humanize_code(code)
}

/// Try to get the details object from various issue shapes.
fn get_details_object(issue: &serde_json::Value) -> Option<&serde_json::Value> {
    issue.get("details").or_else(|| {
        // Some events nest details directly.
        if issue.get("mixedContentIssueDetails").is_some()
            || issue.get("cookieIssueDetails").is_some()
            || issue.get("blockedByResponseIssueDetails").is_some()
        {
            Some(issue)
        } else {
            None
        }
    })
}

/// Extract key-value details from an issue for display.
fn extract_issue_details(issue: &serde_json::Value) -> Option<Vec<(String, String)>> {
    let mut details = Vec::new();

    let data = get_details_object(issue)?;

    // Look for affected URLs in various sub-objects.
    for key in &[
        "mixedContentIssueDetails",
        "blockedByResponseIssueDetails",
        "contentSecurityPolicyIssueDetails",
    ] {
        if let Some(sub) = data.get(*key)
            && let Some(url) = sub
                .get("insecureURL")
                .or_else(|| sub.get("blockedURL"))
                .or_else(|| sub.get("blockedUrl"))
                .and_then(|v| v.as_str())
        {
            details.push(("URL".to_string(), url.to_string()));
        }
    }

    // Affected request.
    if let Some(request) = data.get("request").and_then(|v| v.as_object())
        && let Some(url) = request.get("url").and_then(|v| v.as_str())
    {
        details.push(("Request URL".to_string(), url.to_string()));
    }

    // Source code location.
    if let Some(location) = data.get("sourceCodeLocation").and_then(|v| v.as_object()) {
        let url = location.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let line = location
            .get("lineNumber")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let col = location
            .get("columnNumber")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        if !url.is_empty() {
            details.push(("Source".to_string(), format!("{url}:{line}:{col}")));
        }
    }

    if details.is_empty() {
        None
    } else {
        Some(details)
    }
}

/// Convert a PascalCase code to a human-readable string.
fn humanize_code(code: &str) -> String {
    let mut result = String::new();
    for (i, ch) in code.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_format_mixed_content_issue() {
        let issue = json!({
            "code": "MixedContentIssue",
            "severity": "Warning",
            "details": {
                "mixedContentIssueDetails": {
                    "resolutionStatus": "MixedContentBlocked",
                    "resourceType": "image",
                    "insecureURL": "http://example.com/image.png"
                }
            }
        });
        let out = format_issue(&issue);
        assert!(out.contains("[Warning]"));
        assert!(out.contains("Mixed Content"));
        assert!(out.contains("http://example.com/image.png"));
        assert!(out.contains("Category: Security"));
    }

    #[test]
    fn test_format_unknown_issue() {
        let issue = json!({
            "code": "SomeNewIssue",
            "severity": "Info"
        });
        let out = format_issue(&issue);
        assert!(out.contains("[Info]"));
        assert!(out.contains("Some New Issue"));
    }

    #[test]
    fn test_categorize() {
        assert_eq!(categorize_issue("MixedContentBlocked"), "Security");
        assert_eq!(categorize_issue("SameSiteCookieWarning"), "Cookie");
        assert_eq!(categorize_issue("BlockedByResponseCORP"), "CORS");
        assert_eq!(categorize_issue("LowTextContrast"), "Accessibility");
        assert_eq!(categorize_issue("DeprecationIssue"), "Deprecation");
    }
}
