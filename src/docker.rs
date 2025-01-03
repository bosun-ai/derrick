use std::time::Duration;

use anyhow::{anyhow, Result};
use bollard::Docker;

pub async fn establish_connection() -> Result<Docker> {
    // if windows or linux we connect with socket defaults
    if cfg!(target_os = "windows") || cfg!(target_os = "linux") {
        Docker::connect_with_socket_defaults()
            .map_err(Into::into)
            .map(|docker| docker.with_timeout(Duration::from_secs(60 * 15)))
    } else if cfg!(target_os = "macos") {
        let username = whoami::username();
        let macos_socket_path = format!("unix:///Users/{}/.docker/run/docker.sock", username);
        Docker::connect_with_socket(&macos_socket_path, 5, bollard::API_DEFAULT_VERSION)
            .map_err(Into::into)
    } else {
        Err(anyhow!("Unsupported OS"))
    }
}
