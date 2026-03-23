//! # Genja Plugin Manager
//!
//! A flexible and easy-to-use plugin management system for Rust applications.
//!
//! This crate provides dynamic loading, registration, and management of plugins at runtime.
//! It supports individual plugins and grouped plugins, making it suitable for various
//! application architectures where extensibility is required.
//!
//! ## Overview
//!
//! The plugin manager enables building modular applications where functionality can be
//! added through plugins without recompilation. It handles:
//!
//! - Dynamic loading of plugins from shared libraries (.so, .dll, .dylib)
//! - Plugin lifecycle management (registration, deregistration)
//! - Type-safe plugin registry access
//! - Metadata-driven plugin configuration
//! - Support for multiple plugin types (Connection, Inventory, Runner, Transform)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                      PluginManager                              │
//! │  - Loads plugins from shared libraries                          │
//! │  - Maintains plugin registry                                    │
//! │  - Provides type-safe access to plugins                         │
//! └────────────────┬────────────────────────────────────────────────┘
//!                  │
//!                  │ manages
//!                  │
//!                  ▼
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                         Plugins Enum                            │
//! │  - Connection(Box<dyn PluginConnection>)                        │
//! │  - Inventory(Box<dyn PluginInventory>)                          │
//! │  - Runner(Box<dyn PluginRunner>)                                │
//! │  - TransformFunction(Box<dyn PluginTransformFunction>)          │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Quick Start
//!
//! ### Creating a Plugin
//!
//! ```rust
//! use genja_core::inventory::Hosts;
//! use genja_core::task::{Task, Tasks};
//! use genja_plugin_manager::plugin_types::{Plugin, PluginRunner, Plugins};
//!
//! #[derive(Debug)]
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn name(&self) -> String {
//!         "my_plugin".to_string()
//!     }
//! }
//!
//! impl PluginRunner for MyPlugin {
//!     fn run(&self, _task: Task, _hosts: &Hosts) {
//!         // Task execution logic
//!     }
//!
//!     fn run_tasks(&self, _tasks: Tasks, _hosts: &Hosts) {
//!         // Batch task execution logic
//!     }
//! }
//!
//! // Export plugin factory function
//! #[unsafe(no_mangle)]
//! pub fn create_plugins() -> Vec<Plugins> {
//!     vec![Plugins::Runner(Box::new(MyPlugin))]
//! }
//! ```
//!
//! ### Using the Plugin Manager
//!
//! ```rust
//! # unsafe {
//! #     std::env::set_var("CARGO_MANIFEST_PATH", "../tests/plugin_mods/Cargo.toml");
//! # }
//! use genja_plugin_manager::PluginManager;
//!
//! # fn doc_test() -> Result<(), Box<dyn std::error::Error>> {
//! // Create and activate plugins
//! let mut plugin_manager = PluginManager::new();
//! plugin_manager = plugin_manager.activate_plugins()?;
//!
//! // Access plugins by type
//! if let Some(runner) = plugin_manager.get_runner_plugin("my_plugin") {
//!     // Use the runner plugin
//! }
//!
//! // List all plugins
//! let all_plugins = plugin_manager.get_all_plugin_names_and_groups();
//! for (name, group) in all_plugins {
//!     println!("Plugin: {} ({})", name, group);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Plugin Configuration
//!
//! Plugins are configured in the `Cargo.toml` file of the end-user project using
//! package metadata. You can register plugins as individual entries or grouped
//! by plugin type (e.g., `inventory`, `connection`, `runner`, `transform`):
//!
//! ```toml
//! # Individual plugins
//! [package.metadata.plugins]
//! my_plugin = "/path/to/libmy_plugin.so"
//!
//! # Grouped plugins
//! [package.metadata.plugins.network]
//! ssh = "/path/to/libssh.so"
//! telnet = "/path/to/libtelnet.so"
//!
//! # Grouped by plugin type (recommended)
//! [package.metadata.plugins.inventory]
//! inventory_a = "/path/to/libinventory.so"
//!
//! [package.metadata.plugins.connection]
//! ssh = "/path/to/libssh.so"
//! netconf = "/path/to/libnetconf.so"
//!
//! [package.metadata.plugins.runner]
//! threaded = "/path/to/libthreaded.so"
//!
//! [package.metadata.plugins.transform]
//! normalize = "/path/to/libnormalize.so"
//! ```
//!
//! ## Plugin Types
//!
//! ### Connection Plugins
//!
//! Manage device connections with lifecycle hooks:
//!
//! ```rust
//! use genja_plugin_manager::plugin_types::{Plugin, PluginConnection};
//! use genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
//!
//! #[derive(Debug)]
//! struct SshPlugin {
//!     key: ConnectionKey,
//!     connected: bool,
//! }
//!
//! impl Plugin for SshPlugin {
//!     fn name(&self) -> String { "ssh".to_string() }
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
//!         // Establish connection
//!         self.connected = true;
//!         Ok(())
//!     }
//!
//!     fn close(&mut self) -> ConnectionKey {
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
//! ### Inventory Plugins
//!
//! Load inventory data from various sources:
//!
//! ```rust
//! use genja_plugin_manager::plugin_types::{Plugin, PluginInventory};
//! use genja_plugin_manager::PluginManager;
//! use genja_core::{Settings, InventoryLoadError};
//! use genja_core::inventory::Inventory;
//!
//! #[derive(Debug)]
//! struct DatabaseInventoryPlugin;
//!
//! impl Plugin for DatabaseInventoryPlugin {
//!     fn name(&self) -> String { "database_inventory".to_string() }
//! }
//!
//! impl PluginInventory for DatabaseInventoryPlugin {
//!     fn load(
//!         &self,
//!         settings: &Settings,
//!         plugins: &PluginManager,
//!     ) -> Result<Inventory, InventoryLoadError> {
//!         // Load from database
//!         unimplemented!()
//!     }
//! }
//! ```
//!
//! ### Runner Plugins
//!
//! Execute tasks against hosts:
//!
//! ```rust
//! use genja_plugin_manager::plugin_types::{Plugin, PluginRunner};
//! use genja_core::inventory::Hosts;
//! use genja_core::task::{Task, Tasks};
//!
//! #[derive(Debug)]
//! struct SequentialRunner;
//!
//! impl Plugin for SequentialRunner {
//!     fn name(&self) -> String { "sequential".to_string() }
//! }
//!
//! impl PluginRunner for SequentialRunner {
//!     fn run(&self, task: Task, hosts: &Hosts) {
//!         // Execute task on each host sequentially
//!     }
//!
//!     fn run_tasks(&self, tasks: Tasks, hosts: &Hosts) {
//!         // Execute all tasks sequentially
//!     }
//! }
//! ```
//!
//! ### Transform Function Plugins
//!
//! Provide inventory transformation functions:
//!
//! ```rust
//! use genja_plugin_manager::plugin_types::{Plugin, PluginTransformFunction};
//! use genja_core::inventory::{TransformFunction, Host, BaseBuilderHost};
//!
//! #[derive(Debug)]
//! struct NormalizeHostnamePlugin;
//!
//! impl Plugin for NormalizeHostnamePlugin {
//!     fn name(&self) -> String { "normalize_hostname".to_string() }
//! }
//!
//! impl PluginTransformFunction for NormalizeHostnamePlugin {
//!     fn transform_function(&self) -> TransformFunction {
//!         TransformFunction::new(|host: &Host, _options| {
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
//! ## Building Plugins
//!
//! ### Plugin Project Setup
//!
//! 1. Add dependency in `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! genja-plugin-manager = "0.1.0"
//! genja-core = "0.1.0"
//! ```
//!
//! 2. Configure library type:
//!
//! ```toml
//! [lib]
//! name = "my_plugin"
//! crate-type = ["lib", "cdylib"]
//! ```
//!
//! 3. Build the plugin:
//!
//! ```bash
//! cargo build --release
//! ```
//!
//! The compiled library will be in `target/release/` with platform-specific naming:
//! - Linux: `libmy_plugin.so`
//! - macOS: `libmy_plugin.dylib`
//! - Windows: `my_plugin.dll`
//!
//! When configuring the end-user project, prefer grouping by plugin type in
//! `package.metadata.plugins` (see example below).
//!
//! ## Project Structure Differences
//!
//! ### Core Library (Plugin Consumer)
//!
//! ```toml
//! [package]
//! name = "genja"
//! version = "0.1.0"
//!
//! [dependencies]
//! genja-plugin-manager = "0.1.0"
//! ```
//!
//! ### Plugin Project (Connection/Runner/Inventory/Transform)
//!
//! ```toml
//! [package]
//! name = "my_plugin"
//! version = "0.1.0"
//!
//! [dependencies]
//! genja-plugin-manager = "0.1.0"
//! genja-core = "0.1.0"
//!
//! [lib]
//! name = "my_plugin"
//! crate-type = ["lib", "cdylib"]
//! ```
//!
//! ### End-User Project
//!
//! ```toml
//! [package]
//! name = "genja-app"
//! version = "0.1.0"
//!
//! [dependencies]
//! genja = "0.1.0"
//!
//! # Grouped by plugin type (recommended)
//! [package.metadata.plugins.connection]
//! ssh = "/path/to/libssh.so"
//!
//! [package.metadata.plugins.inventory]
//! inventory_a = "/path/to/libinventory.so"
//!
//! [package.metadata.plugins.runner]
//! threaded = "/path/to/libthreaded.so"
//!
//! [package.metadata.plugins.transform]
//! normalize = "/path/to/libnormalize.so"
//! ```

pub mod plugin_types;
// pub use plugin_types;
pub mod connection_factory;

use libloading::{Library, Symbol};
use plugin_types::{
    GroupOrName, PluginConnection, PluginCreatePlugins, PluginEntry, PluginInventory, PluginName,
    PluginResultPlugins, PluginRunner, PluginTransformFunction, Plugins,
};
use serde::Deserialize;
use std::collections::{HashMap, hash_map};
use std::path::Path;
// use std::error::Error;
use std::io::{Error, ErrorKind};

#[derive(Deserialize, Debug)]
pub struct Metadata {
    pub plugins: Option<HashMap<GroupOrName, PluginEntry>>,
}

/// Central registry and loader for dynamic plugins.
///
/// Holds the loaded plugin instances (`plugins`), metadata discovered from
/// plugin manifests (`plugin_path`), and the underlying dynamic libraries
/// (`libraries`) to keep them alive for the lifetime of the manager.
///
/// Note: `libraries` must be retained for as long as any plugin is in use,
/// otherwise symbol pointers may become invalid.
#[derive(Debug)]
pub struct PluginManager {
    plugins: HashMap<PluginName, Plugins>,
    plugin_path: Vec<HashMap<GroupOrName, PluginEntry>>,
    libraries: Vec<libloading::Library>, // Add this to keep libraries alive
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}
/// Collect plugins matching a specific `Plugins` enum variant as trait objects.
///
/// This macro iterates the internal `plugins` map, filters by the given
/// variant, and returns a collection of `(name, trait_object)` pairs.
/// It is used to build typed views over the heterogeneous plugin registry.
macro_rules! get_plugins_by_variant {
    ($self:expr, $variant:path, $trait_type:ty) => {
        $self
            .plugins
            .iter()
            .filter_map(|(name, plugin)| match plugin {
                $variant(inner) => Some((name, inner as $trait_type)),
                _ => None,
            })
            .collect()
    };
}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {
            plugins: HashMap::new(),
            plugin_path: Vec::new(),
            libraries: Vec::new(),
        }
    }

    /// Activate all plugins discovered from metadata and configured paths.
    ///
    /// Collects registrations from the plugin manifest and any entries in
    /// `plugin_path`, then invokes activation for each entry. Returns the
    /// updated `PluginManager` on success.
    pub fn activate_plugins(mut self) -> Result<PluginManager, Box<dyn std::error::Error>> {
        let meta_data = self.get_plugin_metadata();
        log::debug!("Plugin metadata: {:?}", meta_data);
        let mut registrations = Vec::new();
        if let Some(plugin_config) = meta_data.plugins {
            for (group_or_name, plugin_entry) in plugin_config {
                registrations.push((group_or_name, plugin_entry));
            }
        } else {
            log::error!("No plugin metadata found in manifest");
            return Err("No plugin metadata found in manifest".into());
        }
        if !self.plugin_path.is_empty() {
            for entry in &self.plugin_path {
                for (group_or_name, plugin_entry) in entry {
                    registrations.push((group_or_name.clone(), plugin_entry.clone()));
                }
            }
        }
        for (group_or_name, plugin_entry) in registrations {
            self.activation_registration(group_or_name.clone(), &plugin_entry)?;
        }
        Ok(self)
    }

    /// Retrieves the environment variable CARGO_MANIFEST_PATH containing the
    /// path to  manifest file. The file should contain the plugin metadata
    /// in TOML format which contains the following structure:
    ///
    /// ```toml
    /// [package.metadata.plugins]
    /// plugin_a = "/path/to/plugin_a.so"
    ///
    /// [package.metadata.plugins.inventory]
    /// inventory_plugin = "/path/to/inventory_plugin.so"
    /// ```
    pub fn get_plugin_metadata(&self) -> Metadata {
        let plugin_path = std::env::var("CARGO_MANIFEST_PATH").unwrap_or_else(|_| ".".to_string());

        let file_string = std::fs::read_to_string(plugin_path);
        let manifest = match file_string {
            Ok(manifest) => manifest,
            Err(msg) => {
                eprintln!("Error reading manifest file {}", msg);
                return Metadata { plugins: None };
            }
        };
        let value: toml::Value = match toml::from_str(&manifest) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Error parsing manifest file: {err}");
                return Metadata { plugins: None };
            }
        };
        // let metadata = if let Some(meta_data) = value
        if let Some(meta_data) = value
            .get("package")
            .and_then(|p| p.get("metadata"))
            .and_then(|m| m.as_table())
        {
            let meta: Result<Metadata, toml::de::Error> =
                toml::from_str(&toml::to_string(meta_data).unwrap());
            meta.unwrap()
        } else {
            Metadata { plugins: None }
        }
        // metadata
    }

    /// Load and register plugins for a single manifest entry.
    ///
    /// Supports both individual plugin paths and grouped plugin entries.
    /// Loaded libraries are retained to keep symbols alive.
    fn activation_registration(
        &mut self,
        group_or_name: String,
        plugin_entry: &PluginEntry,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match plugin_entry {
            PluginEntry::Individual(path) => {
                log::debug!("Loading individual plugin: {group_or_name} {path}");
                let (library, plugins) = self.load_plugin(path)?;
                self.libraries.push(library);
                for plugin in plugins {
                    self.register_plugin(plugin);
                }
            }
            PluginEntry::Group(group_plugins) => {
                for (name, path) in group_plugins {
                    log::debug!("Loading plugin group: {group_or_name}, {name} {path}");
                    let (library, plugins) = self.load_plugin(path)?;
                    self.libraries.push(library);
                    for plugin in plugins {
                        self.register_plugin(plugin);
                    }
                }
            }
        }
        Ok(())
    }

    /// Load a dynamic library and invoke its `create_plugins` factory.
    ///
    /// Returns the opened library and the plugins it creates, or an error if
    /// the file is missing, cannot be loaded, or the symbol is unavailable.
    pub fn load_plugin(&self, filename: &str) -> PluginResultPlugins {
        let path = Path::new(filename);

        if !path.exists() {
            let msg = format!("Plugin file does not exist: {}", filename);
            log::error!("{msg}");
            return Err(msg.into());
        } else {
            log::debug!("Attempting to load plugin: {}", filename);
        }

        let library = unsafe { Library::new(path)? };
        log::debug!("Library loaded successfully");

        let create_plugin: Symbol<PluginCreatePlugins> = unsafe { library.get(b"create_plugins")? };
        log::debug!("Found create_plugins symbol");

        let plugins = unsafe { create_plugin() };
        log::debug!("Plugin created successfully");

        Ok((library, plugins))
    }

    /// Insert a plugin into the registry by name.
    ///
    /// Panics if a plugin with the same name is already registered.
    pub fn register_plugin(&mut self, plugin: Plugins) {
        let name = plugin.name();
        log::info!("Registering plugin: {:?}", name);

        println!("Registering plugin: {}", name);
        if let hash_map::Entry::Vacant(entry) = self.plugins.entry(name.clone()) {
            entry.insert(plugin);
        } else {
            let msg = format!("Plugin '{}' already registered", &name);
            log::error!("{msg}");
            panic!("{msg}");
        }
    }

    /// Gets a plugin as a trait object based on its type
    pub fn get_plugin(&self, name: &str) -> Option<&Plugins> {
        self.plugins.get(name)
    }

    /// Gets an inventory plugin, returns None if the plugin is not a Base variant
    #[allow(clippy::borrowed_box)]
    pub fn get_connection_plugin(&self, name: &str) -> Option<&Box<dyn PluginConnection>> {
        self.plugins.get(name).and_then(|plugin| match plugin {
            Plugins::Connection(base) => Some(base),
            _ => None,
        })
    }

    #[allow(clippy::borrowed_box)]
    /// Gets an inventory plugin, returns None if the plugin is not an Inventory variant
    pub fn get_inventory_plugin(&self, name: &str) -> Option<&Box<dyn PluginInventory>> {
        self.plugins.get(name).and_then(|plugin| match plugin {
            Plugins::Inventory(inventory) => Some(inventory),
            _ => None,
        })
    }

    #[allow(clippy::borrowed_box)]
    /// Gets a transform function plugin, returns None if the plugin is not a TransformFunction variant
    pub fn get_transform_function_plugin(
        &self,
        name: &str,
    ) -> Option<&Box<dyn PluginTransformFunction>> {
        self.plugins.get(name).and_then(|plugin| match plugin {
            Plugins::TransformFunction(transform) => Some(transform),
            _ => None,
        })
    }

    /// Generic method to get plugins by variant type with a mapper function
    pub fn get_plugins_by_variant<'a, T>(
        &'a self,
        mapper: impl Fn(&'a Plugins) -> Option<T>,
    ) -> Vec<(&'a String, T)> {
        self.plugins
            .iter()
            .filter_map(|(name, plugin)| mapper(plugin).map(|p| (name, p)))
            .collect()
    }

    // /// Gets all plugins by their type, using a mapper function to extract the desired type
    // pub fn get_plugins_by_group<T>(&self, plugin: Plugins) -> Vec<(&String, &Box<dyn T>)> {
    //     get_plugins_by_variant!(self, plugin, &Box<dyn T>)
    // }
    // pub fn get_plugins_by_group<T>(&self) -> Vec<(&String, T)> {
    //     let mapper = |plugin| match plugin {
    //         Plugins::Base(base) => Some(base),
    //         _ => None,
    //     };
    //     let res = self.get_plugins_by_variant::<T>(mapper);
    //     res
    // }

    /// Gets all Base plugins with their trait objects
    #[allow(clippy::borrowed_box)]
    pub fn get_plugins_by_type_connection(&self) -> Vec<(&String, &Box<dyn PluginConnection>)> {
        get_plugins_by_variant!(self, Plugins::Connection, &Box<dyn PluginConnection>)
    }

    /// Gets all Inventory plugins with their trait objects
    #[allow(clippy::borrowed_box)]
    pub fn get_plugins_by_type_inventory(&self) -> Vec<(&String, &Box<dyn PluginInventory>)> {
        get_plugins_by_variant!(self, Plugins::Inventory, &Box<dyn PluginInventory>)
    }

    /// Gets all TransformFunction plugins with their trait objects
    #[allow(clippy::borrowed_box)]
    pub fn get_plugins_by_type_transform_function(
        &self,
    ) -> Vec<(&String, &Box<dyn PluginTransformFunction>)> {
        get_plugins_by_variant!(
            self,
            Plugins::TransformFunction,
            &Box<dyn PluginTransformFunction>
        )
    }

    /// Deregisters the plugin with the given name.
    pub fn deregister_plugin(&mut self, name: &str) -> Option<String> {
        if let Some(plugin) = self.plugins.remove(name) {
            log::info!("De-registering plugin: {}", name);
            Some(plugin.name())
        } else {
            None
        }
    }

    /// Deregisters all plugins.
    pub fn deregister_all_plugins(&mut self) -> Vec<String> {
        let mut deregistered_plugins = Vec::new();
        for (name, plugin) in self.plugins.drain() {
            log::info!("De-registering plugin: {}", name);
            deregistered_plugins.push(plugin.name());
        }
        deregistered_plugins
    }

    /// Gets all the **names** of the registered plugins.
    pub fn get_all_plugin_names(&self) -> Vec<&String> {
        self.plugins.keys().collect()
    }

    /// Gets all the **names** and **groups** of the registered plugins.
    pub fn get_all_plugin_names_and_groups(&self) -> Vec<(String, String)> {
        self.plugins
            .iter()
            .map(|(name, plugin)| (name.clone(), plugin.group_name()))
            .collect()
    }

    /// Gets a runner plugin by name, if registered.
    #[allow(clippy::borrowed_box)]
    pub fn get_runner_plugin(&self, name: &str) -> Option<&Box<dyn PluginRunner>> {
        self.plugins.get(name).and_then(|plugin| match plugin {
            Plugins::Runner(runner) => Some(runner),
            _ => None,
        })
    }
    pub fn with_path(mut self, path: &str, group: Option<&str>) -> Result<Self, Error> {
        let path = Path::new(&path);
        if path.exists() {
            let path_string = if let Some(path_str) = path.to_str() {
                path_str.to_string()
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Path contains invalid Unicode",
                ));
            };
            if let Some(group_string) = group {
                let group_info = HashMap::from([(
                    group_string.to_string(),
                    PluginEntry::Group(HashMap::from([(group_string.to_string(), path_string)])),
                )]);
                self.plugin_path.push(group_info);
            } else {
                let individual_info =
                    HashMap::from([("base".to_string(), PluginEntry::Individual(path_string))]);
                self.plugin_path.push(individual_info);
            };
            Ok(self)
        } else {
            Err(Error::new(
                ErrorKind::NotFound,
                format!("FileNotFoundError: {:?}", path.as_os_str()),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let lock = LOCK.get_or_init(|| Mutex::new(()));
        lock.lock().unwrap_or_else(|err| err.into_inner())
    }

    fn workspace_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .to_path_buf()
    }

    fn ensure_test_plugins_built() {
        static BUILT: OnceLock<()> = OnceLock::new();
        BUILT.get_or_init(|| {
            let status = Command::new("cargo")
                .current_dir(workspace_root())
                .args([
                    "build",
                    "--quiet",
                    "-p",
                    "plugin-mods",
                    "-p",
                    "plugin_inventory",
                    "-p",
                    "plugin_tasks",
                ])
                .status()
                .expect("Failed to run cargo build for test plugins");
            assert!(status.success(), "Failed to build test plugins");
        });
    }

    fn set_env_var() -> MutexGuard<'static, ()> {
        let guard = env_lock();
        ensure_test_plugins_built();
        let file_name = match std::env::consts::OS {
            "linux" => "Cargo.toml",
            "windows" => "Cargo-windows.toml",
            "macos" => "Cargo-macos.toml",
            _ => "Cargo.toml",
        };
        let file = format!("../genja-plugin-manager/tests/plugin_mods/{}", file_name);
        unsafe {
            std::env::set_var("CARGO_MANIFEST_PATH", file);
        }
        guard
    }

    fn make_file_path(module_name: &str) -> String {
        ensure_test_plugins_built();
        let mut path_name = PathBuf::new();
        let mut module_name_prefix = String::from(std::env::consts::DLL_PREFIX);
        module_name_prefix.push_str(module_name);
        path_name.push("..");
        path_name.push("target");
        path_name.push("debug");
        path_name.push(module_name_prefix);
        path_name.set_extension(std::env::consts::DLL_EXTENSION);
        path_name.to_string_lossy().to_string()
    }

    fn temp_manifest_path(filename: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("genja_plugin_manager_{now}_{filename}"));
        path
    }

    fn temp_file_path(filename: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut path = std::env::temp_dir();
        path.push(format!("genja_plugin_manager_{now}_{filename}"));
        path
    }

    #[cfg(target_os = "linux")]
    fn system_library_path() -> Option<&'static str> {
        let candidates = [
            "/lib/x86_64-linux-gnu/libc.so.6",
            "/lib64/libc.so.6",
            "/usr/lib/x86_64-linux-gnu/libc.so.6",
        ];
        candidates.iter().copied().find(|p| Path::new(p).exists())
    }

    #[cfg(target_os = "macos")]
    fn system_library_path() -> Option<&'static str> {
        let p = "/usr/lib/libSystem.B.dylib";
        if Path::new(p).exists() {
            Some(p)
        } else {
            None
        }
    }

    #[cfg(target_os = "windows")]
    fn system_library_path() -> Option<&'static str> {
        let p = "C:\\Windows\\System32\\kernel32.dll";
        if Path::new(p).exists() {
            Some(p)
        } else {
            None
        }
    }

    #[test]
    fn get_plugin_path_test() {
        let _env = set_env_var();
        let plugin_manager = PluginManager::new();
        let metadata = plugin_manager.get_plugin_metadata();
        let plugins = metadata.plugins;
        match plugins {
            Some(plug_entry) => {
                for (group, entry) in plug_entry {
                    match entry {
                        PluginEntry::Individual(path) => {
                            assert_eq!(path, make_file_path("plugin_mods"));
                        }
                        PluginEntry::Group(path) => {
                            path.iter().for_each(|(metadata_name, path)| {
                                assert_eq!(path, &make_file_path("plugin_inventory"));
                                assert_eq!(metadata_name, "inventory_a");
                                assert_eq!(group, "inventory");
                            });
                        }
                    }
                }
            }
            None => {
                panic!("No plugins found in metadata");
            }
        }
    }

    #[test]
    fn get_plugin_metadata_test() {
        let _env = set_env_var();
        let plugin_manager = PluginManager::new();
        let metadata = plugin_manager.get_plugin_metadata();
        assert!(metadata.plugins.is_some());
        // Check if the metadata contains the expected number of plugin paths.
        assert_eq!(metadata.plugins.clone().unwrap().len(), 2);
    }

    #[test]
    fn get_plugin_metadata_missing_manifest_test() {
        let _env = env_lock();
        let missing = temp_manifest_path("missing_manifest.toml");
        unsafe {
            std::env::set_var("CARGO_MANIFEST_PATH", missing.to_string_lossy().to_string());
        }
        let plugin_manager = PluginManager::new();
        let metadata = plugin_manager.get_plugin_metadata();
        assert!(metadata.plugins.is_none());
    }

    #[test]
    fn get_plugin_metadata_missing_metadata_section_test() {
        let _env = env_lock();
        let manifest = temp_manifest_path("no_metadata.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"no_metadata\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        unsafe {
            std::env::set_var("CARGO_MANIFEST_PATH", manifest.to_string_lossy().to_string());
        }
        let plugin_manager = PluginManager::new();
        let metadata = plugin_manager.get_plugin_metadata();
        assert!(metadata.plugins.is_none());
        let _ = std::fs::remove_file(&manifest);
    }

    #[test]
    fn get_plugin_metadata_invalid_toml_test() {
        let _env = env_lock();
        let manifest = temp_manifest_path("invalid_toml.toml");
        std::fs::write(
            &manifest,
            "[package]\nname = \"invalid\"\nversion =\n",
        )
        .unwrap();
        unsafe {
            std::env::set_var("CARGO_MANIFEST_PATH", manifest.to_string_lossy().to_string());
        }
        let plugin_manager = PluginManager::new();
        let metadata = plugin_manager.get_plugin_metadata();
        assert!(metadata.plugins.is_none());
        let _ = std::fs::remove_file(&manifest);
    }

    #[test]
    fn activate_plugins_group_invalid_path_returns_error_test() {
        let _env = env_lock();
        let manifest = temp_manifest_path("group_invalid_path.toml");
        std::fs::write(
            &manifest,
            r#"[package]
name = "invalid_group"
version = "0.1.0"

[package.metadata.plugins.inventory]
inventory_a = "../this/path/does/not/exist.so"
"#,
        )
        .unwrap();
        unsafe {
            std::env::set_var("CARGO_MANIFEST_PATH", manifest.to_string_lossy().to_string());
        }
        let plugin_manager = PluginManager::new();
        let result = plugin_manager.activate_plugins();
        assert!(result.is_err());
        let _ = std::fs::remove_file(&manifest);
    }

    #[test]
    fn activate_plugins_test() {
        let _env = set_env_var();
        let mut plugin_manager = PluginManager::new();
        plugin_manager = plugin_manager.activate_plugins().unwrap();
        assert!(plugin_manager.get_plugin("plugin_a").is_some());
        assert_eq!(plugin_manager.plugins.len(), 3);
    }

    #[test]
    #[should_panic]
    /// Test for duplicate activation of plugins.
    fn activate_plugins_and_panic_test() {
        let _env = set_env_var();
        let mut plugin_manager = PluginManager::new();
        plugin_manager = plugin_manager.activate_plugins().unwrap();
        _ = plugin_manager.activate_plugins().unwrap();
    }

    #[test]
    fn load_plugin_test() {
        let plugin_manager = PluginManager::new();
        let filename = make_file_path("plugin_mods");
        let (_library, plugins) = plugin_manager.load_plugin(&filename).unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name(), "plugin_a");
    }

    #[test]
    fn load_plugin_and_panic_test() {
        let plugin_manager = PluginManager::new();
        let filename = make_file_path("plugin_mods");
        let (_library, _) = plugin_manager.load_plugin(&filename).unwrap();
        let filename = make_file_path("plugin_mods");
        let (_library, plugins) = plugin_manager.load_plugin(&filename).unwrap();
        assert_eq!(plugins.len(), 2);
        assert_eq!(plugins[0].name(), "plugin_a");
    }

    #[test]
    fn load_plugin_missing_file_test() {
        let plugin_manager = PluginManager::new();
        let missing = temp_file_path("missing_plugin_file.so");
        let result = plugin_manager.load_plugin(&missing.to_string_lossy());
        assert!(result.is_err());
    }

    #[test]
    fn load_plugin_invalid_library_test() {
        let plugin_manager = PluginManager::new();
        let file = temp_file_path("not_a_library.so");
        std::fs::write(&file, "not a library").unwrap();
        let result = plugin_manager.load_plugin(&file.to_string_lossy());
        assert!(result.is_err());
        let _ = std::fs::remove_file(&file);
    }

    #[test]
    fn load_plugin_missing_symbol_test() {
        let plugin_manager = PluginManager::new();
        let Some(path) = system_library_path() else {
            return;
        };
        let result = plugin_manager.load_plugin(path);
        assert!(result.is_err());
    }

    #[test]
    fn activate_plugins_with_groups_test() {
        let _env = set_env_var();
        let plugin_manager = PluginManager::new().activate_plugins().unwrap();

        // Get all plugins in the "base" group
        let inventory_plugins = plugin_manager.get_plugins_by_type_connection();
        assert_eq!(inventory_plugins.len(), 2);

        // Get all plugins in the "inventory" group
        let inventory_plugins = plugin_manager.get_plugins_by_type_inventory();
        assert_eq!(inventory_plugins.len(), 1);
        assert_eq!(inventory_plugins[0].1.name(), "inventory_a");

        assert_eq!(plugin_manager.plugins.len(), 3);
    }

    #[test]
    fn get_all_plugin_names_and_groups_test() {
        let _env = set_env_var();
        let plugin_manager = PluginManager::new().activate_plugins().unwrap();
        let all_plugins = plugin_manager.get_all_plugin_names_and_groups();
        assert_eq!(all_plugins.len(), 3);
        all_plugins
            .iter()
            .for_each(|(name, group)| match name.as_str() {
                "plugin_a" => assert_eq!(group, "Connection"),
                "plugin_b" => assert_eq!(group, "Connection"),
                "inventory_a" => assert_eq!(group, "Inventory"),
                _ => panic!("Unexpected plugin name"),
            });
    }

    #[test]
    fn deregister_plugin_test() {
        let _env = set_env_var();
        let mut plugin_manager = PluginManager::new().activate_plugins().unwrap();
        assert_eq!(plugin_manager.plugins.len(), 3);

        // Deregister individual plugin
        let plugin_name = plugin_manager.deregister_plugin("plugin_a");
        if let Some(plugin) = plugin_name {
            assert_eq!(plugin, "plugin_a");
            assert_eq!(plugin_manager.plugins.len(), 2);
        }

        // Deregister grouped plugin
        let plugin_name = plugin_manager.deregister_plugin("inventory_a");
        if let Some(plugin) = plugin_name {
            assert_eq!(plugin, "inventory_a");
            assert_eq!(plugin_manager.plugins.len(), 1);
        }

        // Deregister non-existent plugin
        let plugin_name = plugin_manager.deregister_plugin("non_existent_plugin");
        assert_eq!(plugin_name, None);
    }

    #[test]
    fn deregister_all_plugins_test() {
        let _env = set_env_var();
        let mut plugin_manager = PluginManager::new().activate_plugins().unwrap();
        assert_eq!(plugin_manager.plugins.len(), 3);

        // Deregister all plugins
        let num_plugins_deregistered = plugin_manager.deregister_all_plugins();
        assert_eq!(num_plugins_deregistered.len(), 3);
        assert_eq!(plugin_manager.plugins.len(), 0);
    }

    #[test]
    fn plugin_manager_new_test() {
        let _env = set_env_var();
        let mut plugin_manager = PluginManager::new();
        assert_eq!(plugin_manager.plugins.len(), 0);
        plugin_manager = plugin_manager.activate_plugins().unwrap();
        assert_eq!(plugin_manager.plugins.len(), 3);
    }

    #[test]
    fn get_plugins_by_type_test() {
        let _env = set_env_var();
        let plugin_manager = PluginManager::new().activate_plugins().unwrap();
        let connection_plugins = plugin_manager.get_plugins_by_type_connection();
        assert_eq!(connection_plugins.len(), 2);

        // Check that the expected plugin names are present
        let base_plugin_names: Vec<&str> = connection_plugins
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        assert!(base_plugin_names.contains(&"plugin_a"));
        assert!(base_plugin_names.contains(&"plugin_b"));

        // Verify the debug output format for base plugins
        for (name, plugin) in connection_plugins {
            let debug_output = format!("{:?}", plugin);
            assert!(debug_output.contains("ConnectionPlugin"));
            assert!(debug_output.contains(name));
        }

        let inventory_plugins = plugin_manager.get_plugins_by_type_inventory();
        assert_eq!(inventory_plugins.len(), 1);
    }

    #[test]
    fn with_path_test() {
        let _env = set_env_var();
        let path = make_file_path("plugin_tasks");
        let plugin_manager = PluginManager::new()
            .with_path(&path, None)
            .unwrap()
            .activate_plugins()
            .unwrap();
        assert_eq!(plugin_manager.plugins.len(), 4);
    }

    #[test]
    fn with_path_not_found_test() {
        let missing = temp_file_path("missing_with_path_plugin.so");
        let result = PluginManager::new().with_path(&missing.to_string_lossy(), None);
        assert!(result.is_err());
        if let Err(err) = result {
            assert_eq!(err.kind(), ErrorKind::NotFound);
        }
    }

    #[test]
    #[should_panic]
    fn with_path_duplicate_plugin_panics_test() {
        let _env = set_env_var();
        let duplicate = make_file_path("plugin_mods");
        let _ = PluginManager::new()
            .with_path(&duplicate, None)
            .unwrap()
            .activate_plugins()
            .unwrap();
    }
}
