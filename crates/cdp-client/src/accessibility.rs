//! CDP Accessibility domain — accessibility tree snapshots.

use serde::{Deserialize, Serialize};

use crate::error::CdpResult;
use crate::session::CdpSession;

/// An accessibility tree node from CDP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXNode {
    pub node_id: String,
    pub ignored: bool,
    #[serde(default)]
    pub role: Option<AXValue>,
    #[serde(default)]
    pub name: Option<AXValue>,
    #[serde(default)]
    pub description: Option<AXValue>,
    #[serde(default)]
    pub value: Option<AXValue>,
    #[serde(default)]
    pub properties: Vec<AXProperty>,
    #[serde(default)]
    pub child_ids: Vec<String>,
    #[serde(default)]
    pub backend_dom_node_id: Option<i64>,
    #[serde(default)]
    pub frame_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXValue {
    #[serde(rename = "type")]
    pub value_type: String,
    #[serde(default)]
    pub value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXProperty {
    pub name: String,
    pub value: AXValue,
}

/// A simplified accessibility node for MCP consumption, built into a tree.
#[derive(Debug, Clone, Serialize)]
pub struct AccessibilityNode {
    pub role: String,
    pub name: String,
    pub value: String,
    pub description: String,
    pub backend_node_id: Option<i64>,
    pub properties: Vec<(String, String)>,
    pub children: Vec<AccessibilityNode>,
}

/// Fetch the full accessibility tree for a page.
pub async fn get_full_ax_tree(session: &CdpSession) -> CdpResult<Vec<AXNode>> {
    let result = session
        .send_command("Accessibility.getFullAXTree", serde_json::json!({}))
        .await?;

    let nodes: Vec<AXNode> = serde_json::from_value(
        result
            .get("nodes")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )
    .map_err(|e| crate::error::CdpError::ParseError(format!("AX tree parse error: {e}")))?;

    Ok(nodes)
}

/// Build a tree of simplified AccessibilityNodes from the flat CDP AX node list.
pub fn build_accessibility_tree(nodes: &[AXNode]) -> Option<AccessibilityNode> {
    if nodes.is_empty() {
        return None;
    }

    use std::collections::HashMap;
    let node_map: HashMap<&str, &AXNode> = nodes.iter().map(|n| (n.node_id.as_str(), n)).collect();

    fn build_node(node: &AXNode, node_map: &HashMap<&str, &AXNode>) -> AccessibilityNode {
        let role = node
            .role
            .as_ref()
            .and_then(|v| v.value.as_ref())
            .and_then(|v| v.as_str())
            .unwrap_or("none")
            .to_string();

        let name = node
            .name
            .as_ref()
            .and_then(|v| v.value.as_ref())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let value = node
            .value
            .as_ref()
            .and_then(|v| v.value.as_ref())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let description = node
            .description
            .as_ref()
            .and_then(|v| v.value.as_ref())
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let properties = node
            .properties
            .iter()
            .filter_map(|p| {
                let val = p.value.value.as_ref()?.as_str()?.to_string();
                Some((p.name.clone(), val))
            })
            .collect();

        let children = node
            .child_ids
            .iter()
            .filter_map(|child_id| {
                let child_node = node_map.get(child_id.as_str())?;
                if child_node.ignored {
                    // Skip ignored nodes but include their children.
                    Some(
                        child_node
                            .child_ids
                            .iter()
                            .filter_map(|gid| {
                                let gnode = node_map.get(gid.as_str())?;
                                Some(build_node(gnode, node_map))
                            })
                            .collect::<Vec<_>>(),
                    )
                } else {
                    Some(vec![build_node(child_node, node_map)])
                }
            })
            .flatten()
            .collect();

        AccessibilityNode {
            role,
            name,
            value,
            description,
            backend_node_id: node.backend_dom_node_id,
            properties,
            children,
        }
    }

    // The first node is the root.
    let root = &nodes[0];
    Some(build_node(root, &node_map))
}
