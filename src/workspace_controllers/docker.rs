use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use bollard::container::{Config, CreateContainerOptions, RemoveContainerOptions};
use bollard::Docker;

use bollard::exec::{CreateExecOptions, StartExecResults};
use futures_util::stream::StreamExt;
use tracing::debug;

use crate::workspace_controllers::{CommandOutput, WorkspaceController};
use anyhow::Result;

pub static BASE_IMAGE: &str = "bosunai/build-baseimage";

#[derive(Debug)]
pub struct DockerController {
    docker: Docker,
    pub container_id: String,
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

async fn stop_container(docker: &Docker, container_id: &str) -> Result<()> {
    docker
        .remove_container(
            container_id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

#[async_trait]
impl WorkspaceController for DockerController {
    async fn init(&self) -> Result<()> {
        // Can also connect over http or tls
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        stop_container(&self.docker, &self.container_id).await
    }

    async fn cmd_with_output(
        &self,
        cmd: &str,
        _working_dir: Option<&str>,
        env: HashMap<String, String>,
        timeout: Option<Duration>,
    ) -> Result<CommandOutput> {
        let env_strings: Vec<String> = env
            .into_iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();

        let timeout_str: String;
        let mut cmd_vec = Vec::with_capacity(5);

        if let Some(timeout) = timeout {
            timeout_str = timeout.as_secs().to_string();
            cmd_vec.push("timeout");
            cmd_vec.push(timeout_str.as_str());
        }
        cmd_vec.push("bash");
        cmd_vec.push("-c");
        cmd_vec.push(cmd);

        // TODO: Working dir
        let exec = self
            .docker
            .create_exec(
                &self.container_id,
                CreateExecOptions {
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    cmd: Some(cmd_vec),
                    env: Some(env_strings.iter().map(|s| s.as_str()).collect()),
                    ..Default::default()
                },
            )
            .await?;

        let mut response = String::new();

        if let StartExecResults::Attached { mut output, .. } =
            self.docker.start_exec(&exec.id, None).await?
        {
            while let Some(Ok(msg)) = output.next().await {
                response.push_str(&msg.to_string());
            }
        } else {
            todo!();
        }

        let exec_inspect = self.docker.inspect_exec(&exec.id).await?;
        let exit_code = exec_inspect.exit_code.unwrap_or(0) as i32;

        Ok(CommandOutput {
            output: response,
            exit_code,
        })
    }

    async fn cmd(
        &self,
        cmd: &str,
        working_dir: Option<&str>,
        env: HashMap<String, String>,
        timeout: Option<Duration>,
    ) -> Result<()> {
        let result = self.cmd_with_output(cmd, working_dir, env, timeout).await?;
        if result.exit_code == 0 {
            Ok(())
        } else {
            Err(anyhow::anyhow!(
                "Command failed with exit code {}: {}",
                result.exit_code,
                result.output
            ))
        }
    }

    async fn write_file(&self, path: &str, content: &str, working_dir: Option<&str>) -> Result<()> {
        self.cmd(
            &format!("echo {} > {}", content, path),
            working_dir,
            HashMap::new(),
            None,
        )
        .await?;
        Ok(())
    }

    async fn read_file(&self, path: &str, working_dir: Option<&str>) -> Result<String> {
        self.cmd_with_output(&format!("cat {}", path), working_dir, HashMap::new(), None)
            .await
            .map(|output| output.output)
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
                    None,
                )
                .await?;
            let has_repository = repository_listing.output.contains("config");
            debug!(
                "Has repository: {}, {:?}",
                has_repository, repository_listing
            );
            if !has_repository {
                debug!("Cloning repository: {}", repository.url);
                self.cmd(
                    &format!("mkdir -p {}", repository.path),
                    None,
                    HashMap::new(),
                    None,
                )
                .await?;
                self.cmd(
                    &format!("git clone {} {}", repository.url, repository.path),
                    None,
                    HashMap::new(),
                    None,
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
                    None,
                )
                .await?;
                self.cmd(
                    &format!("cd {} && git pull origin master", repository.path),
                    None,
                    HashMap::new(),
                    None,
                )
                .await?;
            }
            // remove the remote origin so that we don't leak the access token
            self.cmd(
                &format!("cd {} && git remote remove origin", repository.path),
                None,
                HashMap::new(),
                None,
            )
            .await?;
        }
        Ok(())
    }
}

impl Drop for DockerController {
    fn drop(&mut self) {
        let handle = tokio::runtime::Handle::current();
        let docker = self.docker.clone();
        let container_id = self.container_id.clone();
        handle.spawn(async move { stop_container(&docker, &container_id).await });
    }
}
