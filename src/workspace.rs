use crate::adapters::Adapter;
use crate::Codebase;
use anyhow::Result;
use octocrab::models::pulls::PullRequest;
use shell_escape::escape as escape_cow;
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

fn escape(s: &str) -> String {
    escape_cow(std::borrow::Cow::Borrowed(s)).to_string()
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

    pub async fn full_path(&self) -> String {
        let inner = self.0.lock().await;

        inner.adapter.path(inner.codebase.working_dir.as_deref())
    }

    #[tracing::instrument(skip_all, target = "bosun", name = "workspace.init")]
    pub async fn init(&self) -> Result<()> {
        info!("Initializing workspace");

        self.authenticate_with_repository_if_possible().await?;
        self.0.lock().await.adapter.init().await?;

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
            .await
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
            .await
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
            .await
    }

    #[tracing::instrument(skip(self), target = "bosun", name = "workspace.read_file", err)]
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let inner = self.0.lock().await;

        inner
            .adapter
            .read_file(path, inner.codebase.working_dir.as_deref())
            .await
    }

    #[tracing::instrument(skip_all, target = "bosun", name = "workspace.repository_exists")]
    async fn repository_exists(&self) -> bool {
        let inner = self.0.lock().await;

        inner.adapter.cmd("ls -A .git", None).await.is_ok()
    }

    #[tracing::instrument(skip_all, target = "bosun", name = "workspace.clone_repository")]
    async fn clone_repository(&self) -> Result<()> {
        let inner = self.0.lock().await;

        inner
            .adapter
            .cmd(
                &format!("git clone {} .", escape(inner.codebase.url.as_str())),
                None,
            )
            .await
    }

    #[tracing::instrument(skip_all, target = "bosun", name = "workspace.clean_repository")]
    async fn clean_repository(&self) -> Result<()> {
        let inner = self.0.lock().await;

        let cmd = "git switch -fC $(git symbolic-ref refs/remotes/origin/HEAD | sed 's@^refs/remotes/origin/@@')";
        inner.adapter.cmd(cmd, None).await
    }

    #[tracing::instrument(skip_all)]
    async fn authenticate_with_repository_if_possible(&self) -> Result<()> {
        let mut inner = self.0.lock().await;

        if let Ok(github_session) = infrastructure::github::GithubSession::try_new() {
            let github_url = github_session.add_token_to_url(&inner.codebase.url).await?;
            tracing::warn!("Token added to codebase url");
            inner.codebase.url = github_url;
        } else {
            tracing::warn!("Could not add token to codebase url");
        }
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn create_branch(&self, maybe_name: Option<String>) -> Result<String> {
        let inner = self.0.lock().await;

        let name = maybe_name
            .map(|n| escape(n.as_str()))
            .unwrap_or_else(|| format!("generated/{}", uuid::Uuid::new_v4()));

        let cmd = format!("git switch -c {}", name);
        inner.adapter.cmd(&cmd, None).await?;
        Ok(name)
    }

    #[tracing::instrument(skip_all)]
    pub async fn commit(&self, message: &str, files: Option<Vec<String>>) -> Result<()> {
        let inner = self.0.lock().await;

        if let Some(files) = files {
            // first add all the files, making sure to surround them with quotes
            let add_cmd = format!(
                "git add {}",
                files
                    .iter()
                    .map(|f| format!("\"{}\"", escape(f.as_str())))
                    .collect::<Vec<String>>()
                    .join(" ")
            );

            inner.adapter.cmd(&add_cmd, None).await?;

            let cmd = format!("git commit -m '{}'", escape(message));
            inner.adapter.cmd(&cmd, None).await
        } else {
            let add_cmd = "git add .";
            inner.adapter.cmd(add_cmd, None).await?;
            let cmd = format!("git commit -m '{}'", escape(message));
            inner.adapter.cmd(&cmd, None).await
        }
    }

    #[tracing::instrument(skip_all)]
    pub async fn push(&self, target_branch: &str) -> Result<()> {
        let inner = self.0.lock().await;

        let cmd = format!("git push origin HEAD:{}", escape(target_branch));
        inner.adapter.cmd(&cmd, None).await
    }

    #[tracing::instrument(skip_all)]
    pub async fn create_merge_request(
        &self,
        title: &str,
        description: &str,
    ) -> Result<PullRequest> {
        let branch_name = self.create_branch(None).await?;
        self.push(&branch_name).await?;

        let github_session = infrastructure::github::GithubSession::try_new()?;
        let repo_url = self.0.lock().await.codebase.url.clone();

        let mr = github_session
            .create_merge_request(&repo_url, &branch_name, "master", title, description)
            .await?;

        tracing::info!("Created merge request: {}", mr.url);

        Ok(mr)
    }
}
