use crate::adapters::Adapter;
use anyhow::{Context, Result};
use async_trait::async_trait;
use regex;
use std::process::Command;
use std::sync::OnceLock;
use std::{collections::HashMap, path::PathBuf};
use tokio::sync::RwLock;
use tracing::{debug, warn};

const ALLOWED_ENV: &[&str] = &["PATH", "CARGO_HOME", "RUST_HOME", "RUST_VERSION"];
// Runs commands in a local temporary directory
// Useful for debugging, testing and experimentation
//
// NOTE:
//  - might be useful to drop the directory after out of scope
//  - haven't decided what to do with stdout/stderr
#[derive(Debug)]
pub struct LocalTempSync {
    name: String,
    path: OnceLock<String>,
    whitelisted_env: RwLock<HashMap<String, String>>,
}

// scrub removes x-access-token:<token> from a string like x-access-token:1234@github.com
fn scrub(output: &str) -> String {
    let re = regex::Regex::new(r"x-access-token:[^@]+@").unwrap();
    re.replace_all(output, "x-access-token:***@").to_string()
}

impl LocalTempSync {
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            path: OnceLock::new(),
            whitelisted_env: Default::default(),
        }
    }

    fn spawn_cmd(
        &self,
        cmd: &str,
        working_dir: Option<&str>,
        envs: &HashMap<String, String>,
    ) -> Result<std::process::Output> {
        debug!(
            cmd = scrub(cmd),
            path = self
                .path(working_dir)
                .to_str()
                .context("Could not convert path to string")?,
            "Running command"
        );
        Command::new("bash")
            .args(["-c", cmd])
            .env_clear()
            .envs(envs)
            .current_dir(self.path(working_dir))
            .output()
            .context("Could not run command")
    }
}

fn init_path(name: &str) -> Result<String> {
    let mut current_dir = std::env::current_dir().expect("Could not get current directory");
    current_dir.push("tmp");
    current_dir.push(name);

    if !current_dir.exists() {
        std::fs::create_dir_all(&current_dir).context("Could not create local temp directory")?;
    }
    Ok(current_dir
        .canonicalize()?
        .to_str()
        .context("Could not convert to string")?
        .to_string())
}

#[async_trait]
impl Adapter for LocalTempSync {
    fn path(&self, working_dir: Option<&str>) -> PathBuf {
        let mut base_path = std::path::PathBuf::from(
            self.path
                .get()
                .context("Expected path to be set, workspace not initialized?")
                .unwrap(),
        );

        let mut working_dir = std::path::PathBuf::from(working_dir.unwrap_or(""));

        if working_dir.is_absolute() {
            working_dir = working_dir
                .strip_prefix("/")
                .expect("Expected working_dir to be an absolute path, could not strip prefix '/'")
                .to_path_buf();
        }

        base_path.push(working_dir);
        base_path
    }

    #[tracing::instrument(skip_all)]
    async fn init(&self) -> Result<()> {
        self.path.get_or_init(|| {
            init_path(&self.name)
                .context("Could not create local temp directory")
                .unwrap()
        });
        warn!(path = &self.path.get(), "Creating local temp directory");

        let mut whitelisted_env = self.whitelisted_env.write().await;
        for (key, value) in std::env::vars() {
            if ALLOWED_ENV.contains(&key.as_str()) {
                whitelisted_env.insert(key, value);
            }
        }

        Ok(())
    }

    #[tracing::instrument(skip(self), fields(cmd = scrub(cmd)))]
    async fn cmd(&self, cmd: &str, working_dir: Option<&str>) -> Result<()> {
        let envs = self.whitelisted_env.read().await.clone();
        self.spawn_cmd(cmd, working_dir, &envs)
            .map(handle_command_result)?
            .map(|_| ())
    }

    #[tracing::instrument(skip(self), fields(cmd = scrub(cmd)))]
    async fn cmd_with_output(&self, cmd: &str, working_dir: Option<&str>) -> Result<String> {
        let envs = self.whitelisted_env.read().await.clone();
        self.spawn_cmd(cmd, working_dir, &envs)
            .map(handle_command_result)?
    }

    #[tracing::instrument(skip_all)]
    async fn write_file(&self, file: &str, content: &str, working_dir: Option<&str>) -> Result<()> {
        let path = self.path(working_dir).as_path().join(file);

        // Create directory if it doesn't exist
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent).context("Could not create directory")?;
        }
        std::fs::write(path, content).context("Could not write file")
    }

    #[tracing::instrument(skip_all)]
    async fn read_file(&self, file: &str, working_dir: Option<&str>) -> Result<String> {
        let path = self.path(working_dir).as_path().join(file);
        std::fs::read_to_string(path).context("Could not read file")
    }
}

