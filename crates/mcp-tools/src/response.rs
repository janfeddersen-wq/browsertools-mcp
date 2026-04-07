//! McpResponse — response builder for MCP tool results.
//!
//! Assembles tool output with optional snapshot, network, console enrichment.

use crate::tools::definition::{ToolContent, ToolResult};

/// Builder for assembling MCP tool responses.
pub struct McpResponse {
    text_parts: Vec<String>,
    images: Vec<(String, String)>, // (base64_data, mime_type)
    include_snapshot: bool,
    include_network: bool,
    include_console: bool,
}

impl McpResponse {
    pub fn new() -> Self {
        Self {
            text_parts: Vec::new(),
            images: Vec::new(),
            include_snapshot: true,
            include_network: true,
            include_console: true,
        }
    }

    /// Add a text block to the response.
    pub fn add_text(&mut self, text: impl Into<String>) {
        self.text_parts.push(text.into());
    }

    /// Add an image (base64 encoded) to the response.
    pub fn add_image(&mut self, data: String, mime_type: String) {
        self.images.push((data, mime_type));
    }

    /// Control whether to include snapshot data.
    pub fn set_include_snapshot(&mut self, include: bool) {
        self.include_snapshot = include;
    }

    /// Control whether to include network data.
    pub fn set_include_network(&mut self, include: bool) {
        self.include_network = include;
    }

    /// Control whether to include console data.
    pub fn set_include_console(&mut self, include: bool) {
        self.include_console = include;
    }

    /// Build the final tool result.
    pub fn build(self) -> ToolResult {
        let mut content = Vec::new();

        // Combine all text parts.
        if !self.text_parts.is_empty() {
            content.push(ToolContent::Text {
                text: self.text_parts.join("\n"),
            });
        }

        // Add images.
        for (data, mime_type) in self.images {
            content.push(ToolContent::Image { data, mime_type });
        }

        if content.is_empty() {
            content.push(ToolContent::Text {
                text: "OK".to_string(),
            });
        }

        ToolResult {
            content,
            is_error: false,
        }
    }
}

impl Default for McpResponse {
    fn default() -> Self {
        Self::new()
    }
}
