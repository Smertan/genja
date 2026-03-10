//! Plugin type system and shared aliases for the plugin manager.
//!
//! Defines core plugin traits, type aliases, and registry structures used by
//! the plugin manager and plugin implementations.
//! This module also provides the `Plugins` enum to work with heterogeneous
//! plugin trait objects in a single collection.

use libloading::Library;
use serde::Deserialize;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

use genja_core::inventory::{Hosts, Inventory, TransformFunction};
use genja_core::InventoryLoadError;
use genja_core::task::{Task, Tasks};
/// Filesystem path to a plugin or plugin metadata entry.
pub type PathString = String;
/// Shared alias for a group name or plugin name key.
pub type GroupOrName = String;
/// Display name used to identify a plugin in the registry.
pub type PluginName = String;
/// Result of loading a plugin library and its exported plugin instances.
pub type PluginResult = Result<(Library, Vec<Box<dyn Plugin>>), Box<dyn std::error::Error>>;
/// Signature for a plugin factory function exported by dynamic libraries.
pub type PluginCreate = unsafe fn() -> Vec<Box<dyn Plugin>>;

/// Plugin entry in metadata, either a single path or a named group of paths.
#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum PluginEntry {
    Individual(PathString),
    Group(HashMap<String, PathString>),
}

/// Information about a loaded plugin, including the plugin itself and its group.
pub struct PluginInfo {
    pub plugin: Box<dyn Plugin>,
    pub group: Option<String>,
}

/// Manages the lifecycle of loaded plugins.
pub struct PluginManager {
    pub plugins: HashMap<String, PluginInfo>,
    // plugin_path: Vec<String>
    pub plugin_path: Vec<HashMap<GroupOrName, PluginEntry>>,
}

/// Base plugin interface implemented by all plugins.
///
/// Provides a name and an optional group label.
pub trait Plugin: Send + Sync + Any {
    /// The name of the plugin. This is used to identify the plugin.
    fn name(&self) -> String;

    /// Returns the group name
    fn group(&self) -> String {
        String::from("BasePlugin")
    }
}

/// Loads or prepares inventory data for the system.
///
/// Inventory plugins override the default inventory loading behavior provided
/// by the settings module. They provide the source of host data consumed by
/// runners and transforms. Implementations should be safe to call from multiple
/// threads and should avoid mutating shared state without synchronization.
pub trait PluginInventory: Plugin {
    /// Load and return inventory data for the system.
    fn load(&self) -> Result<Inventory, InventoryLoadError>;

    /// Returns the group name
    fn group(&self) -> String {
        String::from("InventoryPlugin")
    }
}

impl Debug for dyn Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {{ name: {} }}", Plugin::group(self), self.name())
    }
}

impl Debug for dyn PluginInventory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {{ name: {} }}",
            PluginInventory::group(self),
            self.name()
        )
    }
}

impl Debug for dyn PluginConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {{ name: {} }}",
            PluginConnection::group(self),
            self.name()
        )
    }
}

/// Executes tasks against a set of hosts.
///
/// Runner plugins provide task execution for a given inventory and task list.
/// Implementers should be safe to call from multiple threads and should avoid
/// mutating shared state without synchronization.
pub trait PluginRunner: Plugin {
    /// Run a single task against the provided hosts.
    fn run(&self, task: Task, hosts: &Hosts);

    /// Run all tasks in the provided task list against the provided hosts.
    fn run_tasks(&self, tasks: Tasks, hosts: &Hosts);

    /// Returns the group name
    fn group(&self) -> String {
        String::from("RunnerPlugin")
    }
}

impl Debug for dyn PluginRunner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {{ name: {} }}",
            PluginRunner::group(self),
            self.name()
        )
    }
}

/// Provides an inventory transform function.
///
/// Transform-function plugins supply a `TransformFunction` used to modify or
/// normalize inventory data during loading.
pub trait PluginTransformFunction: Plugin {
    /// Returns a transform function instance for inventory processing.
    fn transform_function(&self) -> TransformFunction;

    /// Returns the group name
    fn group(&self) -> String {
        String::from("TransformFunctionPlugin")
    }
}

impl Debug for dyn PluginTransformFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {{ name: {} }}",
            PluginTransformFunction::group(self),
            self.name()
        )
    }
}
/// Manages device connections for plugins that need an explicit session.
///
/// Connection plugins provide lifecycle hooks for establishing and tearing down
/// connections and expose a connection operation for downstream use.
pub trait PluginConnection: Plugin {
    /// Open a connection to a device.
    fn open(&self);

    /// Close a connection to a device.
    fn close(&self);

    /// Perform a connection operation (e.g., handshake or session check).
    fn connection(&self);

    /// Returns the group name
    fn group(&self) -> String {
        String::from("ConnectionPlugin")
    }
}

/// Heterogeneous container for supported plugin trait objects.
///
/// Each variant wraps a boxed trait object that implements a specific plugin
/// interface.
#[derive(Debug)]
pub enum Plugins {
    Connection(Box<dyn PluginConnection>),
    Inventory(Box<dyn PluginInventory>),
    Runner(Box<dyn PluginRunner>),
    TransformFunction(Box<dyn PluginTransformFunction>),
}

impl Plugins {
    /// Return the plugin's declared name.
    pub fn name(&self) -> String {
        match self {
            Plugins::Connection(connection) => connection.name(),
            Plugins::Inventory(inventory) => inventory.name(),
            Plugins::Runner(runner) => runner.name(),
            Plugins::TransformFunction(transform) => transform.name(),
        }
    }

    /// Return the logical group name for this plugin variant.
    pub fn group_name(&self) -> String {
        match self {
            Plugins::Connection(_) => String::from("Connection"),
            Plugins::Inventory(_) => String::from("Inventory"),
            Plugins::Runner(_) => String::from("Runner"),
            Plugins::TransformFunction(_) => String::from("TransformFunction"),
        }
    }

    // No shared execute hook. Use the specific plugin trait APIs instead.
}
