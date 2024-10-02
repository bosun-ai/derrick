mod workspace_controllers;
mod config;
mod github;
mod messaging;
mod repository;
pub mod service;
mod workspace;

pub use workspace_controllers::WorkspaceController;
pub use workspace::Workspace;

// Loads the global config async
pub fn config() -> &'static config::Config {
    config::Config::from_env()
}
