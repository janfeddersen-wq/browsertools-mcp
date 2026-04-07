//! Accessibility tree snapshot formatter.
//!
//! Formats the accessibility tree into a text representation with UIDs
//! that can be referenced by input tools (click, fill, etc.).

use cdp_client::accessibility::AccessibilityNode;

/// Format an accessibility tree into text representation.
pub fn format_snapshot(root: &AccessibilityNode, verbose: bool) -> String {
    let mut output = String::new();
    format_node(root, &mut output, 0, verbose);
    output
}

fn format_node(node: &AccessibilityNode, output: &mut String, depth: usize, verbose: bool) {
    let indent = "  ".repeat(depth);

    // Skip generic/none roles unless verbose.
    let dominated_roles = ["none", "generic", "InlineTextBox", "LineBreak"];
    if !verbose && dominated_roles.contains(&node.role.as_str()) && node.name.is_empty() {
        for child in &node.children {
            format_node(child, output, depth, verbose);
        }
        return;
    }

    let mut line = format!("{indent}- {}", node.role);

    if !node.name.is_empty() {
        line.push_str(&format!(" \"{}\"", node.name));
    }

    if !node.value.is_empty() {
        line.push_str(&format!(" value=\"{}\"", node.value));
    }

    if verbose && !node.description.is_empty() {
        line.push_str(&format!(" description=\"{}\"", node.description));
    }

    if verbose {
        for (key, val) in &node.properties {
            line.push_str(&format!(" {key}={val}"));
        }
    }

    output.push_str(&line);
    output.push('\n');

    for child in &node.children {
        format_node(child, output, depth + 1, verbose);
    }
}
