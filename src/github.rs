use tokio::sync::RwLock;

use anyhow::{Context, Result};
use base64::prelude::*;
use itertools::Itertools;
use jsonwebtoken::EncodingKey;
use octocrab::models::issues::{Comment, Issue};
use octocrab::models::pulls::PullRequest;
use octocrab::models::{Installation, InstallationId};
use octocrab::Octocrab;
use octocrab::{models::InstallationToken, params::apps::CreateInstallationAccessToken};
use url::Url;

fn generate_jwt_key() -> Result<EncodingKey> {
    let mut app_private_key = std::env::var("GITHUB_PRIVATE_KEY").context(
        "Could not find GITHUB_PRIVATE_KEY in environment. Make sure to set it in the .env file",
    )?;
    app_private_key = String::from_utf8(BASE64_STANDARD.decode(app_private_key)?)?;

    jsonwebtoken::EncodingKey::from_rsa_pem(app_private_key.as_bytes())
        .context("Could not generate jwt token")
}

fn extract_owner_and_repo(repo_url: &str) -> Result<(String, String)> {
    let url = url::Url::parse(repo_url)?;
    if let Some((owner, repo)) = url.path_segments().and_then(|s| s.take(2).collect_tuple()) {
        Ok((owner.to_string(), repo.trim_end_matches(".git").to_string()))
    } else {
        anyhow::bail!("Could not extract owner and repo from url")
    }
}

fn get_octocrab() -> Result<Octocrab> {
    if cfg!(feature = "integration_testing") {
        let key = generate_jwt_key()?;
        return Octocrab::builder()
            .base_uri(
                crate::config()
                    .github_endpoint
                    .clone()
                    .expect("Need GITHUB_ENDPOINT during integration tests"),
            )?
            .app(1.into(), key)
            .build()
            .context("Failed to build octocrab");
    }
    let jwt = generate_jwt_key()?;

    let app_id = crate::config()
        .github_app_id
        .ok_or_else(|| anyhow::anyhow!("GITHUB_APP_ID not set"))?
        .into();

    Octocrab::builder()
        .app(app_id, jwt)
        .build()
        .context("Failed to build octocrab")
}

#[derive(Debug)]
pub struct GithubSession {
    octocrab: Octocrab,
    installation_id: RwLock<Option<InstallationId>>,
}

impl GithubSession {
    pub fn try_new() -> Result<Self> {
        Ok(Self {
            octocrab: get_octocrab()?,
            installation_id: RwLock::new(None),
        })
    }

    pub async fn user(&self) -> Result<octocrab::models::Author> {
        let current = self.octocrab.current();
        let name = current.app().await.map_err(anyhow::Error::msg)?.name;
        let user: octocrab::models::Author = octocrab::instance()
            .get(format!("/users/{}[bot]", name), None::<&()>)
            .await?;
        Ok(user)
    }

    #[tracing::instrument(skip_all)]
    async fn with_installation_for_repo(&self, repo_url: &str) -> Result<Octocrab> {
        if let Some(installation_id) = *self.installation_id.read().await {
            return Ok(self.octocrab.installation(installation_id));
        }

        let installation = self.get_installation(repo_url).await?;
        *self.installation_id.write().await = Some(installation.id);

        Ok(self.octocrab.installation(installation.id))
    }

    #[tracing::instrument(skip_all)]
    async fn get_installation(&self, repo_url: &str) -> Result<Installation> {
        let (owner, repo) =
            extract_owner_and_repo(repo_url).context("Could not find owner or repo")?;

        tracing::info!(repo_url, owner, repo, "Getting installation");
        self.octocrab
            .apps()
            .get_repository_installation(&owner, &repo)
            .await
            .map_err(anyhow::Error::msg)
    }

