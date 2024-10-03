mod local_temp_sync;
pub use local_temp_sync::LocalTempSyncProvider;

use anyhow::Result;
use crate::{repository::Repository, WorkspaceController};

#[derive(Debug, Clone)]
pub struct WorkspaceContext {
    pub name: String, // Unique name for the workspace (for inspection/debugging)
    pub repositories: Vec<Repository>,
    pub setup_script: String,
}

pub trait WorkspaceProvider {
    fn provision(&self, context: WorkspaceContext) -> Result<Box<dyn WorkspaceController>>;
}

pub async fn get_provider(provisioning_mode: String) -> Result<Box<dyn WorkspaceProvider>> {
    match provisioning_mode.as_str() {
        "local" => Ok(Box::new(LocalTempSyncProvider::new())),
        _ => return Err(anyhow::anyhow!("Unsupported provisioning mode: {}", provisioning_mode)),
    }
}