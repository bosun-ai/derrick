use async_nats::rustls::internal::msgs::base;
use async_trait::async_trait;

use anyhow::Result;
use bollard::Docker;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
use bollard::image::CreateImageOptions;
use bollard::exec::{CreateExecOptions, StartExecResults};
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;

use crate::WorkspaceController;
use tracing::{debug, info};

use crate::{workspace_controllers::DockerController};
use crate::workspace_controllers::docker::UBUNTU_IMAGE;

use super::{WorkspaceContext, WorkspaceProvider};

pub struct DockerProvider {
    docker: Docker,
    base_image: String,
}

// We want to be able to quickly provision a workspace. There are time consuming steps:
// 1. Creating the container
// 2. Downloading the code
// 3. Downloading the dependencies of the code
// 4. Building the (dependencies of the) code
//
// To speed up the process, we should set up a cache. We can do this by making a snapshot of the
// state of the container after running the setup script. We can then use this snapshot to quickly
// provision new workspaces based off the snapshot.
//
// The snapshot should be stored in a Docker image. We can use the `docker commit` command to create
// a new image from a container. We can then use this image to create new containers.
//
impl DockerProvider {
    pub async fn initialize() -> DockerProvider {
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

        let base_image: &str = UBUNTU_IMAGE;
        Self::create_base_image(&docker, base_image).await.expect("Could not create base image");

        let provider = DockerProvider { docker, base_image: base_image.to_string() };
        provider
    }

    pub async fn create_base_image(docker: &Docker, base_image: &str) -> Result<()> {
        debug!("Creating container with image: {}", base_image);

        docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: base_image,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await?;
        Ok(())
    }
}

#[async_trait]
impl WorkspaceProvider for DockerProvider {
    async fn provision(&mut self, context: &WorkspaceContext) -> Result<Box<dyn WorkspaceController>> {
        let controller = DockerController::start(&self.docker, &self.base_image, &context.name).await?;
        Ok(Box::new(controller))
    }
}
