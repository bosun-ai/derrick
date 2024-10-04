mod local_temp_sync;
pub use local_temp_sync::LocalTempSyncProvider;

use crate::{repository::Repository, WorkspaceController};
use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceContext {
    pub name: String, // Unique name for the workspace (for inspection/debugging)
    pub repositories: Vec<Repository>,
    pub setup_script: String,
}

impl WorkspaceContext {
    pub fn from_file(path: String) -> Result<WorkspaceContext> {
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        let context = serde_json::from_reader(reader)?;
        Ok(context)
    }
}

pub trait WorkspaceProvider: Send + Sync {
    fn provision(&self, context: &WorkspaceContext) -> Result<Box<dyn WorkspaceController>>;
}

pub async fn get_provider(provisioning_mode: String) -> Result<Box<dyn WorkspaceProvider>> {
    match provisioning_mode.as_str() {
        "local" => Ok(Box::new(LocalTempSyncProvider::new())),
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported provisioning mode: {}",
                provisioning_mode
            ))
        }
    }
}
