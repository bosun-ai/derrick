use anyhow::Result;
use derive_builder::Builder;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default, PartialEq, Builder)]
#[serde(rename_all = "camelCase")]
#[builder(
    derive(Deserialize, Debug),
    setter(into, strip_option),
    build_fn(error = "anyhow::Error")
)]
#[builder_struct_attr(serde(rename_all = "camelCase"))]
pub struct Repository {
    #[builder(default = "self.default_repository_name()?")]
    pub url: String,
    #[builder(default)]
    pub path: String,
    #[builder(default)]
    pub reference: Option<String>,
}

impl Repository {
    pub fn from_url(url: impl Into<String>) -> RepositoryBuilder {
        RepositoryBuilder::default().url(url.into()).to_owned()
    }

    pub fn builder() -> RepositoryBuilder {
        RepositoryBuilder::default()
    }
}

impl From<&Repository> for Repository {
    fn from(val: &Repository) -> Self {
        val.clone()
    }
}

impl RepositoryBuilder {
    fn default_repository_name(&self) -> Result<String> {
        let mut parts = self
            .url
            .as_ref()
            .ok_or(anyhow::anyhow!("Expected url when building repository"))?
            .split('/');
        let last_two = parts.by_ref().rev().take(2).collect::<Vec<&str>>();

        Ok(format!(
            "{}/{}",
            last_two[1],
            last_two[0].trim_end_matches(".git")
        ))
    }
}
