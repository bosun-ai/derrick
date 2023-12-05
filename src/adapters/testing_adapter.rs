use crate::adapters::Adapter;
use anyhow::{Context, Result};
use async_trait::async_trait;
use rand::Rng;
use std::fmt::Debug;
use std::process::Command;
use tracing::{debug, warn};

// Runs commands in a local temporary directory
// Useful for debugging, testing and experimentation
//
// NOTE:
//  - might be useful to drop the directory after out of scope
//  - haven't decided what to do with stdout/stderr
#[derive(Debug)]
pub struct TestingAdapter {
    name: String,
    path: String,
}

impl TestingAdapter {
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        let path = init_path(name)
            .context("Could not create local temp directory")
            .unwrap();
        Self {
            name: name.into(),
            path,
        }
    }

    #[tracing::instrument(skip(self), name = "TestingAdapter#spawn_cmd")]
    fn spawn_cmd(&self, cmd: &str) -> Result<std::process::Output> {
        Command::new("bash")
            .args(["-c", cmd])
            .current_dir(&self.path)
            .output()
            .context("Could not run command")
    }
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

// Clean up the temporary directory when the adapter is dropped
impl Drop for TestingAdapter {
    #[tracing::instrument]
    fn drop(&mut self) {
        warn!(path = &self.path, "Deleting local temp directory");
        std::fs::remove_dir_all(&self.path).unwrap();
    }
}

#[async_trait]
impl Adapter for TestingAdapter {
    #[tracing::instrument]
    fn init(&self) -> Result<()> {
        warn!(path = &self.path, "Creating local temp directory");
        Ok(())
    }

    #[tracing::instrument(skip(self), name = "TestingAdapter#cmd")]
    fn cmd(&self, cmd: &str) -> Result<()> {
        self.spawn_cmd(cmd)
            .map(handle_command_result)
            .context("Could not run command")?
            .map(|_| ())
    }

    #[tracing::instrument(skip(self), name = "TestingAdapter#cmd_with_output")]
    fn cmd_with_output(&self, cmd: &str) -> Result<String> {
        self.spawn_cmd(cmd)
            .map(handle_command_result)?
            .context("Could not run command")
    }

    fn debug(&self) -> String {
        format!("TestingAdapter{{name: {}, path: {}}}", self.name, self.path)
    }

    fn write_file(&self, file: &str, content: &str) -> Result<()> {
        std::fs::write(format!("{}/{}", &self.path, file), content).context("Could not write file")
    }
}

#[tracing::instrument]
fn handle_command_result(result: std::process::Output) -> Result<String> {
    let stdout = String::from_utf8_lossy(&result.stdout).to_string();
    let stderr = String::from_utf8_lossy(&result.stderr).to_string();
    if result.status.success() {
        debug!(stdout = &stdout, stderr = &stderr, "Command succeeded");
        Ok(stdout)
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

    #[test]
    fn test_cmd_with_output() {
        let adapter = TestingAdapter::new("test");
        adapter.init().unwrap();
        let result = adapter.cmd_with_output("pwd");
        assert!(result.is_ok());
        let stdout = result.unwrap();
        assert!(stdout.contains("tmp/test"));
    }

    #[test]
    fn test_sets_path_correctly_for_run_cmd() {
        let adapter = TestingAdapter::new("test");
        adapter.init().unwrap();
        let output = adapter.spawn_cmd("pwd").unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        assert!(stdout.contains("tmp/test"));
    }

    #[test]
    fn test_init_path() {
        let path = init_path("test").unwrap();
        dbg!(&path);
        let regex = regex::Regex::new(r"^.*/tmp/test.*$").unwrap();
        assert!(regex.is_match(&path));
        assert!(std::path::PathBuf::from(&path).exists())
    }

    #[test]
    fn test_init() {
        let adapter = TestingAdapter::new("test");
        let result = adapter.init();
        assert!(result.is_ok());
        let path = std::path::Path::new(&adapter.path);
        assert!(path.exists());
    }

    #[test]
    fn test_cmd_valid() {
        let adapter = TestingAdapter::new("test");
        adapter.init().unwrap();
        let result = adapter.cmd("ls");
        println!("{:#?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_piping_a_command() {
        let adapter = TestingAdapter::new("test");
        adapter.init().unwrap();
        adapter.cmd("echo 'hello' > test.txt").unwrap();
        // check if file was created
        let result = adapter.cmd("cat test.txt | grep 'hello'");
        assert!(result.is_ok());
    }

    #[test]
    fn test_writing_file() {
        let adapter = TestingAdapter::new("test");
        adapter.init().unwrap();
        adapter
            .write_file("test.txt", "Hello, world!")
            .expect("Could not write file");
        let result = adapter.cmd_with_output("cat test.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");

        adapter.write_file("test.txt", "Hello, back!").unwrap();
        let result = adapter.cmd_with_output("cat test.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, back!");
    }
}
