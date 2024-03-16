use crate::adapters::Adapter;
use crate::Codebase;
use anyhow::Result;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone)]
pub struct Workspace(Arc<Mutex<WorkspaceInner>>);

#[derive(Debug)]
pub struct WorkspaceInner {
    adapter: Box<dyn Adapter>,
    pub codebase: Codebase,
}

impl Workspace {
    #[tracing::instrument]
    pub fn new(adapter: Box<dyn Adapter>, codebase: &Codebase) -> Self {
        let inner = WorkspaceInner {
            adapter,
            codebase: codebase.to_owned(),
        };

        Self(Arc::new(Mutex::new(inner)))
    }

    #[tracing::instrument(skip_all, target = "bosun", name = "workspace.init")]
    pub async fn init(&self) -> Result<()> {
        info!("Initializing workspace");

        self.authenticate_with_repository_if_possible().await?;
        self.0.lock().await.adapter.init()?;
        if self.repository_exists().await {
            self.clean_repository().await
        } else {
            self.clone_repository().await
        }
    }

    #[tracing::instrument(skip(self), target = "bosun", name = "workspace.cmd", err, ret)]
    pub async fn cmd(&self, cmd: &str) -> Result<()> {
        let inner = self.0.lock().await;

        inner
            .adapter
            .cmd(cmd, inner.codebase.working_dir.as_deref())
    }

    pub async fn clone_codebase(&self) -> Codebase {
        // Clones it for now
        // Alternative is to return the MutexGuard
        let guard = self.0.lock().await;
        guard.codebase.clone()
    }

    #[tracing::instrument(
        skip(self),
        target = "bosun",
        name = "workspace.cmd_with_output",
        err,
        ret
    )]
    pub async fn cmd_with_output(&self, cmd: &str) -> Result<String> {
        let inner = self.0.lock().await;

        inner
            .adapter
            .cmd_with_output(cmd, inner.codebase.working_dir.as_deref())
    }

    #[tracing::instrument(
        skip(self, content),
        target = "bosun",
        name = "workspace.write_file",
        err
    )]
    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let inner = self.0.lock().await;

        inner
            .adapter
            .write_file(path, content, inner.codebase.working_dir.as_deref())
    }

    #[tracing::instrument(skip(self), target = "bosun", name = "workspace.read_file", err)]
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let inner = self.0.lock().await;

        inner
            .adapter
            .read_file(path, inner.codebase.working_dir.as_deref())
    }

    #[tracing::instrument(skip_all, target = "bosun", name = "workspace.repository_exists")]
    async fn repository_exists(&self) -> bool {
        let inner = self.0.lock().await;

        inner.adapter.cmd("ls -A .git", None).is_ok()
    }

    #[tracing::instrument(skip_all, target = "bosun", name = "workspace.clone_repository")]
    async fn clone_repository(&self) -> Result<()> {
        let inner = self.0.lock().await;

        inner
            .adapter
            .cmd(&format!("git clone {} .", inner.codebase.url), None)
    }

    #[tracing::instrument(skip_all, target = "bosun", name = "workspace.clean_repository")]
    async fn clean_repository(&self) -> Result<()> {
        let inner = self.0.lock().await;

        let cmd = "git switch -fC $(git symbolic-ref refs/remotes/origin/HEAD | sed 's@^refs/remotes/origin/@@')";
        inner.adapter.cmd(cmd, None)
    }

    #[tracing::instrument(skip_all)]
    async fn authenticate_with_repository_if_possible(&self) -> Result<()> {
        let mut inner = self.0.lock().await;
        if let Ok(github_url) =
            infrastructure::github_token_generator::add_token_to_url(&inner.codebase.url).await
        {
            tracing::warn!("Token added to codebase url");
            inner.codebase.url = github_url;
        } else {
            tracing::warn!("Could not add token to codebase url");
        }
        Ok(())
    }
}
