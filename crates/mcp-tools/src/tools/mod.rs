//! MCP tool registry — aggregates all tool modules.

pub mod categories;
pub mod console;
pub mod definition;
pub mod emulation;
pub mod extensions;
pub mod in_page;
pub mod input;
pub mod lighthouse;
pub mod memory;
pub mod network;
pub mod pages;
pub mod performance;
pub mod screencast;
pub mod screenshot;
pub mod script;
pub mod slim;
pub mod snapshot;

pub use categories::ToolCategory;
pub use definition::{ToolAnnotations, ToolContent, ToolDefinition, ToolResult};
