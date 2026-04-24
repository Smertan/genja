//! Plugin integration layer for Genja.
//!
//! This module bridges Genja Core settings with the plugin system. Built-in
//! plugins are grouped by responsibility so inventory loading and task runner
//! execution can evolve independently.

mod inventory;
mod runners;

pub use inventory::DefaultInventoryPlugin;
pub use runners::{SerialRunnerPlugin, ThreadedRunnerPlugin};

use genja_plugin_manager::PluginManager;
use genja_plugin_manager::plugin_types::Plugins;

pub(crate) fn built_in_plugin_manager() -> PluginManager {
    let mut manager = PluginManager::new();
    manager.register_plugin(Plugins::Inventory(Box::new(DefaultInventoryPlugin)));
    manager.register_plugin(Plugins::Runner(Box::new(SerialRunnerPlugin)));
    manager.register_plugin(Plugins::Runner(Box::new(ThreadedRunnerPlugin)));
    manager
}
