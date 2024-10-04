use anyhow::Result;
use clap::Parser;

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let provider = workspace_provider::get_provider(opts.provisioning_mode).await?;
    let workspace_config_path = opts.workspace_config_path;

    let context = workspace_provider::WorkspaceContext::from_file(workspace_config_path)?;

    match opts.server_mode.as_str() {
        "nats" => {
            todo!()
        }
        "http" => {
            todo!()
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Unsupported server mode: {}",
                opts.server_mode
            ))
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Opts {
    /// The provisioning mode to use (local, docker, remote_nats)
    #[arg(short, long)]
    provisioning_mode: String,
    /// The path to the workspace configuration file
    #[arg(short, long)]
    workspace_config_path: String,
    /// The server mode to use (nats, http)
    #[arg(short, long)]
    server_mode: String,
}
