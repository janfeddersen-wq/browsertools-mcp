//! # mcp-tools
//!
//! MCP tool definitions and handlers for BrowserTools.
//!
//! Provides the McpContext (central state), tool definitions for all 29 tools,
//! response builders, event collectors, and output formatters.

pub mod collector;
pub mod context;
pub mod formatters;
pub mod page_state;
pub mod response;
pub mod server;
pub mod slim_server;
pub mod tools;
pub mod utils;
pub mod wait_for;

pub use context::{McpContext, SharedContext};
pub use page_state::McpPageState;
pub use response::McpResponse;
pub use server::BrowserToolsServer;
pub use slim_server::SlimServer;
