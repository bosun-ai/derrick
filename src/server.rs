use std::collections::HashMap;

use crate::{WorkspaceContext, WorkspaceController, WorkspaceProvider};
use anyhow::Result;
use tracing::info;

pub struct Server {
    context: WorkspaceContext,
    provider: Box<dyn WorkspaceProvider>,
    workspaces: HashMap<String, Box<dyn WorkspaceController>>,
}

impl Server {
    pub fn create_server(
        context: WorkspaceContext,
        provider: Box<dyn WorkspaceProvider>,
    ) -> Result<Server> {
        Ok(Server {
            context,
            provider,
            workspaces: HashMap::new(),
        })
    }

    //
    // HTTP Server endpoints:
    // POST /workspaces                                 creates a new workspace
    // DELETE /workspaces/:workspace_id                 destroys a workspace
    // GET /workspaces                                  lists existing workspaces
    //
    // Workspace actions
    // POST /workspaces/:workspace_id/cmd               runs a command in the workspace
    // POST /workspaces/:workspace_id/cmd_with_output   runs a command in the workspace and returns the output
    // POST /workspaces/:workspace_id/write_file        writes a file in the workspace
    // POST /workspaces/:workspace_id/read_file         reads a file in the workspace

    pub async fn create_workspace(&mut self) -> Result<String> {
        info!("Creating workspace");
        let controller = self.provider.provision(&self.context).await?;
        let id: String = uuid::Uuid::new_v4().to_string();
        controller.init().await?;
        self.workspaces.insert(id.clone(), controller);
        Ok(id)
    }

    pub async fn destroy_workspace(&mut self, id: &str) -> Result<bool> {
        match self.workspaces.get(id) {
            Some(controller) => {
                controller.stop().await?;
                self.workspaces.remove(id);
                Ok(true)
            }
            None => Ok(false),
        }
    }

    // TODO implement showable workspace type
    pub async fn list_workspaces(&self) -> Result<Vec<String>> {
        Ok(self.workspaces.keys().cloned().collect())
    }

    pub async fn cmd(&self, id: &str, cmd: &str, working_dir: Option<&str>) -> Result<()> {
        match self.workspaces.get(id) {
            Some(controller) => controller.cmd(cmd, working_dir).await,
            None => Err(anyhow::anyhow!("Workspace not found: {}", id)),
        }
    }

    pub async fn cmd_with_output(
        &self,
        id: &str,
        cmd: &str,
        working_dir: Option<&str>,
    ) -> Result<String> {
        match self.workspaces.get(id) {
            Some(controller) => controller.cmd_with_output(cmd, working_dir).await,
            None => Err(anyhow::anyhow!("Workspace not found: {}", id)),
        }
    }

    pub async fn write_file(
        &self,
        id: &str,
        path: &str,
        content: &str,
        working_dir: Option<&str>,
    ) -> Result<()> {
        match self.workspaces.get(id) {
            Some(controller) => controller.write_file(path, content, working_dir).await,
            None => Err(anyhow::anyhow!("Workspace not found: {}", id)),
        }
    }

    pub async fn read_file(
        &self,
        id: &str,
        path: &str,
        working_dir: Option<&str>,
    ) -> Result<String> {
        match self.workspaces.get(id) {
            Some(controller) => controller.read_file(path, working_dir).await,
            None => Err(anyhow::anyhow!("Workspace not found: {}", id)),
        }
    }

    pub async fn workspace_cmd(
        &self,
        id: &str,
        cmd: &str,
        working_dir: Option<&str>,
    ) -> Result<()> {
        match self.workspaces.get(id) {
            Some(controller) => controller.cmd(cmd, working_dir).await,
            None => Err(anyhow::anyhow!("Workspace not found: {}", id)),
        }
    }

    pub async fn workspace_cmd_with_output(
        &self,
        id: &str,
        cmd: &str,
        working_dir: Option<&str>,
    ) -> Result<String> {
        match self.workspaces.get(id) {
            Some(controller) => controller.cmd_with_output(cmd, working_dir).await,
            None => Err(anyhow::anyhow!("Workspace not found: {}", id)),
        }
    }

    pub async fn workspace_write_file(
        &self,
        id: &str,
        path: &str,
        content: &str,
        working_dir: Option<&str>,
    ) -> Result<()> {
        match self.workspaces.get(id) {
            Some(controller) => controller.write_file(path, content, working_dir).await,
            None => Err(anyhow::anyhow!("Workspace not found: {}", id)),
        }
    }

    pub async fn workspace_read_file(
        &self,
        id: &str,
        path: &str,
        working_dir: Option<&str>,
    ) -> Result<String> {
        match self.workspaces.get(id) {
            Some(controller) => controller.read_file(path, working_dir).await,
            None => Err(anyhow::anyhow!("Workspace not found: {}", id)),
        }
    }
}
