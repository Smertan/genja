//! # Genja
//!
//! The main runtime composition layer for the Genja network automation framework.
//!
//! This crate provides the [`Genja`] type, which orchestrates inventory management,
//! plugin loading, and task execution. It serves as the primary entry point for
//! building and running network automation workflows.
//!
//! ## Quick Start
//!
//! ```no_run
//! use genja::Genja;
//! use genja_core::Settings;
//!
//! // Load from settings file
//! let genja = Genja::from_settings_file("config.yaml")?;
//!
//! // Or build manually
//! let settings = Settings::from_file("config.yaml")?;
//!
//! let inventory = genja_core::inventory::Inventory::builder()
//!     .hosts(genja_core::inventory::Hosts::new())
//!     .build();
//!
//! let genja = Genja::builder(inventory)
//!     .with_settings(settings)
//!     .build()?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ## Architecture
//!
//! - **Inventory**: Manages hosts, groups, and defaults
//! - **Plugins**: Extensible plugin system for inventory sources and task runners
//! - **Settings**: Configuration loaded from files or environment variables
//!
//! See [`Genja`] for the main API and [`GenjaBuilder`] for construction patterns.

pub use genja_core::GenjaError;
use genja_core::inventory::{Host, Inventory};
use genja_core::task::{Task, TaskDefinition, TaskInfo, TaskResults};
use genja_core::{NatString, Settings};
use genja_plugin_manager::PluginManager;
use genja_plugin_manager::connection_factory::build_connection_factory;
use genja_plugin_manager::plugin_types::{PluginRunner, Plugins};
use std::sync::Arc;

// GenjaError is re-exported from genja-core.

/// Runtime composition layer for `Genja`.
///
/// This type owns the runtime inventory, settings, and plugin manager used to
/// execute tasks. It provides methods to load plugins, load inventory, and run
/// operations against the configured environment.
///
/// # Fields
///
/// * `inventory` - Optional runtime inventory. Set by `load_inventory(...)`.
/// * `host_ids` - Cached host identifiers derived from the loaded inventory.
/// * `settings` - Active settings used by the runtime.
/// * `plugins` - Plugin manager responsible for plugin discovery and execution.
/// * `plugins_loaded` - Tracks whether plugins have been loaded.
///
/// # Examples
///
/// Create an instance from a settings file:
///
/// ```
/// # use genja::Genja;
/// # let filename = format!("genja_settings_{}.yml", std::process::id());
/// # let path = std::env::temp_dir().join(filename);
/// # std::fs::write(&path, "").unwrap();
/// let genja = Genja::from_settings_file(path.to_str().unwrap());
/// assert!(genja.is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct Genja {
    inventory: Option<Arc<Inventory>>,
    host_ids: Arc<Vec<NatString>>,
    settings: Arc<Settings>,
    plugins: Arc<PluginManager>,
    plugins_loaded: bool,
}

pub mod plugins;