#[tracing::instrument(skip_all)]
fn handle_command_result(result: std::process::Output) -> Result<String> {
    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    if result.status.success() {
        debug!(stdout = &stdout, stderr = &stderr, "Command succeeded");
        Ok(stdout)
    } else {
        warn!(stdout = &stdout, stderr = &stderr, "Command failed");
        Err(anyhow::anyhow!(stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[tokio::test]
    async fn test_cmd_with_output() {
        let adapter = LocalTempSync::new("test");
        adapter.init().await.unwrap();
        let result = adapter.cmd_with_output("pwd", None).await;
        assert!(result.is_ok());
        let stdout = result.unwrap();
        assert!(stdout.contains("tmp/test"));
    }

    #[tokio::test]
    async fn test_sets_path_correctly_for_run_cmd() {
        let adapter = LocalTempSync::new("test");
        adapter.init().await.unwrap();
        let output = adapter.spawn_cmd("pwd", None, &Default::default()).unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        assert!(stdout.contains("tmp/test"));
    }

    #[tokio::test]
    async fn test_working_dir() {
        let adapter = LocalTempSync::new("test");
        adapter.init().await.unwrap();
        adapter
            .spawn_cmd("mkdir subdir", None, &Default::default())
            .unwrap();
        let output = adapter
            .spawn_cmd("pwd", Some("subdir"), &Default::default())
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        assert!(stdout.contains("tmp/test/subdir"));
        assert!(adapter
            .path(Some("subdir"))
            .to_string_lossy()
            .contains("tmp/test/subdir"));
    }

    #[test]
    fn test_init_path() {
        let path = init_path("test").unwrap();
        dbg!(&path);
        let regex = regex::Regex::new(r"^.*/tmp/test$").unwrap();
        assert!(regex.is_match(&path));
        assert!(std::path::PathBuf::from(&path).exists())
    }

    #[tokio::test]
    async fn test_init() {
        let adapter = LocalTempSync::new("test");
        let result = adapter.init().await;
        assert!(result.is_ok());
        let path = adapter.path(None);
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_cmd_valid() {
        let adapter = LocalTempSync::new("test");
        adapter.init().await.unwrap();
        let result = adapter.cmd("ls", None).await;
        println!("{:#?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cmd_invalid() {
        let adapter = LocalTempSync::new("test");
        adapter.init().await.unwrap();
        let result = adapter.cmd("invalid command", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_piping_a_command() {
        let adapter = LocalTempSync::new("test");
        adapter.init().await.unwrap();
        adapter.cmd("echo 'hello' > test.txt", None).await.unwrap();
        // check if file was created
        let result = adapter.cmd("cat test.txt | grep 'hello'", None).await;
        dbg!(&result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_writing_file() {
        let adapter = LocalTempSync::new("test");
        adapter.init().await.unwrap();
        adapter
            .write_file("write.txt", "Hello, world!", None)
            .await
            .expect("Could not write file");
        let result = adapter.cmd_with_output("cat write.txt", None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");

        adapter
            .write_file("write.txt", "Hello, back!", None)
            .await
            .unwrap();
        let result = adapter.cmd_with_output("cat write.txt", None).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, back!");
    }

    #[tokio::test]
    async fn test_reading_file_with_nextjs_style_path() {
        let adapter = LocalTempSync::new("test");
        let path = "(unauthenticated)/[slug]/index.tsx";

        adapter.init().await.unwrap();
        adapter
            .write_file(path, "Hello, world!", None)
            .await
            .expect("Could not write file");
        let result = adapter
            .read_file(path, None)
            .await
            .expect("Could not read file");
        assert_eq!(result, "Hello, world!");
    }

    #[tokio::test]
    async fn test_it_should_write_and_read_newlines_and_other_weird_characters() {
        let adapter = LocalTempSync::new("weird_characters");
        adapter.init().await.unwrap();
        adapter
            .write_file("write.txt", "Hello, world!\n", None)
            .await
            .expect("Could not write file");

        let result = adapter.read_file("write.txt", None).await.unwrap();
        assert_eq!(result, "Hello, world!\n");

        // And unicode characters
        adapter
            .write_file("write.txt", "Hello, 🌍!\n", None)
            .await
            .expect("Could not write file");

        let result = adapter.read_file("write.txt", None).await.unwrap();
        assert_eq!(result, "Hello, 🌍!\n");
    }

    #[tokio::test]
    async fn test_it_should_allow_whitelisted_env_variables() {
        let adapter = LocalTempSync::new("whitelisted_env");
        adapter.init().await.unwrap();

        let env = adapter.cmd_with_output("printenv", None).await.unwrap();

        // In tests we only have path available, so just check that
        // We cannot reliably set env variables in test to to multithreading
        assert!(env.contains("PATH"));

        // And it should not contain any other env variables
        env.lines().for_each(|line| {
            let key = line.split('=').next().unwrap();
            // Stupid vars always present in subprocesses
            if ["PWD", "SHLVL", "GIT_TERMINAL_PROMPT", "_"].contains(&key) {
                return;
            }
            assert!(
                ALLOWED_ENV.contains(&key),
                "Unexpected env variable: {}",
                key
            );
        });
    }
}
