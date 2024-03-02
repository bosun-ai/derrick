use anyhow::Result;
use async_trait::async_trait;

mod local_sync_adapter;
pub use local_sync_adapter::LocalTempSync;

mod testing_adapter;
pub use testing_adapter::TestingAdapter;

#[async_trait]
pub trait Adapter: Send + Sync + std::fmt::Debug {
    fn init(&self) -> Result<()>;
    fn cmd(&self, cmd: &str, working_dir: Option<&str>) -> Result<()>;
    fn cmd_with_output(&self, cmd: &str, working_dir: Option<&str>) -> Result<String>;
    fn write_file(&self, path: &str, content: &str, working_dir: Option<&str>) -> Result<()>;
    fn read_file(&self, path: &str, working_dir: Option<&str>) -> Result<String>;
}
