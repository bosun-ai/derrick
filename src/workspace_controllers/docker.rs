use async_trait::async_trait;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
use bollard::Docker;

use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;
use tracing::debug;

use crate::WorkspaceController;
use anyhow::Result;

static ALPINE_IMAGE: &str = "alpine:3.20";
static UBUNTU_IMAGE: &str = "ubuntu:noble";

#[derive(Debug)]
pub struct DockerController {
    docker: Docker,
    container_id: String,
}

impl DockerController {
    pub async fn start(docker: &Docker, name: &str) -> Result<Self> {
        let name = format!("{}-{}", name, uuid::Uuid::new_v4());
        let base_image = UBUNTU_IMAGE;

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

        let container_config = Config {
            image: Some(base_image),
            tty: Some(true),
            ..Default::default()
        };

        let container_options = Some(CreateContainerOptions {
            name: name.as_str(),
            platform: None,
        });

        let id = docker
            .create_container::<&str, &str>(container_options, container_config)
            .await?
            .id;

        debug!("Starting container with name: {}", id);

        docker.start_container::<String>(&id, None).await?;

        Ok(Self {
            docker: docker.clone(),
            container_id: id,
        })
    }
}

#[async_trait]
impl WorkspaceController for DockerController {
    async fn init(&self) -> Result<()> {
        // Can also connect over http or tls
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.docker
            .remove_container(
                &self.container_id,
                Some(RemoveContainerOptions {
                    force: true,
                    ..Default::default()
                }),
            )
            .await?;
        Ok(())
    }

    async fn cmd_with_output(&self, cmd: &str, working_dir: Option<&str>) -> Result<String> {
        // TODO: Working dir
        let exec = self
            .docker
            .create_exec(
                &self.container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(cmd.split_whitespace().map(String::from).collect()),
                    ..Default::default()
                },
            )
            .await?
            .id;

        let mut response = String::new();

        if let StartExecResults::Attached { mut output, .. } =
            self.docker.start_exec(&exec, None).await?
        {
            while let Some(Ok(msg)) = output.next().await {
                response.push_str(&msg.to_string());
            }
        } else {
            todo!();
        }
        Ok(response)
    }

    async fn cmd(&self, cmd: &str, working_dir: Option<&str>) -> Result<()> {
        self.cmd_with_output(cmd, working_dir).await?;
        Ok(())
    }

    async fn write_file(&self, path: &str, content: &str, working_dir: Option<&str>) -> Result<()> {
        self.cmd(&format!("echo {} > {}", content, path), working_dir)
            .await?;
        Ok(())
    }

    async fn read_file(&self, path: &str, working_dir: Option<&str>) -> Result<String> {
        self.cmd_with_output(&format!("cat {}", path), working_dir)
            .await
    }

    async fn provision_repositories(
        &self,
        _repositories: Vec<crate::repository::Repository>,
    ) -> Result<()> {
        Ok(())
    }
}

impl Drop for DockerController {
    fn drop(&mut self) {
        let handle = tokio::runtime::Handle::current();
        let result = handle.block_on(async { self.stop().await });

        if let Err(e) = result {
            tracing::error!(error = %e, "Could not remove container");
        }
    }
}
