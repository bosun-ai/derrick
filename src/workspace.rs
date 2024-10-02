use crate::workspace_controllers::WorkspaceController;
use crate::repository::Repository;
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
    adapter: Box<dyn WorkspaceController>,
    pub repository: Repository,
}

fn escape(s: &str) -> String {
    escape_cow(std::borrow::Cow::Borrowed(s)).to_string()
}

static MAIN_BRANCH_CMD: &str =
    "git symbolic-ref refs/remotes/origin/HEAD | sed 's@^refs/remotes/origin/@@'";

impl Workspace {
    #[tracing::instrument(skip_all)]
    pub fn new(adapter: Box<dyn WorkspaceController>, repository: &Repository) -> Self {
        let inner = WorkspaceInner {
            adapter,
            repository: repository.to_owned(),
        };

        Self(Arc::new(Mutex::new(inner)))
    }

    #[tracing::instrument(skip_all, fields(bosun.tracing=true), name = "workspace.init")]
    pub async fn init(&self) -> Result<()> {
        info!("Initializing workspace");

        self.authenticate_with_repository_if_possible().await?;
        self.0.lock().await.adapter.init().await?;

        if self.repository_exists().await {
            self.configure_git().await?;
            // Token might be outdated so lets update it
            self.update_remote().await?;
            self.clean_repository().await
        } else {
            self.clone_repository().await?;
            self.configure_git().await
        }
    }

    #[tracing::instrument(skip(self), fields(bosun.tracing=true), name = "workspace.cmd", err, ret)]
    pub async fn cmd(&self, cmd: &str) -> Result<()> {
        let inner = self.0.lock().await;

        inner.adapter.cmd(cmd, None).await
    }

    pub async fn repository(&self) -> Repository {
        // Clones it for now
        // Alternative is to return the MutexGuard
        let guard = self.0.lock().await;
        guard.repository.clone()
    }

