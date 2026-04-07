//! Chrome extension metadata registry.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Metadata about an installed extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledExtension {
    pub id: String,
    pub name: String,
    pub version: String,
    pub path: PathBuf,
}

/// Registry tracking installed extensions.
#[derive(Debug, Default)]
pub struct ExtensionRegistry {
    extensions: HashMap<String, InstalledExtension>,
}

impl ExtensionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a newly installed extension by reading its manifest.json.
    pub async fn register(&mut self, id: String, path: PathBuf) -> Result<()> {
        let manifest_path = path.join("manifest.json");
        let manifest_data = tokio::fs::read_to_string(&manifest_path).await?;
        let manifest: serde_json::Value = serde_json::from_str(&manifest_data)?;

        let name = manifest
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let version = manifest
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.0.0")
            .to_string();

        self.extensions.insert(
            id.clone(),
            InstalledExtension {
                id,
                name,
                version,
                path,
            },
        );

        Ok(())
    }

    /// Remove an extension from the registry.
    pub fn remove(&mut self, id: &str) {
        self.extensions.remove(id);
    }

    /// List all registered extensions.
    pub fn list(&self) -> Vec<&InstalledExtension> {
        self.extensions.values().collect()
    }

    /// Get an extension by ID.
    pub fn get(&self, id: &str) -> Option<&InstalledExtension> {
        self.extensions.get(id)
    }
}
