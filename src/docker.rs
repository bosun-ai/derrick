use bollard::Docker;
use anyhow::{anyhow, Result};

pub async fn establish_connection() -> Result<Docker> {
  // if windows or linux we connect with socket defaults
  if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
    Docker::connect_with_socket_defaults().map_err(Into::into)
  } else if cfg!(target_os = "macos") {
      let username = whoami::username();
      let macos_socket_path = format!("unix:///Users/{}/.docker/run/docker.sock", username);
      Docker::connect_with_socket(&macos_socket_path, 5, bollard::API_DEFAULT_VERSION).map_err(Into::into)
  } else {
      Err(anyhow!("Unsupported OS"))
  }
}