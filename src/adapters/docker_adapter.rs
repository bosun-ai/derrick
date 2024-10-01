use async_trait::async_trait;
use bollard::container::{Config, RemoveContainerOptions};
use bollard::Docker;

use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use futures_util::future::poll_fn;
use futures_util::stream::StreamExt;
use futures_util::TryStreamExt;

use crate::Adapter;
use anyhow::Result;

static IMAGE: &str = "alpine:latest";

#[derive(Debug)]
pub struct DockerAdapter {
    path: String,
    docker: Option<Docker>,
    container_id: Option<String>,
}

#[async_trait]
impl Adapter for DockerAdapter {
    async fn init(&self) -> Result<()> {
        // Can also connect over http or tls
        let docker = Docker::connect_with_socket_defaults().unwrap();

        docker
            .create_image(
                Some(CreateImageOptions {
                    from_image: IMAGE,
                    ..Default::default()
                }),
                None,
                None,
            )
            .try_collect::<Vec<_>>()
            .await?;

        let alpine_config = Config {
            image: Some(IMAGE),
            tty: Some(true),
            ..Default::default()
        };

        let id = docker
            .create_container::<&str, &str>(None, alpine_config)
            .await?
            .id;

        docker.start_container::<String>(&id, None).await?;
        Ok(())
    }

    async fn cmd_with_output(&self, cmd: &str, working_dir: Option<&str>) -> Result<String> {
        // TODO: Working dir
        let mut response = String::new();
        let docker = self
            .docker
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Docker not initialized"))?;
        let container_id = self
            .container_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Container not initialized"))?;

        let exec = docker
            .create_exec(
                &container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(cmd.split_whitespace().map(String::from).collect()),
                    ..Default::default()
                },
            )
            .await?
            .id;

        if let StartExecResults::Attached { mut output, .. } =
            docker.start_exec(&exec, None).await?
        {
            while let Some(Ok(msg)) = output.next().await {
                response.push_str(&msg.to_string());
            }
        } else {
            // It's definately reachable
            unreachable!();
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

    fn path(&self, _working_dir: Option<&str>) -> String {
        panic!("This should never ever be called");
        self.path.clone()
    }
}

impl Drop for DockerAdapter {
    fn drop(&mut self) {
        let Some(container_id) = self.container_id.take() else {
            return;
        };
        let Some(docker) = self.docker.take() else {
            return;
        };

        let handle = tokio::runtime::Handle::current();
        let result = handle.block_on(async {
            docker
                .remove_container(
                    &container_id,
                    Some(RemoveContainerOptions {
                        force: true,
                        ..Default::default()
                    }),
                )
                .await
        });

        if let Err(e) = result {
            tracing::error!(error = %e, "Could not remove container");
        }
    }
}
