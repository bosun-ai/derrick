use crate::workspace_controllers::{CommandOutput, WorkspaceController};
use anyhow::{Context, Result};
use async_trait::async_trait;
use rand::Rng;
use std::process::Command;
use std::time::Duration;
use std::{collections::HashMap, fmt::Debug};
use tracing::{debug, warn};

// Runs commands in a local temporary directory
// Useful for debugging, testing and experimentation
//
// NOTE:
//  - might be useful to drop the directory after out of scope
//  - haven't decided what to do with stdout/stderr
#[derive(Debug)]
pub struct TestingController {
    path: String,
}

fn init_path(name: &str) -> Result<String> {
    // Use the tmp directory with a random directory name
    let mut temp_dir = std::env::temp_dir();
    let mut rng = rand::thread_rng();
    let path = format!("{}-{}", name, rng.gen::<u64>());
    temp_dir.push(path);

    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir).context("Could not create local temp directory")?;
    }
    Ok(temp_dir
        .canonicalize()?
        .to_str()
        .context("Could not convert to string")?
        .to_string())
}

impl TestingController {
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        let path = init_path(name)
            .context("Could not create local temp directory")
            .unwrap();
        Self { path }
    }

    #[tracing::instrument(skip(self), name = "TestingAdapter#spawn_cmd")]
    fn spawn_cmd(
        &self,
        cmd: &str,
        _working_dir: Option<&str>,
        _env: HashMap<String, String>,
    ) -> Result<std::process::Output> {
        // Never push in tests
        if cmd.contains("git push") {
            return Ok(std::process::Output {
                status: std::process::ExitStatus::default(),
                stdout: Vec::new(),
                stderr: Vec::new(),
            });
        }

        Command::new("bash")
            .args(["-c", cmd])
            .env_clear()
            .env("GIT_AUTHOR_NAME", "fluyt-test")
            .env("GIT_AUTHOR_EMAIL", "fluyt@bosun.ai")
            .env("GIT_COMMITTER_NAME", "fluyt-test")
            .env("GIT_COMMITTER_EMAIL", "fluyt@bosun.ai")
            .env("GIT_TERMINAL_PROMPT", "0")
            .current_dir(&self.path)
            .output()
            .context("Could not run command")
    }
}

// Clean up the temporary directory when the adapter is dropped
impl Drop for TestingController {
    #[tracing::instrument]
    fn drop(&mut self) {
        warn!(path = &self.path, "Deleting local temp directory");
        std::fs::remove_dir_all(&self.path).unwrap();
    }
}

#[async_trait]
impl WorkspaceController for TestingController {
    #[tracing::instrument]
    async fn init(&self) -> Result<()> {
        warn!(path = &self.path, "Creating local temp directory");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        todo!();
    }

    #[tracing::instrument(skip(self), name = "TestingAdapter#cmd")]
    async fn cmd(
        &self,
        cmd: &str,
        _working_dir: Option<&str>,
        _env: HashMap<String, String>,
        _timeout: Option<Duration>,
    ) -> Result<()> {
        self.spawn_cmd(cmd, _working_dir, _env)
            .map(handle_command_result)
            .context("Could not run command")?
            .map(|_| ())
    }

    #[tracing::instrument(skip(self), name = "TestingAdapter#cmd_with_output")]
    async fn cmd_with_output(
        &self,
        cmd: &str,
        _working_dir: Option<&str>,
        _env: HashMap<String, String>,
        _timeout: Option<Duration>,
    ) -> Result<CommandOutput> {
        self.spawn_cmd(cmd, _working_dir, _env)
            .map(handle_command_result)?
    }

    async fn write_file(
        &self,
        file: &str,
        content: &[u8],
        _working_dir: Option<&str>,
    ) -> Result<()> {
        std::fs::write(format!("{}/{}", &self.path, file), content).context("Could not write file")
    }

    async fn read_file(&self, file: &str, _working_dir: Option<&str>) -> Result<Vec<u8>> {
        self.cmd_with_output(&format!("cat {}", file), None, HashMap::new(), None)
            .await
            .map(|output| output.output.as_bytes().to_vec())
    }

    #[tracing::instrument(skip_all)]
    async fn provision_repositories(
        &self,
        _repositories: Vec<crate::repository::Repository>,
    ) -> Result<()> {
        todo!()
    }
}

#[tracing::instrument]
fn handle_command_result(result: std::process::Output) -> Result<CommandOutput> {
    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    if result.status.success() {
        debug!(stdout = &stdout, stderr = &stderr, "Command succeeded");
        Ok(CommandOutput {
            output: stdout,
            exit_code: result.status.code().unwrap_or(0),
        })
    } else {
        warn!(stdout = &stdout, stderr = &stderr, "Command failed");
        Err(anyhow::anyhow!(
            "Command failed with status: {}",
            result.status
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cmd_with_output() {
        let adapter = TestingController::new("test");
        adapter.init().await.unwrap();
        let result = adapter
            .cmd_with_output("pwd", None, HashMap::new(), None)
            .await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("test"));
        assert_eq!(output.exit_code, 0);
    }

    #[tokio::test]
    async fn test_sets_path_correctly_for_run_cmd() {
        let adapter = TestingController::new("test");
        adapter.init().await.unwrap();
        let output = adapter.spawn_cmd("pwd", None, HashMap::new()).unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        assert!(stdout.contains("test"));
    }

    #[test]
    fn test_init_path() {
        let path = init_path("test").unwrap();
        dbg!(&path);
        let regex = regex::Regex::new(r"test-\d+$").unwrap();
        assert!(regex.is_match(&path));
        assert!(std::path::PathBuf::from(&path).exists())
    }

    #[tokio::test]
    async fn test_init() {
        let adapter = TestingController::new("test");
        let result = adapter.init().await;
        assert!(result.is_ok());
        let path = std::path::Path::new(&adapter.path);
        assert!(path.exists());
    }

    #[tokio::test]
    async fn test_cmd_valid() {
        let adapter = TestingController::new("test");
        adapter.init().await.unwrap();
        let result = adapter.cmd("ls", None, HashMap::new(), None).await;
        println!("{:#?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_piping_a_command() {
        let adapter = TestingController::new("test");
        adapter.init().await.unwrap();
        adapter
            .cmd("echo 'hello' > test.txt", None, HashMap::new(), None)
            .await
            .unwrap();
        // check if file was created
        let result = adapter
            .cmd("cat test.txt | grep 'hello'", None, HashMap::new(), None)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_writing_file() {
        let adapter = TestingController::new("test");
        adapter.init().await.unwrap();
        adapter
            .write_file("test.txt", b"Hello, world!", None)
            .await
            .expect("Could not write file");
        let result = adapter
            .cmd_with_output("cat test.txt", None, HashMap::new(), None)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().output, "Hello, world!");

        adapter
            .write_file("test.txt", b"Hello, back!", None)
            .await
            .unwrap();
        let result = adapter
            .cmd_with_output("cat test.txt", None, HashMap::new(), None)
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().output, "Hello, back!");
    }
}
