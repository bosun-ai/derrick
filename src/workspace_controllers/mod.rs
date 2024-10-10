use anyhow::Result;
use async_trait::async_trait;

mod local_temp_sync;
pub use local_temp_sync::LocalTempSyncController;

mod testing;

pub mod docker;
mod remote_nats;
pub use docker::DockerController;

#[async_trait]
pub trait WorkspaceController: Send + Sync + std::fmt::Debug {
    async fn init(&self) -> Result<()>;
    async fn stop(&self) -> Result<()>;
    async fn provision_repositories(
        &self,
        repositories: Vec<crate::repository::Repository>,
    ) -> Result<()>;
    async fn cmd(&self, cmd: &str, working_dir: Option<&str>) -> Result<()>;
    // TODO instead of returning a string, return a stream of output (using tokio::sync)
    async fn cmd_with_output(&self, cmd: &str, working_dir: Option<&str>) -> Result<String>;
    async fn write_file(&self, path: &str, content: &str, working_dir: Option<&str>) -> Result<()>;
    async fn read_file(&self, path: &str, working_dir: Option<&str>) -> Result<String>;
}
