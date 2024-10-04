mod config;
mod github;
mod messaging;
mod repository;
mod server;
pub mod service;
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
