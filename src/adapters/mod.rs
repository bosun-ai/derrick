use anyhow::Result;
use async_trait::async_trait;
use core::fmt::Debug;

mod local_sync_adapter;
pub use local_sync_adapter::LocalTempSync;

mod testing_adapter;
pub use testing_adapter::TestingAdapter;

#[async_trait]
pub trait Adapter: Send + Sync {
    fn init(&self) -> Result<()>;
    fn cmd(&self, cmd: &str) -> Result<()>;
    fn cmd_with_output(&self, cmd: &str) -> Result<String>;
    fn write_file(&self, path: &str, content: &str) -> Result<()>;
    fn debug(&self) -> String;
}

impl Debug for dyn Adapter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Adapter{{{}}}", self.debug())
    }
}
