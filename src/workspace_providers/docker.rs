use std::collections::HashMap;

use async_trait::async_trait;

use anyhow::Result;
use bollard::image::{CommitContainerOptions, CreateImageOptions};
use bollard::Docker;
use futures_util::TryStreamExt;

use crate::{Repository, WorkspaceController};
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

    pub async fn prepare_base_image_repositories(
        &self,
        repositories: Vec<Repository>,
    ) -> Result<String> {
        let repositories_hash = repositories_hash(&repositories);
        let image_name = format!(
            "{}-cache-{}",
            self.base_image.replace("/", "-"),
            repositories_hash
        );

        if !self.docker.inspect_image(&image_name).await.is_ok() {
            tracing::info!("Creating base image with repositories: {}", image_name);
            let controller =
                DockerController::start(&self.docker, &self.base_image, &image_name).await?;
            controller.provision_repositories(repositories).await?;

            self.docker
                .commit_container(
                    CommitContainerOptions {
                        container: controller.container_id.clone(),
                        repo: image_name.clone(),
                        ..Default::default()
                    },
                    bollard::container::Config::<String>::default(),
                )
                .await?;

            controller.stop().await?;
        } else {
            tracing::info!(
                "Base image with repositories already exists: {}",
                image_name
            );
        }

        Ok(image_name)
    }

    pub async fn prepare_image(
        &self,
        context: &WorkspaceContext,
        env: HashMap<String, String>,
    ) -> Result<String> {
        let context_hash = context_hash(context, &env);
        let image_name = format!(
            "{}-{}-cache-{}",
            context.name,
            self.base_image.replace("/", "-"),
            context_hash
        );

        if !self.docker.inspect_image(&image_name).await.is_ok() {
            tracing::info!("Creating image with context: {}", image_name);
            let base_image = self
                .prepare_base_image_repositories(context.repositories.clone())
                .await?;

            let controller =
                DockerController::start(&self.docker, &base_image, &context.name).await?;

            controller
                .write_file("/tmp/setup.sh", context.setup_script.as_bytes(), None)
                .await?;
            controller
                .cmd_with_output("chmod +x /tmp/setup.sh", Some("/"), env.clone(), None)
                .await?;

            debug!("Running setup script: {}", context.setup_script);
            let output = controller
                .cmd_with_output("/tmp/setup.sh", Some("/"), env, None)
                .await?;

            if output.exit_code != 0 {
                return Err(anyhow::anyhow!("Setup script failed: {:?}", output));
            } else {
                debug!("Setup script succeeded");
            }

            self.docker
                .commit_container(
                    CommitContainerOptions {
                        container: controller.container_id.clone(),
                        repo: image_name.clone(),

                        ..Default::default()
                    },
                    bollard::container::Config::<String>::default(),
                )
                .await?;

            controller.stop().await?;
        } else {
            tracing::info!("Image with context already exists: {}", image_name);
        }

        Ok(image_name)
    }
}

fn repositories_hash(repositories: &Vec<Repository>) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    repositories.iter().for_each(|repo| {
        hasher.update(repo.url.as_str());
        hasher.update(repo.path.as_str());
        if let Some(reference) = repo.reference.clone() {
            hasher.update(reference.as_str());
        }
    });
    let mut result = hex::encode(hasher.finalize());
    result.truncate(16);
    result
}

fn context_hash(context: &WorkspaceContext, env: &HashMap<String, String>) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(context.name.as_str());
    context.repositories.iter().for_each(|repo| {
        hasher.update(repo.url.as_str());
        hasher.update(repo.path.as_str());
        if let Some(reference) = repo.reference.clone() {
            hasher.update(reference.as_str());
        }
    });
    hasher.update(context.setup_script.as_str());
    env.iter().for_each(|(key, value)| {
        hasher.update(key.as_str());
        hasher.update(value.as_str());
    });
    let mut result = hex::encode(hasher.finalize());
    result.truncate(16);
    result
}

#[async_trait]
impl WorkspaceProvider for DockerProvider {
    async fn provision(
        &mut self,
        context: &WorkspaceContext,
        env: HashMap<String, String>,
    ) -> Result<Box<dyn WorkspaceController>> {
        let image_name = self.prepare_image(context, env).await?;
        let controller = DockerController::start(&self.docker, &image_name, &context.name).await?;
        Ok(Box::new(controller))
    }
}
