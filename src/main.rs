use anyhow::Result;
use clap::Parser;

use workspace_provider::{http_server, server};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let opts: Opts = Opts::parse();
    let provider = workspace_provider::get_provider(opts.provisioning_mode).await?;
    let workspace_config_path = opts.workspace_config_path;

    let context = workspace_provider::WorkspaceContext::from_file(workspace_config_path)?;
    let server = server::Server::create_server(context, provider)?;

    match opts.server_mode.as_str() {
        "nats" => {
            todo!()
        }
        "http" => http_server::serve_http(server).await,
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
