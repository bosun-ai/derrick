pub struct WorkspaceProvider {
    adapter: Box<dyn Adapter>,
    repositories: Vec<Repository>,
    setup_script: String,
}

impl WorkspaceProvider {
    #[tracing::instrument(skip_all)]
    fn new(adapter: Box<dyn Adapter>, repositories: Vec<Repository>, setup_script: String) -> Self {
        Self {
            adapter,
            repositories,
            setup_script,
        }
    }

    #[tracing::instrument(skip_all, fields(bosun.tracing=true), name = "workspace_provider.provision")]
    async fn provision(&self) -> Result<()> {
        info!("Provisioning workspace");
        self.adapter
        Ok(())
    }
}
