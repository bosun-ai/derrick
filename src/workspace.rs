use crate::adapters::Adapter;
use crate::Codebase;
use anyhow::Result;
use std::fmt::Debug;
use tracing::info;

#[derive(Debug)]
pub struct Workspace {
    adapter: Box<dyn Adapter>,
    pub codebase: Codebase,
}

impl Workspace {
    #[tracing::instrument]
    pub fn new(adapter: Box<dyn Adapter>, codebase: &Codebase) -> Self {
        Self {
            adapter,
            codebase: codebase.to_owned(),
        }
    }

    #[tracing::instrument(skip_all, name = "workspace.init")]
    pub fn init(&self) -> Result<()> {
        info!("Initializing workspace");
        self.adapter.init()?;
        if self.repository_exists() {
            self.clone_repository()
        } else {
            self.update_repository()?;
            self.clean_repository()
        }
    }

    #[tracing::instrument(skip(self), name = "workspace.cmd", err, ret)]
    pub fn cmd(&self, cmd: &str) -> Result<()> {
        self.adapter.cmd(cmd, self.codebase.working_dir.as_deref())
    }

    #[tracing::instrument(skip(self), name = "workspace.cmd_with_output", err, ret)]
    pub fn cmd_with_output(&self, cmd: &str) -> Result<String> {
        self.adapter
            .cmd_with_output(cmd, self.codebase.working_dir.as_deref())
    }

    #[tracing::instrument(skip(self, content), name = "workspace.write_file", err)]
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        self.adapter
            .write_file(path, content, self.codebase.working_dir.as_deref())
    }

    #[tracing::instrument(skip(self), name = "workspace.read_file", err)]
    pub fn read_file(&self, path: &str) -> Result<String> {
        self.adapter
            .read_file(path, self.codebase.working_dir.as_deref())
    }

    #[tracing::instrument(skip_all, name = "workspace.repository_exists")]
    fn repository_exists(&self) -> bool {
        self.adapter.cmd("ls -A .git", None).is_err()
    }

    #[tracing::instrument(skip_all, name = "workspace.clone_repository")]
    fn clone_repository(&self) -> Result<()> {
        self.adapter
            .cmd(&format!("git clone {} .", self.codebase.url), None)
    }

    #[tracing::instrument(skip_all, name = "workspace.update_repository")]
    fn update_repository(&self) -> Result<()> {
        self.adapter.cmd("git pull", None)
    }

    #[tracing::instrument(skip_all, name = "workspace.clean_repository")]
    fn clean_repository(&self) -> Result<()> {
        self.adapter.cmd("git clean -f .", None)?;
        self.adapter.cmd("git checkout .", None)
    }
}
