use crate::adapters::Adapter;
use anyhow::Result;
use std::fmt::Debug;
use tracing::info;

#[derive(Debug)]
pub struct Workspace {
    adapter: Box<dyn Adapter>,
    codebase_url: String,
    working_dir: Option<String>,
}

impl Workspace {
    #[tracing::instrument]
    pub fn new(adapter: Box<dyn Adapter>, codebase_url: &str, working_dir: Option<&str>) -> Self {
        Self {
            adapter,
            codebase_url: codebase_url.to_owned(),
            working_dir: working_dir.map(|s| s.to_owned()),
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

    #[tracing::instrument(skip_all, name = "workspace.cmd", err)]
    pub fn cmd(&self, cmd: &str) -> Result<()> {
        self.adapter.cmd(cmd, self.working_dir.as_deref())
    }

    #[tracing::instrument(skip_all, name = "workspace.cmd_with_output", err)]
    pub fn cmd_with_output(&self, cmd: &str) -> Result<String> {
        self.adapter
            .cmd_with_output(cmd, self.working_dir.as_deref())
    }

    #[tracing::instrument(skip_all, name = "workspace.write_file", err)]
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        self.adapter
            .write_file(path, content, self.working_dir.as_deref())
    }

    #[tracing::instrument]
    fn repository_exists(&self) -> bool {
        self.adapter.cmd("ls -A .git", None).is_err()
    }

    #[tracing::instrument]
    fn clone_repository(&self) -> Result<()> {
        self.adapter
            .cmd(&format!("git clone {} .", self.codebase_url), None)
    }

    #[tracing::instrument]
    fn update_repository(&self) -> Result<()> {
        self.adapter.cmd("git pull", None)
    }

    #[tracing::instrument]
    fn clean_repository(&self) -> Result<()> {
        self.adapter.cmd("git clean -f .", None)?;
        self.adapter.cmd("git checkout .", None)
    }
}
