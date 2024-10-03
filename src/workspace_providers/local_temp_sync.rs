use anyhow::Result;

use crate::{workspace_controllers::LocalTempSyncController, WorkspaceController};

use super::{WorkspaceContext, WorkspaceProvider};

pub struct LocalTempSyncProvider {
}

impl LocalTempSyncProvider {
    pub fn new() -> LocalTempSyncProvider {
        LocalTempSyncProvider {
        }
    }
}

impl WorkspaceProvider for LocalTempSyncProvider {
    fn provision(&self, context: WorkspaceContext) -> Result<Box<dyn WorkspaceController>> {
        let controller = Box::new(LocalTempSyncController::new(&context.name));
        Ok(controller)
    }
}