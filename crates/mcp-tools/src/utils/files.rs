//! Temporary file utilities.

use std::path::PathBuf;

use anyhow::Result;

/// Save data to a temporary file and return its path.
pub async fn save_temporary_file(data: &[u8], filename: &str) -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("browsertools-mcp");
    tokio::fs::create_dir_all(&dir).await?;

    let path = dir.join(filename);
    tokio::fs::write(&path, data).await?;

    Ok(path)
}

/// Save data to a specific file path, creating parent directories.
pub async fn save_file(data: &[u8], path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, data).await?;
    Ok(path)
}