impl Genja {
    /// Returns a builder that requires an inventory up front.
    ///
    /// # Examples
    ///
    /// ```
    /// # use genja::Genja;
    /// # use genja_core::Settings;
    /// # use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    /// let inventory = Inventory::builder().hosts(hosts).build();
    ///
    /// let genja = Genja::builder(inventory)
    ///     .with_settings(Settings::default())
    ///     .build()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn builder(inventory: Inventory) -> GenjaBuilder {
        GenjaBuilder::new(inventory)
    }

    pub fn new() -> Self {
        Self {
            inventory: None,
            host_ids: Arc::new(Vec::new()),
            settings: Arc::new(Settings::default()),
            plugins: Arc::new(PluginManager::new()),
            plugins_loaded: false,
        }
    }

    /// Creates a `Genja` instance from an existing `Inventory`.
    ///
    /// Initializes default settings and an empty plugin manager, and derives
    /// the host ID cache from the provided inventory.
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    /// use genja_core::inventory::{Inventory, Hosts};
    ///
    /// let inventory = Inventory::builder()
    ///     .hosts(Hosts::new())
    ///     .build();
    ///
    /// let genja = Genja::from_inventory(inventory);
    /// assert!(genja.inventory().is_ok());
    /// ```
    pub fn from_inventory(inventory: Inventory) -> Self {
        let host_ids = inventory.hosts().keys().cloned().collect();
        Self {
            inventory: Some(Arc::new(inventory)),
            host_ids: Arc::new(host_ids),
            settings: Arc::new(Settings::default()),
            plugins: Arc::new(PluginManager::new()),
            plugins_loaded: false,
        }
    }

    /// Creates a `Genja` instance from a settings file path.
    ///
    /// Loads settings, initializes plugins, and loads inventory based on the
    /// settings file. This is equivalent to calling [`Settings::from_file`],
    /// then [`Genja::new`], [`set_settings`](Self::set_settings),
    /// [`load_plugins`](Self::load_plugins), and
    /// [`load_inventory_from_settings`](Self::load_inventory_from_settings).
    ///
    /// For more control over the construction process, use [`Genja::builder`]
    ///
    /// # Errors
    ///
    /// Returns `Err(GenjaError::ConfigLoad)` if the settings file cannot be read
    /// or parsed. Returns `Err(GenjaError::PluginsNotLoaded)` if plugin loading
    /// fails. Returns `Err(GenjaError::InventoryNotLoaded)` if inventory loading
    /// fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    ///
    /// let filename = format!("genja_settings_{}.yml", std::process::id());
    /// let path = std::env::temp_dir().join(filename);
    /// std::fs::write(&path, "").unwrap();
    ///
    /// let genja = Genja::from_settings_file(path.to_str().unwrap());
    /// assert!(genja.is_ok());
    /// ```
    pub fn from_settings_file(settings_file_path: &str) -> Result<Self, GenjaError> {
        let settings = Settings::from_file(settings_file_path)
            .map_err(|err| GenjaError::ConfigLoad(err.to_string()))?;

        let mut genja = Self::new();
        genja.set_settings(settings);
        genja.load_plugins()?;
        genja.load_inventory_from_settings()?;
        Ok(genja)
    }

    /// Loads inventory using the plugin specified in settings.
    ///
    /// Attempts to load inventory through the configured inventory plugin. Falls
    /// back to `FileInventoryPlugin` if no plugin is specified in settings.
    ///
    /// # Errors
    ///
    /// - `GenjaError::PluginsNotLoaded` - Plugins have not been loaded yet
    /// - `GenjaError::NotInventoryPlugin` - Named plugin exists but is not an inventory plugin
    /// - `GenjaError::PluginNotFound` - No matching plugin found
    /// - `GenjaError::InventoryLoad` - Inventory loading failed
    fn load_inventory_from_settings(&mut self) -> Result<(), GenjaError> {
        self.ensure_plugins_loaded()?;
        let inventory_cfg = self.settings.inventory();
        let plugin_name = inventory_cfg.plugin();

        if !plugin_name.is_empty() {
            if let Some(plugin) = self.plugins.get_inventory_plugin(plugin_name) {
                let inventory = plugin
                    .load(&self.settings, &self.plugins)
                    .map_err(|err| GenjaError::InventoryLoad(err.to_string()))?;
                self.load_inventory(inventory);
                return Ok(());
            }

            if self.plugins.get_plugin(plugin_name).is_some() {
                return Err(GenjaError::NotInventoryPlugin(plugin_name.to_string()));
            }

            return Err(GenjaError::PluginNotFound(plugin_name.to_string()));
        }

        let default_name = "FileInventoryPlugin";
        if let Some(plugin) = self.plugins.get_inventory_plugin(default_name) {
            let inventory = plugin
                .load(&self.settings, &self.plugins)
                .map_err(|err| GenjaError::InventoryLoad(err.to_string()))?;
            self.load_inventory(inventory);
            return Ok(());
        }

        if self.plugins.get_plugin(default_name).is_some() {
            return Err(GenjaError::NotInventoryPlugin(default_name.to_string()));
        }

        Err(GenjaError::PluginNotFound(default_name.to_string()))
    }

    /// Loads plugins from the plugin directory or registers default plugins.
    ///
    /// This method attempts to activate plugins using the plugin manager. If no
    /// plugins are found in the manifest, it falls back to registering the default
    /// `FileInventoryPlugin`.
    ///
    /// # Errors
    ///
    /// Returns `Err(GenjaError::PluginLoad)` if plugin loading fails for reasons
    /// other than missing plugin metadata.
    fn load_plugins(&mut self) -> Result<(), GenjaError> {
        let default_name = self.settings.inventory().plugin();
        match PluginManager::new().activate_plugins() {
            Ok(mut manager) => {
                if default_name == "FileInventoryPlugin"
                    && manager
                        .get_inventory_plugin("FileInventoryPlugin")
                        .is_none()
                {
                    manager.register_plugin(Plugins::Inventory(Box::new(
                        crate::plugins::DefaultInventoryPlugin,
                    )));
                }
                self.plugins = Arc::new(manager);
                self.plugins_loaded = true;
                Ok(())
            }
            Err(err) => {
                let msg = err.to_string();
                if msg.contains("No plugin metadata found in manifest") {
                    let mut manager = PluginManager::new();
                    if default_name == "FileInventoryPlugin"
                        && manager
                            .get_inventory_plugin("FileInventoryPlugin")
                            .is_none()
                    {
                        manager.register_plugin(Plugins::Inventory(Box::new(
                            crate::plugins::DefaultInventoryPlugin,
                        )));
                    }
                    self.plugins = Arc::new(manager);
                    self.plugins_loaded = true;
                    Ok(())
                } else {
                    Err(GenjaError::PluginLoad(err.to_string()))
                }
            }
        }
    }

    /// Loads an `Inventory` into the runtime and caches host identifiers.
    ///
    /// This replaces any previously loaded inventory and updates the internal
    /// host ID cache used by runtime operations.
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    /// use genja_core::inventory::{Inventory, Hosts};
    ///
    /// let inventory = Inventory::builder()
    ///     .hosts(Hosts::new())
    ///     .build();
    ///
    /// let mut genja = Genja::new();
    /// genja.load_inventory(inventory);
    /// ```
    pub fn load_inventory(&mut self, inventory: Inventory) {
        let factory = build_connection_factory(Arc::clone(&self.plugins));
        inventory.connections().set_connection_factory(factory);
        let host_ids = inventory.hosts().keys().cloned().collect();
        self.inventory = Some(Arc::new(inventory));
        self.host_ids = Arc::new(host_ids);
    }

    /// Returns `true` if plugins have been loaded for this instance.
    pub fn plugins_loaded(&self) -> bool {
        self.plugins_loaded
    }

    /// Returns `true` if inventory has been loaded into this instance.
    pub fn inventory_loaded(&self) -> bool {
        self.inventory.is_some()
    }

    /// Returns the current settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// Returns a reference to the loaded inventory, if available.
    ///
    /// # Errors
    ///
    /// Returns `GenjaError::InventoryNotLoaded` if no inventory has been loaded yet.
    pub fn inventory(&self) -> Result<&Inventory, GenjaError> {
        self.inventory
            .as_deref()
            .ok_or(GenjaError::InventoryNotLoaded)
    }

    /// Replaces the current settings with the provided configuration.
    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = Arc::new(settings);
    }

    /// Returns a reference to the plugin manager.
    pub fn plugin_manager(&self) -> &PluginManager {
        self.plugins.as_ref()
    }

    /// Guarded access for runner plugins.
    /// Runner plugins are not usable until inventory is loaded.
    pub fn get_runner_plugin(&self, name: &str) -> Result<&dyn PluginRunner, GenjaError> {
        self.ensure_plugins_loaded()?;
        self.ensure_inventory_loaded()?;

        let plugin = self
            .plugins
            .get_plugin(name)
            .ok_or_else(|| GenjaError::PluginNotFound(name.to_string()))?;

        match plugin {
            Plugins::Runner(runner) => Ok(runner.as_ref()),
            _ => Err(GenjaError::NotRunnerPlugin(name.to_string())),
        }
    }

    /// Returns the names of all available runner plugins.
    ///
    /// # Errors
    ///
    /// Returns `Err(GenjaError::PluginsNotLoaded)` if plugins are not loaded.
    /// Returns `Err(GenjaError::InventoryNotLoaded)` if inventory is not loaded.
    pub fn runner_plugin_names(&self) -> Result<Vec<String>, GenjaError> {
        self.ensure_plugins_loaded()?;
        self.ensure_inventory_loaded()?;
        Ok(self
            .plugins
            .get_all_plugin_names_and_groups()
            .into_iter()
            .filter_map(|(name, group)| if group == "Runner" { Some(name) } else { None })
            .collect())
    }

    /// Returns the number of currently selected hosts.
    pub fn host_count(&self) -> usize {
        self.host_ids.len()
    }

    /// Returns the currently selected host IDs.
    ///
    /// This list reflects any filtering applied via `filter_hosts`. To get all
    /// hosts in the inventory (with full host data), use `iter_inventory_hosts`.
    ///
    /// # See Also
    ///
    /// * [`host_count`](Self::host_count) - Get the number of selected hosts
    /// * [`filter_hosts`](Self::filter_hosts) - Filter hosts by predicate
    /// * [`iter_selected_hosts`](Self::iter_selected_hosts) - Get full host objects
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    /// use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};
    ///
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    /// hosts.add_host("router2", Host::builder().hostname("10.0.0.2").build());
    ///
    /// let inventory = Inventory::builder().hosts(hosts).build();
    /// let genja = Genja::from_inventory(inventory);
    ///
    /// // All hosts
    /// assert_eq!(genja.host_ids().len(), 2);
    ///
    /// // After filtering
    /// let filtered = genja.filter_hosts(|host| host.hostname() == Some("10.0.0.1"))?;
    /// assert_eq!(filtered.host_ids().len(), 1);
    /// assert_eq!(filtered.host_ids()[0].as_str(), "router1");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn host_ids(&self) -> &[NatString] {
        &self.host_ids
    }

    /// Returns the currently selected hosts, based on `host_ids`.
    ///
    /// This list reflects any prior filtering via `filter_hosts`.
    pub fn iter_selected_hosts(&self) -> Result<Vec<Host>, GenjaError> {
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(GenjaError::InventoryNotLoaded)?;

        Ok(self
            .host_ids
            .iter()
            .filter_map(|id| inventory.hosts().get(id))
            .collect())
    }

    /// Returns all hosts in the inventory with their IDs.
    ///
    /// This ignores any selection or filtering applied to `host_ids`.
    pub fn iter_inventory_hosts(&self) -> Result<Vec<(NatString, Host)>, GenjaError> {
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(GenjaError::InventoryNotLoaded)?;
        Ok(inventory
            .hosts()
            .iter()
            .map(|(id, host)| (id.clone(), host))
            .collect())
    }

    /// Returns a new `Genja` with hosts filtered by the provided predicate.
    ///
    /// The resulting instance shares the same inventory, settings, and plugins,
    /// but its host list is restricted to those that match `predicate_fn`.
    ///
    /// # Errors
    ///
    /// Returns `Err(GenjaError::InventoryNotLoaded)` if inventory has not been loaded.
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    /// use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};
    ///
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    /// hosts.add_host("router2", Host::builder().hostname("10.0.0.2").build());
    ///
    /// let inventory = Inventory::builder().hosts(hosts).build();
    /// let genja = Genja::from_inventory(inventory);
    ///
    /// let filtered = genja.filter_hosts(|host| host.hostname() == Some("10.0.0.1"))?;
    /// assert_eq!(filtered.host_ids().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn filter_hosts(&self, predicate_fn: impl Fn(&Host) -> bool) -> Result<Self, GenjaError> {
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(GenjaError::InventoryNotLoaded)?;

        let host_ids = self
            .host_ids
            .iter()
            .filter_map(|id| {
                inventory.hosts().get(id).and_then(|host| {
                    if predicate_fn(&host) {
                        Some(id.clone())
                    } else {
                        None
                    }
                })
            })
            .collect();

        Ok(Self {
            inventory: Some(Arc::clone(inventory)),
            host_ids: Arc::new(host_ids),
            settings: Arc::clone(&self.settings),
            plugins: Arc::clone(&self.plugins),
            plugins_loaded: self.plugins_loaded,
        })
    }

    fn ensure_plugins_loaded(&self) -> Result<(), GenjaError> {
        if self.plugins_loaded {
            Ok(())
        } else {
            Err(GenjaError::PluginsNotLoaded)
        }
    }

    fn ensure_inventory_loaded(&self) -> Result<(), GenjaError> {
        if self.inventory.is_some() {
            Ok(())
        } else {
            Err(GenjaError::InventoryNotLoaded)
        }
    }
    // TODO: Create a run function which tasks a Task definition
    // should be able to take any number of args, maybe serde_json::Value
    // There might need to be a TaskExecutor to handle failures and retries.
    // Run should use the selected PluginRunner to execute the tasks.
    pub fn run<T: Task + 'static>(
        &self,
        task: T,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(GenjaError::InventoryNotLoaded)?;
        let task_def = TaskDefinition::new(task);
        let mut results = TaskResults::new(task_def.name());

        for host_id in self.host_ids.iter() {
            let host = inventory
                .hosts()
                .get(host_id)
                .ok_or_else(|| GenjaError::Message(format!("host '{}' not found", host_id)))?;
            task_def.start(host_id.as_str(), &host, &mut results, max_depth)?;
        }

        Ok(results)
    }
}

