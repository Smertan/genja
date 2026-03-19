
//! Plugin type system and trait definitions for the plugin manager.
//!
//! This module defines the core plugin architecture used throughout the Genja plugin system.
//! It provides trait definitions for different plugin types, type aliases for common patterns,
//! and the `Plugins` enum for working with heterogeneous plugin collections.
//!
//! # Overview
//!
//! The plugin system is built around a hierarchy of traits that define different plugin
//! capabilities:
//!
//! ```text
//! ┌───────────────────────────────────────────────────────────────┐
//! │                        Plugin (Base)                          │
//! │                   - name() -> String                          │
//! │                   - group() -> String                         │
//! └───────────────────────────┬───────────────────────────────────┘
//!                             │
//!           ┌─────────────────┼─────────────────┬────────────────┐
//!           │                 │                 │                │
//!           ▼                 ▼                 ▼                ▼
//! ┌────────────────┐  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐
//! │PluginConnection│  │PluginInventory │  │ PluginRunner   │  │PluginTransform │
//! │                │  │                │  │                │  │  Function      │
//! │ - create()     │  │ - load()       │  │ - run()        │  │ - transform_   │
//! │ - open()       │  │                │  │ - run_tasks()  │  │   function()   │
//! │ - close()      │  │                │  │                │  │                │
//! │ - is_alive()   │  │                │  │                │  │                │
//! └────────────────┘  └────────────────┘  └────────────────┘  └────────────────┘
//! ```
//!
//! # Plugin Types
//!
//! ## Base Plugin Trait
//!
//! All plugins must implement the [`Plugin`] trait, which provides:
//! - A unique name for identification
//! - A group classification for organizational purposes
//!
//! ## Specialized Plugin Traits
//!
//! ### [`PluginConnection`]
//! Manages device connections with lifecycle hooks for establishing and tearing down
//! sessions. Used for protocols like SSH, Telnet, NETCONF, etc.
//!
//! **Key Methods:**
//! - `create()` - Create new connection instances per host
//! - `open()` - Establish connection with resolved parameters
//! - `close()` - Tear down connection and cleanup resources
//! - `is_alive()` - Check connection health status
//!
//! ### [`PluginInventory`]
//! Loads and prepares inventory data from various sources. Overrides default
//! inventory loading behavior.
//!
//! **Key Methods:**
//! - `load()` - Load inventory from source (files, APIs, databases, etc.)
//!
//! ### [`PluginRunner`]
//! Executes tasks against sets of hosts. Provides different execution strategies
//! (sequential, parallel, etc.).
//!
//! **Key Methods:**
//! - `run()` - Execute a single task
//! - `run_tasks()` - Execute multiple tasks in sequence
//!
//! ### [`PluginTransformFunction`]
//! Provides inventory transformation functions for normalizing or modifying
//! inventory data during loading.
//!
//! **Key Methods:**
//! - `transform_function()` - Returns the transform function implementation
//!
//! # Type Aliases
//!
//! The module provides several type aliases for common patterns:
//!
//! - [`PathString`] - Filesystem path to a plugin library
//! - [`GroupOrName`] - Plugin name or group identifier
//! - [`PluginName`] - Display name for plugin identification
//! - [`PluginResult`] - Result type for plugin loading operations
//! - [`PluginCreate`] - Factory function signature for plugin creation
//!
//! # The Plugins Enum
//!
//! The [`Plugins`] enum provides a heterogeneous container for different plugin types,
//! allowing them to be stored in a single collection:
//!
//! ```rust
//! use plugin_manager::plugin_types::Plugins;
//!
//! // Store different plugin types in a single vector
//! let plugins: Vec<Plugins> = vec![
//!     // Plugins::Connection(Box::new(ssh_plugin)),
//!     // Plugins::Inventory(Box::new(file_plugin)),
//!     // Plugins::Runner(Box::new(threaded_runner)),
//! ];
//! ```
//!
//! # Plugin Metadata
//!
//! ## PluginEntry
//!
//! The [`PluginEntry`] enum represents plugin configuration in metadata:
//!
//! ```toml
//! # Individual plugin
//! [package.metadata.plugins]
//! ssh_plugin = "/path/to/libssh_plugin.so"
//!
//! # Grouped plugins
//! [package.metadata.plugins.network]
//! ssh = "/path/to/libssh.so"
//! telnet = "/path/to/libtelnet.so"
//! ```
//!
//! ## PluginInfo
//!
//! The [`PluginInfo`] struct combines a plugin instance with its optional group:
//!
//! ```rust
//! use plugin_manager::plugin_types::PluginInfo;
//!
//! // let info = PluginInfo {
//! //     plugin: Box::new(my_plugin),
//! //     group: Some("network".to_string()),
//! // };
//! ```
//!
//! # Usage Examples
//!
//! ## Implementing a Connection Plugin
//!
//! ```rust
//! use plugin_manager::plugin_types::{Plugin, PluginConnection};
//! use genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
//!
//! #[derive(Debug)]
//! struct SshPlugin {
//!     key: ConnectionKey,
//!     connected: bool,
//! }
//!
//! impl Plugin for SshPlugin {
//!     fn name(&self) -> String {
//!         "ssh".to_string()
//!     }
//! }
//!
//! impl PluginConnection for SshPlugin {
//!     fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
//!         Box::new(SshPlugin {
//!             key: key.clone(),
//!             connected: false,
//!         })
//!     }
//!
//!     fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String> {
//!         // Establish SSH connection
//!         self.connected = true;
//!         Ok(())
//!     }
//!
//!     fn close(&mut self) -> ConnectionKey {
//!         // Clean up SSH connection
//!         self.connected = false;
//!         self.key.clone()
//!     }
//!
//!     fn is_alive(&self) -> bool {
//!         self.connected
//!     }
//! }
//! ```
//!
//! ## Implementing an Inventory Plugin
//!
//! ```rust
//! use plugin_manager::plugin_types::{Plugin, PluginInventory};
//! use plugin_manager::PluginManager;
//! use genja_core::{Settings, InventoryLoadError};
//! use genja_core::inventory::Inventory;
//!
//! #[derive(Debug)]
//! struct DatabaseInventoryPlugin;
//!
//! impl Plugin for DatabaseInventoryPlugin {
//!     fn name(&self) -> String {
//!         "database_inventory".to_string()
//!     }
//! }
//!
//! impl PluginInventory for DatabaseInventoryPlugin {
//!     fn load(
//!         &self,
//!         settings: &Settings,
//!         plugins: &PluginManager,
//!     ) -> Result<Inventory, InventoryLoadError> {
//!         // Load inventory from database
//!         // let inventory = fetch_from_database(settings)?;
//!         // Ok(inventory)
//!         unimplemented!()
//!     }
//! }
//! ```
//!
//! ## Implementing a Runner Plugin
//!
//! ```rust
//! use plugin_manager::plugin_types::{Plugin, PluginRunner};
//! use genja_core::inventory::Hosts;
//! use genja_core::task::{Task, Tasks};
//!
//! #[derive(Debug)]
//! struct SequentialRunner;
//!
//! impl Plugin for SequentialRunner {
//!     fn name(&self) -> String {
//!         "sequential".to_string()
//!     }
//! }
//!
//! impl PluginRunner for SequentialRunner {
//!     fn run(&self, task: Task, hosts: &Hosts) {
//!         // Execute task sequentially on each host
//!         for (name, host) in hosts.iter() {
//!             // Execute task on host
//!         }
//!     }
//!
//!     fn run_tasks(&self, tasks: Tasks, hosts: &Hosts) {
//!         // Execute all tasks sequentially
//!         for task in tasks.iter() {
//!             self.run(task.clone(), hosts);
//!         }
//!     }
//! }
//! ```
//!
//! ## Implementing a Transform Function Plugin
//!
//! ```rust
//! use plugin_manager::plugin_types::{Plugin, PluginTransformFunction};
//! use genja_core::inventory::{TransformFunction, Host, BaseBuilderHost};
//!
//! #[derive(Debug)]
//! struct NormalizeHostnamePlugin;
//!
//! impl Plugin for NormalizeHostnamePlugin {
//!     fn name(&self) -> String {
//!         "normalize_hostname".to_string()
//!     }
//! }
//!
//! impl PluginTransformFunction for NormalizeHostnamePlugin {
//!     fn transform_function(&self) -> TransformFunction {
//!         TransformFunction::new(|host: &Host, _options| {
//!             // Normalize hostname to lowercase
//!             if let Some(hostname) = host.hostname() {
//!                 host.to_builder().hostname(hostname.to_lowercase()).build()
//!             } else {
//!                 host.clone()
//!             }
//!         })
//!     }
//! }
//! ```
//!
//! ## Working with the Plugins Enum
//!
//! ```rust
//! use plugin_manager::plugin_types::Plugins;
//!
//! fn process_plugin(plugin: &Plugins) {
//!     match plugin {
//!         Plugins::Connection(conn) => {
//!             println!("Connection plugin: {}", conn.name());
//!         }
//!         Plugins::Inventory(inv) => {
//!             println!("Inventory plugin: {}", inv.name());
//!         }
//!         Plugins::Runner(runner) => {
//!             println!("Runner plugin: {}", runner.name());
//!         }
//!         Plugins::TransformFunction(tf) => {
//!             println!("Transform function plugin: {}", tf.name());
//!         }
//!     }
//! }
//! ```
//!
//! # Plugin Factory Functions
//!
//! Plugins are created through factory functions exported from dynamic libraries:
//!
//! ```rust
//! use plugin_manager::plugin_types::Plugins;
//!
//! #[unsafe(no_mangle)]
//! pub fn create_plugins() -> Vec<Plugins> {
//!     vec![
//!         // Plugins::Connection(Box::new(SshPlugin::new())),
//!         // Plugins::Runner(Box::new(SequentialRunner)),
//!     ]
//! }
//!

use libloading::Library;
use serde::Deserialize;
use std::any::Any;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;

use crate::PluginManager;
use genja_core::inventory::{
    ConnectionKey, Hosts, Inventory, ResolvedConnectionParams, TransformFunction,
};
use genja_core::task::{Task, Tasks};
use genja_core::{InventoryLoadError, Settings};
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

/// Signature for a plugin factory function exported by dynamic libraries.
pub type PluginCreatePlugins = unsafe fn() -> Vec<Plugins>;
/// Result of loading a plugin library and its exported plugin instances.
pub type PluginResultPlugins = Result<(Library, Vec<Plugins>), Box<dyn std::error::Error>>;

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
    fn load(
        &self,
        settings: &Settings,
        plugins: &PluginManager,
    ) -> Result<Inventory, InventoryLoadError>;

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
    /// Create a new per-host connection instance.
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection>;

    /// Open a connection to a device.
    fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String>;

    /// Close a connection to a device.
    fn close(&mut self) -> ConnectionKey;

    /// Returns `true` if the connection is alive.
    fn is_alive(&self) -> bool;

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
}
