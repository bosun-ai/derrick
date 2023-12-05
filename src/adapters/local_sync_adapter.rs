use crate::adapters::Adapter;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::fmt::Debug;
use std::process::Command;
use std::sync::OnceLock;
use tracing::{debug, warn};

// Runs commands in a local temporary directory
// Useful for debugging, testing and experimentation
//
// NOTE:
//  - might be useful to drop the directory after out of scope
//  - haven't decided what to do with stdout/stderr
pub struct LocalTempSync {
    name: String,
    path: OnceLock<String>,
}

impl Debug for LocalTempSync {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "LocalTempSync")
    }
}

impl LocalTempSync {
    #[tracing::instrument]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            path: OnceLock::new(),
        }
    }

    #[tracing::instrument]
    fn spawn_cmd(&self, cmd: &str) -> std::result::Result<std::process::Output, std::io::Error> {
        Command::new("bash")
            .args(["-c", cmd])
            .current_dir(self.path())
            .output()
    }

    fn path(&self) -> &str {
        self.path
            .get()
            .context("Expected path to be set, workspace not initialized?")
            .unwrap()
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
    #[tracing::instrument]
    fn init(&self) -> Result<()> {
        self.path.get_or_init(|| {
            init_path(&self.name)
                .context("Could not create local temp directory")
                .unwrap()
        });
        warn!(path = &self.path.get(), "Creating local temp directory");
        Ok(())
    }

    #[tracing::instrument]
    fn cmd(&self, cmd: &str) -> Result<()> {
        self.spawn_cmd(cmd)
            .map(handle_command_result)?
            .map(|_| ())
            .context("Could not run command")
    }

    #[tracing::instrument]
    fn cmd_with_output(&self, cmd: &str) -> Result<String> {
        self.spawn_cmd(cmd).map(handle_command_result)?
    }

    fn debug(&self) -> String {
        format!(
            "LocalTempSync{{name: {}, path: {}}}",
            self.name,
            self.path.get().map(|v| v.as_str()).unwrap_or("not set")
        )
    }

    #[tracing::instrument]
    fn write_file(&self, file: &str, content: &str) -> Result<()> {
        std::fs::write(format!("{}/{}", &self.path(), file), content)
            .context("Could not write file")
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
        Err(anyhow::anyhow!(stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_with_output() {
        let adapter = LocalTempSync::new("test");
        adapter.init().unwrap();
        let result = adapter.cmd_with_output("pwd");
        assert!(result.is_ok());
        let stdout = result.unwrap();
        assert!(stdout.contains("tmp/test"));
    }

    #[test]
    fn test_sets_path_correctly_for_run_cmd() {
        let adapter = LocalTempSync::new("test");
        adapter.init().unwrap();
        let output = adapter.spawn_cmd("pwd").unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        assert!(stdout.contains("tmp/test"));
    }

    #[test]
    fn test_init_path() {
        let path = init_path("test").unwrap();
        dbg!(&path);
        let regex = regex::Regex::new(r"^.*/tmp/test$").unwrap();
        assert!(regex.is_match(&path));
        assert!(std::path::PathBuf::from(&path).exists())
    }

    #[test]
    fn test_init() {
        let adapter = LocalTempSync::new("test");
        let result = adapter.init();
        assert!(result.is_ok());
        let str_path = adapter.path().to_string();
        let path = std::path::Path::new(&str_path);
        assert!(path.exists());
    }

    #[test]
    fn test_cmd_valid() {
        let adapter = LocalTempSync::new("test");
        adapter.init().unwrap();
        let result = adapter.cmd("ls");
        println!("{:#?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_cmd_invalid() {
        let adapter = LocalTempSync::new("test");
        adapter.init().unwrap();
        let result = adapter.cmd("invalid command");
        assert!(result.is_err());
    }

    #[test]
    fn test_piping_a_command() {
        let adapter = LocalTempSync::new("test");
        adapter.init().unwrap();
        adapter.cmd("echo 'hello' > test.txt").unwrap();
        // check if file was created
        let result = adapter.cmd("cat test.txt | grep 'hello'");
        dbg!(&result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_writing_file() {
        let adapter = LocalTempSync::new("test");
        adapter.init().unwrap();
        adapter
            .write_file("write.txt", "Hello, world!")
            .expect("Could not write file");
        let result = adapter.cmd_with_output("cat write.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, world!");

        adapter.write_file("write.txt", "Hello, back!").unwrap();
        let result = adapter.cmd_with_output("cat write.txt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello, back!");
    }
}
