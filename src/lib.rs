pub mod adapters;
pub mod config;
pub mod github;
pub mod messaging;
pub mod repository;
pub mod service;
pub mod workspace;

pub use adapters::Adapter;
pub use workspace::Workspace;

// Loads the global config async
pub fn config() -> &'static config::Config {
    config::Config::from_env()
}
