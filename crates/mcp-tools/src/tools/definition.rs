//! Tool definition types — schema, annotations, and handler traits.

use std::future::Future;
use std::pin::Pin;

use super::categories::ToolCategory;

/// Annotations attached to a tool for filtering and metadata.
#[derive(Debug, Clone)]
pub struct ToolAnnotations {
    pub category: ToolCategory,
    pub read_only_hint: bool,
    pub destructive_hint: bool,
    pub conditions: Vec<String>,
}

impl Default for ToolAnnotations {
    fn default() -> Self {
        Self {
            category: ToolCategory::Debugging,
            read_only_hint: true,
            destructive_hint: false,
            conditions: Vec::new(),
        }
    }
}

/// Result of a tool invocation, matching MCP's CallToolResult.
#[derive(Debug, Clone)]
pub struct ToolResult {
    pub content: Vec<ToolContent>,
    pub is_error: bool,
}

/// A content block in a tool result.
#[derive(Debug, Clone)]
pub enum ToolContent {
    Text { text: String },
    Image { data: String, mime_type: String },
}

impl ToolResult {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text { text: text.into() }],
            is_error: false,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![ToolContent::Text {
                text: message.into(),
            }],
            is_error: true,
        }
    }

    pub fn with_image(mut self, data: String, mime_type: String) -> Self {
        self.content.push(ToolContent::Image { data, mime_type });
        self
    }
}

/// Type alias for the async tool handler function.
pub type ToolHandler = Box<
    dyn Fn(serde_json::Value) -> Pin<Box<dyn Future<Output = anyhow::Result<ToolResult>> + Send>>
        + Send
        + Sync,
>;

/// A registered MCP tool with its metadata and handler.
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub annotations: ToolAnnotations,
    pub page_scoped: bool,
    pub handler: ToolHandler,
}
