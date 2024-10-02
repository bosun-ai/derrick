use crate::adapters::Adapter;
use anyhow::{Context, Result};
// use async_nats::jetstream::response;
use crate::messaging;
use async_trait::async_trait;
use regex;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use std::sync::OnceLock;
use tracing::{debug, warn};

// Runs commands on a remote workspace using nats
#[derive(Debug)]
pub struct RemoteNatsAdapter {
    name: String,
    path: OnceLock<String>,
    channel: OnceLock<messaging::Channel>,
    subscriber: OnceLock<messaging::Subscriber>,
}

impl RemoteNatsAdapter {
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            path: OnceLock::new(),
            channel: OnceLock::new(),
            subscriber: OnceLock::new(),
        }
    }

    fn spawn_cmd(
        &self,
        cmd: &str,
        _working_dir: Option<&str>,
    ) -> std::result::Result<std::process::Output, std::io::Error> {
        debug!(cmd = scrub(cmd), "Running command");
        todo!()
    }

    async fn rpc_call<CmdType: Serialize, ResponseType: DeserializeOwned>(
        &self,
        cmd: CmdType,
    ) -> Result<ResponseType> {
        let channel = self
            .channel
            .get()
            .ok_or_else(|| anyhow::anyhow!("Channel not set"))?;

        let cmd_str = serde_json::to_string(&cmd).context("Could not serialize command")?;

        let response_str = channel
            .request(cmd_str)
            .await
            .context("Could not send request")?;

        let response =
            serde_json::from_str(&response_str).context("Could not deserialize response")?;

        Ok(response)
    }
}

#[async_trait]
impl Adapter for RemoteNatsAdapter {
    #[tracing::instrument]
    async fn init(&self) -> Result<()> {
        let channel = messaging::Channel::establish("workspace.init".to_string()).await?;

        self.channel
            .set(channel)
            .map_err(|_| anyhow::anyhow!("Channel already set"))?;

        Ok(())
    }

    #[tracing::instrument(fields(cmd = scrub(cmd)))]
    async fn cmd(&self, cmd: &str, working_dir: Option<&str>) -> Result<()> {
        self.spawn_cmd(cmd, working_dir)
            .map(handle_command_result)?
            .map(|_| ())
    }

    #[tracing::instrument(fields(cmd = scrub(cmd)))]
    async fn cmd_with_output(&self, cmd: &str, working_dir: Option<&str>) -> Result<String> {
        self.spawn_cmd(cmd, working_dir)
            .map(handle_command_result)?
    }

    #[tracing::instrument]
    async fn write_file(
        &self,
        file: &str,
        content: &str,
        _working_dir: Option<&str>,
    ) -> Result<()> {
        // std::fs::write(format!("{}/{}", &self.path(working_dir), file), content)
        //     .context("Could not write file")
        todo!()
    }

    #[tracing::instrument]
    async fn read_file(&self, file: &str, working_dir: Option<&str>) -> Result<String> {
        // std::fs::read_to_string(format!("{}/{}", &self.path(working_dir), file))
        //     .context("Could not read file")
        todo!()
    }
}

#[tracing::instrument]
fn handle_command_result(result: std::process::Output) -> Result<String> {
    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    if result.status.success() {
        debug!(stdout = &stdout, stderr = &stderr, "Command succeeded");
        Ok(stdout)
    } else {
        warn!(stdout = &stdout, stderr = &stderr, "Command failed");
        Err(anyhow::anyhow!(stderr))
    }
}

// scrub removes x-access-token:<token> from a string like x-access-token:1234@github.com
fn scrub(output: &str) -> String {
    let re = regex::Regex::new(r"x-access-token:[^@]+@").unwrap();
    re.replace_all(output, "x-access-token:***@").to_string()
}
