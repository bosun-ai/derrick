use std::collections::HashMap;

use async_trait::async_trait;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
use bollard::Docker;

use bollard::exec::{CreateExecOptions, StartExecResults};
use futures_util::stream::StreamExt;
use tracing::debug;

use crate::WorkspaceController;
use anyhow::Result;

pub static BASE_IMAGE: &str = "bosunai/build-baseimage";

#[derive(Debug)]
pub struct DockerController {
    docker: Docker,
    container_id: String,
}

impl DockerController {
    pub async fn start(docker: &Docker, base_image: &str, name: &str) -> Result<Self> {
        let name = format!("{}-{}", name, uuid::Uuid::new_v4());

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

        debug!("Starting container with name: {} and id {}", name, id);

        docker.start_container::<String>(&id, None).await?;

        Ok(Self {
            docker: docker.clone(),
            container_id: id,
        })
    }

    pub async fn start_with_mounts(
        docker: &Docker,
        base_image: &str,
        name: &str,
        mounts: Vec<(&str, &str)>,
    ) -> Result<Self> {
        let name = format!("{}-{}", name, uuid::Uuid::new_v4());

        let container_config = Config {
            image: Some(base_image),
            tty: Some(true),
            host_config: Some(bollard::models::HostConfig {
                binds: Some(
                    mounts
                        .iter()
                        .map(|(host, container)| format!("{}:{}", host, container))
                        .collect(),
                ),
                ..Default::default()
            }),
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

        debug!("Starting container with name: {} and id {}", name, id);

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

    async fn cmd_with_output(
        &self,
        cmd: &str,
        _working_dir: Option<&str>,
        env: HashMap<String, String>,
    ) -> Result<String> {
        let env_strings: Vec<String> = env
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        // TODO: Working dir
        let exec = self
            .docker
            .create_exec(
                &self.container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(vec!["sh", "-c", cmd]),
                    env: Some(env_strings.iter().map(|s| s.as_str()).collect()),
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

    async fn cmd(
        &self,
        cmd: &str,
        working_dir: Option<&str>,
        env: HashMap<String, String>,
    ) -> Result<()> {
        self.cmd_with_output(cmd, working_dir, env).await?;
        Ok(())
    }

    async fn write_file(&self, path: &str, content: &str, working_dir: Option<&str>) -> Result<()> {
        self.cmd(
            &format!("echo {} > {}", content, path),
            working_dir,
            HashMap::new(),
        )
        .await?;
        Ok(())
    }

    async fn read_file(&self, path: &str, working_dir: Option<&str>) -> Result<String> {
        self.cmd_with_output(&format!("cat {}", path), working_dir, HashMap::new())
            .await
    }

    async fn provision_repositories(
        &self,
        repositories: Vec<crate::repository::Repository>,
    ) -> Result<()> {
        for repository in repositories {
            // if the repository does not yet exist, we clone it
            debug!("Provisioning repository: {}", repository.url);
            let repository_listing = self
                .cmd_with_output(
                    &format!("ls {}/.git", repository.path),
                    None,
                    HashMap::new(),
                )
                .await?;
            let has_repository = repository_listing.contains("config");
            debug!("Has repository: {}, {}", has_repository, repository_listing);
            if !has_repository {
                debug!("Cloning repository: {}", repository.url);
                self.cmd(
                    &format!("mkdir -p {}", repository.path),
                    None,
                    HashMap::new(),
                )
                .await?;
                self.cmd(
                    &format!("git clone {} {}", repository.url, repository.path),
                    None,
                    HashMap::new(),
                )
                .await?;
            } else {
                debug!("Pulling latest changes for repository: {}", repository.url);
                // if the repository exists, we pull the latest changes, but first we add back the remote origin
                self.cmd(
                    &format!(
                        "cd {} && git remote add origin {}",
                        repository.path, repository.url
                    ),
                    None,
                    HashMap::new(),
                )
                .await?;
                self.cmd(
                    &format!("cd {} && git pull origin master", repository.path),
                    None,
                    HashMap::new(),
                )
                .await?;
            }
            // remove the remote origin so that we don't leak the access token
            self.cmd(
                &format!("cd {} && git remote remove origin", repository.path),
                None,
                HashMap::new(),
            )
            .await?;
        }
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
