mod config;
mod docker;
mod github;
pub mod http_server;
mod messaging;
mod repository;
pub mod server;
pub mod service;
pub mod traits;
mod workspace;
mod workspace_controllers;
mod workspace_providers;

pub use workspace::Workspace;
pub use workspace_controllers::WorkspaceController;
pub use workspace_providers::get_provider;
pub use workspace_providers::{WorkspaceContext, WorkspaceProvider};

// Loads the global config async
pub fn config() -> &'static config::Config {
    config::Config::from_env()
}
