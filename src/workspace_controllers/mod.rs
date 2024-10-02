use anyhow::Result;
use async_trait::async_trait;

mod local_sync;
pub use local_sync::LocalTempSyncController;

mod testing;
pub use testing::TestingController;

mod remote_nats;
pub use remote_nats::RemoteNatsController;
mod docker;
pub use docker::DockerController;

#[async_trait]
pub trait WorkspaceController: Send + Sync + std::fmt::Debug {
    async fn init(&self) -> Result<()>;
    async fn cmd(&self, cmd: &str, working_dir: Option<&str>) -> Result<()>;
    async fn cmd_with_output(&self, cmd: &str, working_dir: Option<&str>) -> Result<String>;
    async fn write_file(&self, path: &str, content: &str, working_dir: Option<&str>) -> Result<()>;
    async fn read_file(&self, path: &str, working_dir: Option<&str>) -> Result<String>;
}
