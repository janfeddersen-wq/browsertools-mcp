//! Tool category definitions, mirroring the TypeScript ToolCategory enum.

use serde::{Deserialize, Serialize};

/// Categories for grouping and filtering MCP tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    Input,
    Navigation,
    Emulation,
    Performance,
    Network,
    Debugging,
    Extensions,
    InPage,
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Input => write!(f, "input"),
            Self::Navigation => write!(f, "navigation"),
            Self::Emulation => write!(f, "emulation"),
            Self::Performance => write!(f, "performance"),
            Self::Network => write!(f, "network"),
            Self::Debugging => write!(f, "debugging"),
            Self::Extensions => write!(f, "extensions"),
            Self::InPage => write!(f, "in_page"),
        }
    }
}
