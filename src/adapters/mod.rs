use anyhow::Result;
use async_trait::async_trait;

mod local_sync_adapter;
pub use local_sync_adapter::LocalTempSync;

mod testing_adapter;
pub use testing_adapter::TestingAdapter;

mod remote_nats_adapter;
pub use remote_nats_adapter::RemoteNatsAdapter;
mod docker_adapter;
pub use docker_adapter::DockerAdapter;

#[async_trait]
pub trait Adapter: Send + Sync + std::fmt::Debug {
    async fn init(&self) -> Result<()>;
    async fn cmd(&self, cmd: &str, working_dir: Option<&str>) -> Result<()>;
    async fn cmd_with_output(&self, cmd: &str, working_dir: Option<&str>) -> Result<String>;
    async fn write_file(&self, path: &str, content: &str, working_dir: Option<&str>) -> Result<()>;
    async fn read_file(&self, path: &str, working_dir: Option<&str>) -> Result<String>;
}
