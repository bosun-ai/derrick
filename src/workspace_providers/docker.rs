use async_trait::async_trait;

use anyhow::Result;
use bollard::Docker;
use tracing::{info, trace};

use crate::{workspace_controllers::DockerController, WorkspaceController};

use super::{WorkspaceContext, WorkspaceProvider};

pub struct DockerProvider {
    docker: Docker
}

impl DockerProvider {
    pub fn new() -> DockerProvider {
        let docker = Docker::connect_with_socket_defaults().unwrap();
        DockerProvider {
            docker
        }
    }
}

#[async_trait]
impl WorkspaceProvider for DockerProvider {
    async fn provision(&self, context: &WorkspaceContext) -> Result<Box<dyn WorkspaceController>> {
        info!("Provisioning workspace with DockerProvider");
        let controller = DockerController::start(&self.docker, &context.name).await?;
        Ok(Box::new(controller))
    }
}