impl Default for Genja {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing `Genja` instances with required inventory.
///
/// This builder provides a fluent interface for creating `Genja` objects with
/// a preloaded inventory and optional settings or plugin manager.
///
/// # Required Fields
///
/// * `inventory` - Must be provided via `new(inventory)`
///
/// # Optional Fields
///
/// * `settings` - Defaults to `Settings::default()`
/// * `plugin_manager` - Defaults to auto-loaded plugins
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use genja::Genja;
/// use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};
///
/// let mut hosts = Hosts::new();
/// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
/// let inventory = Inventory::builder().hosts(hosts).build();
///
/// let genja = Genja::builder(inventory)
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ## With Custom Settings
///
/// ```no_run
/// use genja::Genja;
/// use genja_core::{Settings, inventory::{Inventory, Hosts}};
///
/// let inventory = Inventory::builder().hosts(Hosts::new()).build();
/// let settings = Settings::from_file("config.yaml")?;
///
/// let genja = Genja::builder(inventory)
///     .with_settings(settings)
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ## With Custom Plugin Manager
///
/// ```
/// use genja::Genja;
/// use genja_core::inventory::{Inventory, Hosts};
/// use genja_plugin_manager::PluginManager;
///
/// let inventory = Inventory::builder().hosts(Hosts::new()).build();
/// let mut plugin_manager = PluginManager::new();
/// // ... register custom plugins
///
/// let genja = Genja::builder(inventory)
///     .with_plugin_manager(plugin_manager)
///     .build()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```

#[derive(Debug)]
pub struct GenjaBuilder {
    inventory: Inventory,
    settings: Option<Settings>,
    plugin_manager: Option<PluginManager>,
}

impl GenjaBuilder {
    pub fn new(inventory: Inventory) -> Self {
        Self {
            inventory,
            settings: None,
            plugin_manager: None,
        }
    }

