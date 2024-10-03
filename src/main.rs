use anyhow::Result;
use clap::Parser;

// The workspace provider is started with the following arguments:
// - provisioning mode (e.g. local, docker, cloud)
// - server mode (NATS or HTTP)
// - repositories and their target paths, and authentication tokens
// - a setup script

#[tokio::main]
async fn main() -> Result<()> {
    let opts: Opts = Opts::parse();
    let provider = workspace_provider::get_provider(opts.provisioning_mode).await?;
    let context = workspace_provider::WorkspaceContext {
        name: "test".to_string(),
        repositories: vec![],
        setup_script: "".to_string(),
    };
    let controller = provider.provision(context)?;
    controller.init().await?;
    Ok(())
}

#[derive(Parser, Debug)]
struct Opts {
    #[arg(short, long)]
    provisioning_mode: String,
}