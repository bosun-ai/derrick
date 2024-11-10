use async_trait::async_trait;

use anyhow::Result;
use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::TryStreamExt;

use crate::WorkspaceController;
use tracing::debug;

use crate::workspace_controllers::docker::BASE_IMAGE;
use crate::workspace_controllers::DockerController;

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
    pub async fn initialize(base_image: Option<&str>) -> Result<DockerProvider> {
        let docker = crate::docker::establish_connection().await?;

        let base_image: &str = base_image.unwrap_or(BASE_IMAGE);
        Self::create_base_image(&docker, base_image)
            .await
            .expect("Could not create base image");

        let provider = DockerProvider {
            docker,
            base_image: base_image.to_string(),
        };
        Ok(provider)
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
    async fn provision(
        &mut self,
        context: &WorkspaceContext,
    ) -> Result<Box<dyn WorkspaceController>> {
        let controller =
            DockerController::start(&self.docker, &self.base_image, &context.name).await?;
        controller
            .provision_repositories(context.repositories.clone())
            .await?;

        // TODO we should cache the docker image after the setup script has run so subsequent
        // provisioning is faster
        controller
            .cmd_with_output(context.setup_script.as_str(), Some("/"))
            .await?;

        Ok(Box::new(controller))
    }
}