    #[tracing::instrument(
        skip(self), fields(bosun.tracing=true),
        name = "workspace.cmd_with_output",
        err,
        ret
    )]
    pub async fn cmd_with_output(&self, cmd: &str) -> Result<String> {
        let inner = self.0.lock().await;

        inner.adapter.cmd_with_output(cmd, None).await
    }

    #[tracing::instrument(
        skip(self, content), fields(bosun.tracing=true),
        name = "workspace.write_file",
        err
    )]
    pub async fn write_file(&self, path: &str, content: &str) -> Result<()> {
        let inner = self.0.lock().await;

        inner.adapter.write_file(path, content, None).await
    }

    #[tracing::instrument(skip(self), fields(bosun.tracing=true), name = "workspace.read_file", err)]
    pub async fn read_file(&self, path: &str) -> Result<String> {
        let inner = self.0.lock().await;

        inner.adapter.read_file(path, None).await
    }

    // TODO: All the git commands should be pushed to the adapters so that there is a well defined
    // interface for interacting with git that can be controlled by the adapters.

    #[tracing::instrument(skip_all, fields(bosun.tracing=true), name = "workspace.repository_exists")]
    async fn repository_exists(&self) -> bool {
        let inner = self.0.lock().await;

        inner.adapter.cmd("ls -A .git", None).await.is_ok()
    }

    #[tracing::instrument(skip_all, fields(bosun.tracing=true), name = "workspace.clone_repository")]
    async fn clone_repository(&self) -> Result<()> {
        let inner = self.0.lock().await;

        let url = escape(inner.repository.clone_url.as_str());

        inner
            .adapter
            .cmd(&format!("git clone {} .", url), None)
            .await
    }

    #[tracing::instrument(skip_all, fields(bosun.tracing=true), name = "workspace.update_remote")]
    async fn update_remote(&self) -> Result<()> {
        let inner = self.0.lock().await;
        let url = inner.repository.clone_url.clone();

        let cmd = format!("git remote set-url origin {}", escape(&url));
        inner.adapter.cmd(&cmd, None).await
    }

    #[tracing::instrument(skip_all, fields(bosun.tracing=true), name = "workspace.clean_repository")]
    async fn clean_repository(&self) -> Result<()> {
        let inner = self.0.lock().await;

        let checkout_cmd = format!("git checkout $({MAIN_BRANCH_CMD})");
        let cmds = vec![
            "git reset --hard",
            "git clean -fd",
            "git fetch origin",
            &checkout_cmd,
        ];

        for cmd in cmds {
            inner.adapter.cmd(cmd, None).await?;
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, fields(bosun.tracing=true), name = "workspace.configure_git")]
    async fn configure_git(&self) -> Result<()> {
        if cfg!(feature = "integration_testing") {
            return Ok(());
        }

        let inner = self.0.lock().await;
        match crate::github::GithubSession::try_new() {
            Ok(github_session) => {
                // https://github.com/orgs/community/discussions/24664
                let user = github_session.user().await?;
                let bot_email = format!("{}+{}@users.noreply.github.com", user.id, user.login);
                let bot_username = user.login;
                inner
                    .adapter
                    .cmd(
                        format!("git config user.email \"{}\"", bot_email).as_str(),
                        None,
                    )
                    .await?;
                inner
                    .adapter
                    .cmd(
                        format!("git config user.name \"{}\"", bot_username).as_str(),
                        None,
                    )
                    .await?;
            }
            Err(_e) => {
                inner
                    .adapter
                    .cmd("git config user.email \"swabbie@bosun.ai\"", None)
                    .await?;
                inner
                    .adapter
                    .cmd("git config user.name \"Swabbie\"", None)
                    .await?;
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, err)]
    async fn authenticate_with_repository_if_possible(&self) -> Result<()> {
        // Auth is tricky and not needed in integration tests.

        if cfg!(feature = "integration_testing") {
            return Ok(());
        }

        match crate::github::GithubSession::try_new() {
            Ok(github_session) => {
                // Locks should never go over awaits
                let mut codebase_url: String = String::new();
                {
                    let guard = self.0.lock().await;
                    guard.repository.clone_url.clone_into(&mut codebase_url)
                }

                let github_url = github_session.add_token_to_url(&codebase_url).await?;
                tracing::warn!("Token added to codebase url");

                let mut inner = self.0.lock().await;
                inner.repository.clone_url = github_url;
            }
            Err(e) => {
                tracing::warn!(error = ?e, "Could not authenticate with github, continuing anyway ...");
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn create_branch(&self, maybe_name: Option<&str>) -> Result<String> {
        let inner = self.0.lock().await;

        let name = maybe_name
            .map(escape)
            .unwrap_or_else(|| format!("generated/{}", uuid::Uuid::new_v4()));

        let cmd = format!("git switch -c {}", name);
        inner.adapter.cmd(&cmd, None).await?;
        Ok(name)
    }

    #[tracing::instrument(skip_all, err)]
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

            let cmd = format!("git commit -m {}", escape(message));
            inner.adapter.cmd(&cmd, None).await
        } else {
            let add_cmd = "git add .";
            inner.adapter.cmd(add_cmd, None).await?;
            let cmd = format!("git commit -m {}", escape(message));
            inner.adapter.cmd(&cmd, None).await
        }
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn push(&self, target_branch: &str) -> Result<()> {
        let inner = self.0.lock().await;

        let cmd = format!("git push origin HEAD:{}", escape(target_branch));
        inner.adapter.cmd(&cmd, None).await
    }

    #[tracing::instrument(skip_all, err)]
    pub async fn create_merge_request(
        &self,
        title: &str,
        description: &str,
        branch_name: &str,
    ) -> Result<PullRequest> {
        let github_session = crate::github::GithubSession::try_new()?;
        let repo_url = self.0.lock().await.repository.clone_url.clone();
        let main_branch = self
            .cmd_with_output(MAIN_BRANCH_CMD)
            .await?
            .trim()
            .to_owned();

        let mr = github_session
            .create_merge_request(&repo_url, branch_name, &main_branch, title, description)
            .await?;

        tracing::info!("Created merge request: {}", mr.url);

        Ok(mr)
    }
}
