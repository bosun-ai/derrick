use std::collections::HashMap;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;

#[derive(Debug)]
pub struct CommandOutput {
    pub output: String,
    pub exit_code: i32,
}

mod local_temp_sync;
pub use local_temp_sync::LocalTempSyncController;

#[cfg(test)]
mod testing;

pub mod docker;
// mod remote_nats;
pub use docker::DockerController;

#[async_trait]
pub trait WorkspaceController: Send + Sync + std::fmt::Debug {
    async fn init(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn provision_repositories(
        &self,
        repositories: Vec<crate::repository::Repository>,
    ) -> Result<()>;
    async fn cmd(
        &self,
        cmd: &str,
        working_dir: Option<&str>,
        env: HashMap<String, String>,
        timeout: Option<Duration>,
    ) -> Result<()>;
    async fn cmd_with_output(
        &self,
        cmd: &str,
        working_dir: Option<&str>,
        env: HashMap<String, String>,
        timeout: Option<Duration>,
    ) -> Result<CommandOutput>;
    async fn write_file(&self, path: &str, content: &[u8], working_dir: Option<&str>)
        -> Result<()>;
    async fn read_file(&self, path: &str, working_dir: Option<&str>) -> Result<Vec<u8>>;
}
