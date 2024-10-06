use async_trait::async_trait;

use anyhow::Result;
use bollard::Docker;
use tracing::info;

use crate::{workspace_controllers::DockerController, WorkspaceController};

use super::{WorkspaceContext, WorkspaceProvider};

pub struct DockerProvider {
    docker: Docker,
}

impl DockerProvider {
    pub fn new() -> DockerProvider {
        // if windows or linux we connect with socket defaults
        let docker = if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
            Docker::connect_with_socket_defaults().expect("Could not create Docker client")
        } else if cfg!(target_os = "macos") {
            let username = whoami::username();
            let macos_socket_path = format!("unix:///Users/{}/.docker/run/docker.sock", username);
            Docker::connect_with_socket(&macos_socket_path, 5, bollard::API_DEFAULT_VERSION)
                .expect("Could not create Docker client")
        } else {
            panic!("Unsupported OS")
        };
        // Test the connection
        DockerProvider { docker }
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
