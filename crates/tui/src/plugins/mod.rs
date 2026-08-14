#![allow(dead_code)]

pub mod activation;
pub mod agent_plugin;
pub mod context;
pub mod discovery;
pub mod export;
pub mod install;
pub mod manifest;
pub mod mutation;
mod path_identity;
pub mod registry;
pub mod types;

#[cfg(test)]
mod tests;

pub use context::{HostEnvironment, PluginDiscoveryContext};
pub use registry::PluginRegistry;
