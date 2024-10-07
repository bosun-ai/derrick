use anyhow::Result;
use async_trait::async_trait;

use crate::{workspace_controllers::LocalTempSyncController, WorkspaceController};

use super::{WorkspaceContext, WorkspaceProvider};

pub struct LocalTempSyncProvider {}

impl LocalTempSyncProvider {
    pub fn new() -> LocalTempSyncProvider {
        LocalTempSyncProvider {}
    }
}

#[async_trait]
impl WorkspaceProvider for LocalTempSyncProvider {
    async fn provision(&mut self, context: &WorkspaceContext) -> Result<Box<dyn WorkspaceController>> {
        let controller = Box::new(LocalTempSyncController::initialize(&context.name).await);
        controller.init().await?;
        for repository in &context.repositories {
            controller.provision_repositories(vec![repository.clone()]).await?;
        }
        Ok(controller)
    }
}