    pub fn with_settings(mut self, settings: Settings) -> Self {
        self.settings = Some(settings);
        self
    }

    pub fn with_plugin_manager(mut self, plugin_manager: PluginManager) -> Self {
        self.plugin_manager = Some(plugin_manager);
        self
    }

    /// Builds a `Genja` instance from the configured builder state.
    ///
    /// Applies optional settings, initializes or loads plugins, and loads the
    /// required inventory into the resulting `Genja`.
    ///
    /// # Errors
    ///
    /// Returns `Err(GenjaError::PluginsNotLoaded)` if plugin loading fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::GenjaBuilder;
    /// use genja_core::inventory::{Inventory, Hosts};
    ///
    /// let inventory = Inventory::builder()
    ///     .hosts(Hosts::new())
    ///     .build();
    ///
    /// let genja = GenjaBuilder::new(inventory)
    ///     .build()
    ///     .expect("failed to build Genja");
    /// ```
    pub fn build(self) -> Result<Genja, GenjaError> {
        let mut genja = Genja::new();

        if let Some(settings) = self.settings {
            genja.set_settings(settings);
        }

        if let Some(plugin_manager) = self.plugin_manager {
            genja.plugins = Arc::new(plugin_manager);
            genja.plugins_loaded = true;
        } else {
            genja.load_plugins()?;
        }

        genja.load_inventory(self.inventory);
        Ok(genja)
    }
}
