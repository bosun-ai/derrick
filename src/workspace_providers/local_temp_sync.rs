use std::collections::HashMap;

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
    async fn provision(
        &mut self,
        context: &WorkspaceContext,
        env: HashMap<String, String>,
    ) -> Result<Box<dyn WorkspaceController>> {
        let controller = Box::new(LocalTempSyncController::initialize(&context.name).await);
        controller.init().await?;
        for repository in &context.repositories {
            controller
                .provision_repositories(vec![repository.clone()])
                .await?;
        }

        controller
            .cmd_with_output(context.setup_script.as_str(), Some("/"), env)
            .await?;

        Ok(controller)
    }
}
