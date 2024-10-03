mod workspace_controllers;
mod workspace_providers;
mod config;
mod github;
mod messaging;
mod repository;
pub mod service;
mod workspace;

pub use workspace_controllers::WorkspaceController;
pub use workspace_providers::{WorkspaceProvider, WorkspaceContext};
pub use workspace::Workspace;
pub use workspace_providers::get_provider;

// Loads the global config async
pub fn config() -> &'static config::Config {
    config::Config::from_env()
}