    #[tracing::instrument(skip_all)]
    async fn create_installation_token(
        &self,
        installation: Installation,
    ) -> Result<InstallationToken> {
        let create_access_token = CreateInstallationAccessToken::default();
        let access_token_url = Url::parse(installation.access_tokens_url.as_ref().unwrap())?;

        self.octocrab
            .post(access_token_url.path(), Some(&create_access_token))
            .await
            .map_err(anyhow::Error::msg)
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_issue(&self, repo_url: &str, issue_number: u64) -> Result<Issue> {
        let (owner, repo) =
            extract_owner_and_repo(repo_url).context("Could not find owner or repo")?;

        self.with_installation_for_repo(repo_url)
            .await?
            .issues(owner, repo)
            .get(issue_number)
            .await
            .map_err(anyhow::Error::msg)
    }

    #[tracing::instrument(skip_all)]
    pub async fn create_issue(&self, repo_url: &str, title: &str, body: &str) -> Result<Issue> {
        let (owner, repo) =
            extract_owner_and_repo(repo_url).context("Could not find owner or repo")?;
        self.with_installation_for_repo(repo_url)
            .await?
            .issues(owner, repo)
            .create(title)
            .body(body)
            .send()
            .await
            .map_err(anyhow::Error::msg)
    }

    #[tracing::instrument(skip_all)]
    pub async fn update_issue(
        &self,
        repo_url: &str,
        issue_number: u64,
        body: &str,
    ) -> Result<Issue> {
        let (owner, repo) =
            extract_owner_and_repo(repo_url).context("Could not find owner or repo")?;
        self.with_installation_for_repo(repo_url)
            .await?
            .issues(owner, repo)
            .update(issue_number)
            .body(body)
            .send()
            .await
            .map_err(anyhow::Error::msg)
    }

    #[tracing::instrument(skip_all)]
    pub async fn create_merge_request(
        &self,
        repo_url: &str,
        branch_name: &str,
        base_branch_name: &str,
        title: &str,
        description: &str,
    ) -> Result<PullRequest> {
        let (owner, repo) =
            extract_owner_and_repo(repo_url).context("Could not find owner or repo")?;

        self.with_installation_for_repo(repo_url)
            .await?
            .pulls(owner, repo)
            .create(title, branch_name, base_branch_name)
            .body(description)
            .send()
            .await
            .map_err(anyhow::Error::msg)
    }

    #[tracing::instrument(skip_all)]
    pub async fn add_comment_to_merge_request(
        &self,
        repo_url: &str,
        merge_request: &PullRequest,
        comment: &str,
    ) -> Result<Comment> {
        let (owner, repo) =
            extract_owner_and_repo(repo_url).context("Could not find owner or repo")?;

        self.with_installation_for_repo(repo_url)
            .await?
            .issues(owner, repo)
            .create_comment(merge_request.number, comment)
            .await
            .map_err(anyhow::Error::msg)
    }

    #[tracing::instrument(skip_all)]
    pub async fn add_token_to_url(&self, repo_url: &str) -> Result<String> {
        if !repo_url.starts_with("https://") {
            anyhow::bail!("Only https urls are supported")
        }

        let mut parsed = url::Url::parse(repo_url).context("Failed to parse url")?;

        let installation = self
            .get_installation(repo_url)
            .await
            .context("Failed to get installation")?;
        let installation_id = installation.id.to_string();
        let token = self
            .create_installation_token(installation)
            .await
            .context("Failed to create installation token")?;

        let result1 = parsed.set_username("x-access-token");
        let result2 = parsed.set_password(Some(&token.token));
        if result1.is_err() || result2.is_err() {
            anyhow::bail!("Could not set token on url")
        }

        tracing::info!(installation_id = installation_id, "Token added to url");
        Ok(parsed.to_string())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_extract_owner_and_repo() {
        let inputs = [
            "https://github.com/bosun-ai/fluyt",
            "https://github.com/bosun-ai/fluyt.git",
        ];

        for input in inputs {
            let owner_and_repo = extract_owner_and_repo(input).expect("Failed to extract");
            assert_eq!(
                owner_and_repo,
                ("bosun-ai".to_string(), "fluyt".to_string())
            );
        }
    }
}
