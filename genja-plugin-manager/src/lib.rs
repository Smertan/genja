//! # Genja Plugin Manager
//!
//! A flexible and easy-to-use plugin management system for Rust applications.
//!
//! This module provides a `PluginManager` that allows dynamic loading, registration,
//! and management of plugins at runtime. It supports individual plugins and grouped plugins,
//! making it suitable for various application architectures.
//!
//! ## Features
//!
//! - Dynamic loading of plugins from shared object files (.so)
//! - Support for individual and grouped plugins
//! - Plugin registration and deregistration
//! - Registry access for typed plugin functionality
//! - Metadata-driven plugin configuration
//!
//!
//! ## Creating Plugins
//!
//! To create a plugin, implement the `Plugin` trait and export a `create_plugins` function:
//!
//! ```rust
//! use genja_core::inventory::Hosts;
//! use genja_core::task::{Task, Tasks};
//! use plugin_manager::plugin_types::{Plugin, PluginRunner};
//!
//! #[derive(Debug)]
//! struct MyPlugin;
//!
//! impl Plugin for MyPlugin {
//!     fn name(&self) -> String {
//!         "my_plugin".to_string()
//!     }
//!
//! }
//!
//! impl PluginRunner for MyPlugin {
//!     fn run(&self, _task: Task, _hosts: &Hosts) {}
//!     fn run_tasks(&self, _tasks: Tasks, _hosts: &Hosts) {}
//! }
//!
//! #[unsafe(no_mangle)]
//! pub fn create_plugins() -> Vec<plugin_manager::plugin_types::Plugins> {
//!     vec![plugin_manager::plugin_types::Plugins::Runner(Box::new(MyPlugin))]
//! }
//! ```
//!
//! ## Setting up Cargo.toml for Plugins
//!
//! When creating a plugin, you need to set up your `Cargo.toml` file correctly:
//!
//! 1. Add the `genja-plugin-manager` as a dependency:
//!
//! ```toml
//! [dependencies]
//! genja-plugin-manager = "0.1.0"
//! ```
//!
//! 2. Configure the library to be both a Rust library and a dynamic library:
//!
//! ```toml
//! [lib]
//! name = "your_plugin_name"
//! crate-type = ["lib", "cdylib"]
//! ```
//!
//! This configuration allows your plugin to be compiled as both a Rust library
//! and a dynamic library, which is necessary for the PluginManager to load it at runtime.
//!
//! ## Building the Plugin
//!
//! To build your plugin for use with the main project:
//!
//! 1. Navigate to your plugin's directory.
//! 2. Run the following command to build the plugin as a dynamic library:
//!
//!    ```bash
//!    cargo build --release
//!    ```
//!
//! 3. The compiled dynamic library will be in the `target/release` directory with a name like
//!    `libyour_plugin_name.so` (on Linux), `libyour_plugin_name.dylib` (on macOS),
//!    or `your_plugin_name.dll` (on Windows).
//!
//! ## Differences between Cargo.toml Files
//!
//! Both the main project using plugins and the individual plugin projects are end users of the plugin_manager.
//!
//! 1. Main Project Cargo.toml:
//!    - Located in the root of the project that will use plugins.
//!    - Includes `genja-plugin-manager` as a dependency.
//!    - Does not need the `crate-type` specification.
//!    - Does not contain any metadata for plugin configuration.
//!    - The loaded plugins are dependant on the plugins specified in the `End-User's` project Cargo.toml.
//!
//!    Example:
//!    ```toml
//!    [package]
//!    name = "main_project"
//!    version = "0.1.0"
//!    edition = "2024"
//!
//!    [dependencies]
//!    genja-plugin-manager = "0.1.0"
//!    ```
//!
//! 2. Plugin Project Cargo.toml:
//!    - Located in a separate project directory for each plugin.
//!    - Includes `genja-plugin-manager` as a dependency.
//!    - Specifies `crate-type = ["lib", "cdylib"]` to build as both a Rust library and a dynamic library.
//!    - Does not contain plugin metadata configuration.
//!
//!    Example:
//!    ```toml
//!    [package]
//!    name = "my_plugin"
//!    version = "0.1.0"
//!    edition = "2024"
//!
//!    [dependencies]
//!    genja-plugin-manager = "0.1.0"
//!
//!    [lib]
//!    name = "my_plugin"
//!    crate-type = ["lib", "cdylib"]
//!    ```
//!
//! 3. End-User Project Cargo.toml:
//!    - Includes the main project as dependencies.
//!    - Contains metadata for plugin configuration.
//!
//!    Example:
//!    ```toml
//!    [package]
//!    name = "my_application"
//!    version = "0.1.0"
//!    edition = "2024"
//!
//!    [dependencies]
//!    main_project = "0.1.0"
//!
//!    [package.metadata.plugins]
//!    my_plugin = "/path/to/libmy_plugin.so"
//!    ```
//!
//! The main differences between these Cargo.toml files are:
//!
//! 1. The Main Project Cargo.toml sets up the core project that will use plugins:
//!    - It includes the genja-plugin-manager as a dependency.
//!    - It doesn't specify crate-type or contain plugin metadata.
//!    - The plugins it can load are determined by the End-User's project configuration.
//!
//! 2. The Plugin Project Cargo.toml configures individual plugin projects:
//!    - It includes the genja-plugin-manager as a dependency.
//!    - It specifies crate-type as both "lib" and "cdylib" to produce a dynamic library.
//!    - It doesn't contain any plugin metadata configuration.
//!
//! 3. The End-User Project Cargo.toml configures the application that will use the main project and its plugins:
//!    - It includes the main project (not the genja-plugin-manager directly) as a dependency.
//!    - It contains the metadata for plugin configuration, specifying which plugins to load and how to group them.
//!
//! ## Plugin Configuration
//!
//! Plugins are configured in the `Cargo.toml` file of the end-user project:
//!
//! ```toml
//! [package.metadata.plugins]
//! plugin_a = "/path/to/plugin_a.so"
//!
//! [package.metadata.plugins.group_name]
//! plugin_b = "/path/to/plugin_b.so"
//! plugin_c = "/path/to/plugin_c.so"
//! ```
//!
//! ## Usage
//!
//! Here's a basic example of how to use the `PluginManager`:
//!
//! ```rust
//! # unsafe {
//! #     std::env::set_var("CARGO_MANIFEST_PATH", "../tests/plugin_mods/Cargo.toml");
//! # }
//! use plugin_manager::PluginManager;
//!
//! # fn doc_test() -> Result<(), Box<dyn std::error::Error>> {
//! // Create a new PluginManager
//! let mut plugin_manager = PluginManager::new();
//!
//! // Activate plugins based on metadata in Cargo.toml
//! plugin_manager = plugin_manager.activate_plugins()?;
//!
//! // Access a specific plugin by type
//! if let Some(runner) = plugin_manager.get_runner_plugin("my_plugin") {
//!     // runner.run(...);
//! }
//!
//! // Deregister a plugin
//! let deregistered = plugin_manager.deregister_plugin("plugin_b");
//! print!("Deregistered plugin: {:?}", deregistered);
//!
//! // Deregister all plugins
//! let deregistered = plugin_manager.deregister_all_plugins();
//! println!("Deregistered plugins: {:?}", deregistered);
//!
//! #    Ok(())
//! # }
//! ```
//!
//!
//! This module provides a robust foundation for building plugin-based architectures
//! in Rust applications, offering flexibility and ease of use.

