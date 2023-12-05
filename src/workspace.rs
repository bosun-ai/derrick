use crate::adapters::Adapter;
use anyhow::Result;
use std::fmt::Debug;
use tracing::info;

#[derive(Debug)]
pub struct Workspace {
    adapter: Box<dyn Adapter>,
    codebase_url: String,
}

impl Workspace {
    #[tracing::instrument]
    pub fn new(adapter: Box<dyn Adapter>, codebase_url: String) -> Self {
        Self {
            adapter,
            codebase_url,
        }
    }

    #[tracing::instrument(skip_all, name = "Workspace#init")]
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

    #[tracing::instrument(skip_all, name = "Workspace#cmd")]
    pub fn cmd(&self, cmd: &str) -> Result<()> {
        self.adapter.cmd(cmd)
    }

    #[tracing::instrument(skip_all, name = "Workspace#cmd_with_output")]
    pub fn cmd_with_output(&self, cmd: &str) -> Result<String> {
        self.adapter.cmd_with_output(cmd)
    }

    #[tracing::instrument(skip_all, name = "Workspace#write_file")]
    pub fn write_file(&self, path: &str, content: &str) -> Result<()> {
        self.adapter.write_file(path, content)
    }

    #[tracing::instrument]
    fn repository_exists(&self) -> bool {
        self.adapter.cmd("ls -A .git").is_err()
    }

    #[tracing::instrument]
    fn clone_repository(&self) -> Result<()> {
        self.adapter
            .cmd(&format!("git clone {} .", self.codebase_url))
    }

    #[tracing::instrument]
    fn update_repository(&self) -> Result<()> {
        self.adapter.cmd("git pull")
    }

    #[tracing::instrument]
    fn clean_repository(&self) -> Result<()> {
        self.adapter.cmd("git clean -f .")?;
        self.adapter.cmd("git checkout .")
    }
}
