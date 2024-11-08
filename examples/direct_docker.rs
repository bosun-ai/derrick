//! # Direct Docker example
//!
//! This example demonstrates how to use the docker controller directly to run a container.
//!

use anyhow::Result;
use bollard::Docker;
use derrick::traits::{self, Workspace as WorkspaceTrait};
use derrick::Repository;
use derrick::{workspace_controllers::DockerController, Workspace};

#[tokio::main]
async fn main() -> Result<()> {
    let docker = Docker::connect_with_defaults()?;
    let controller = DockerController::start_with_mounts(
        &docker,
        "alpine",
        "derrick-direct-docker-example",
        vec![(".", "/root/test")],
    )
    .await?;

    let repository = Repository::from_url("https://github.com/bosun-ai/derrick.git").build()?;
    let workspace = Workspace::new(Box::new(controller), &repository);

    // workspace implements WorkspaceTrait, so we can call the functions defined by it on it.
    workspace
        .exec_cmd(&traits::Command::File(traits::FileCommands::Read {
            filename: "Cargo.toml".to_string(),
        }))
        .await?;

    Ok(())
}