pub mod plugin_structs;
pub mod plugin_types;
// pub use plugin_types;
pub mod connection_factory;

use libloading::{Library, Symbol};
use plugin_structs::{PluginCreate as PluginCreateNew, PluginResult as PluginResultNew};
use plugin_types::{
    GroupOrName, PluginConnection, PluginEntry, PluginInventory, PluginName, PluginRunner,
    PluginTransformFunction, Plugins,
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
        let plugin_path =
            std::env::var("CARGO_MANIFEST_PATH").unwrap_or_else(|_| ".".to_string());

        let file_string = std::fs::read_to_string(plugin_path);
        let manifest = match file_string {
            Ok(manifest) => manifest,
            Err(msg) => {
                eprintln!("Error reading manifest file {}", msg);
                return Metadata { plugins: None };
            }
        };
        let value: toml::Value = toml::from_str(&manifest).unwrap();
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
                group_plugins.iter().for_each(|(name, path)| {
                    log::debug!("Loading plugin group: {group_or_name}, {name} {path}");
                    let (library, plugins) = self.load_plugin(path).unwrap();
                    self.libraries.push(library);
                    for plugin in plugins {
                        self.register_plugin(plugin);
                    }
                });
            }
        }
        Ok(())
    }

    /// Load a dynamic library and invoke its `create_plugins` factory.
    ///
    /// Returns the opened library and the plugins it creates, or an error if
    /// the file is missing, cannot be loaded, or the symbol is unavailable.
    pub fn load_plugin(&self, filename: &str) -> PluginResultNew {
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

        let create_plugin: Symbol<PluginCreateNew> = unsafe { library.get(b"create_plugins")? };
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
    use std::path::PathBuf;

    use super::*;

    fn set_env_var() {
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
    }

    fn make_file_path(module_name: &str) -> String {
        let mut path_name = PathBuf::new();
        let mut module_name_prefix = String::from(std::env::consts::DLL_PREFIX);
        module_name_prefix.push_str(module_name);
        path_name.push("..");
        path_name.push("target");
        path_name.push("release");
        path_name.push(module_name_prefix);
        path_name.set_extension(std::env::consts::DLL_EXTENSION);
        path_name.to_string_lossy().to_string()
    }

    #[test]
    fn get_plugin_path_test() {
        set_env_var();
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
        set_env_var();
        let plugin_manager = PluginManager::new();
        let metadata = plugin_manager.get_plugin_metadata();
        assert!(metadata.plugins.is_some());
        // Check if the metadata contains the expected number of plugin paths.
        assert_eq!(metadata.plugins.clone().unwrap().len(), 2);
    }

    #[test]
    fn activate_plugins_test() {
        set_env_var();
        let mut plugin_manager = PluginManager::new();
        plugin_manager = plugin_manager.activate_plugins().unwrap();
        assert!(plugin_manager.get_plugin("plugin_a").is_some());
        assert_eq!(plugin_manager.plugins.len(), 3);
    }

    #[test]
    #[should_panic]
    /// Test for duplicate activation of plugins.
    fn activate_plugins_and_panic_test() {
        set_env_var();
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
    fn activate_plugins_with_groups_test() {
        set_env_var();
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
        set_env_var();
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
        set_env_var();
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
        set_env_var();
        let mut plugin_manager = PluginManager::new().activate_plugins().unwrap();
        assert_eq!(plugin_manager.plugins.len(), 3);

        // Deregister all plugins
        let num_plugins_deregistered = plugin_manager.deregister_all_plugins();
        assert_eq!(num_plugins_deregistered.len(), 3);
        assert_eq!(plugin_manager.plugins.len(), 0);
    }

    #[test]
    fn plugin_manager_new_test() {
        set_env_var();
        let mut plugin_manager = PluginManager::new();
        assert_eq!(plugin_manager.plugins.len(), 0);
        plugin_manager = plugin_manager.activate_plugins().unwrap();
        assert_eq!(plugin_manager.plugins.len(), 3);
    }

    #[test]
    fn get_plugins_by_type_test() {
        set_env_var();
        let plugin_manager = PluginManager::new().activate_plugins().unwrap();
        let connection_plugins = plugin_manager.get_plugins_by_type_connection();
        assert_eq!(connection_plugins.len(), 2);

        // Check that the expected plugin names are present
        let base_plugin_names: Vec<&str> =
            connection_plugins.iter().map(|(name, _)| name.as_str()).collect();
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
        set_env_var();
        let path = make_file_path("plugin_tasks");
        let plugin_manager = PluginManager::new()
            .with_path(&path, None)
            .unwrap()
            .activate_plugins()
            .unwrap();
        assert_eq!(plugin_manager.plugins.len(), 4);
    }
}
