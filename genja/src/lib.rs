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
use genja_core::inventory::{Host, Hosts, Inventory};
use genja_core::settings::RunnerConfig;
use genja_core::task::{
    Task, TaskDefinition, TaskInfo, TaskProcessorResolver, TaskResults, TaskResultsSummary,
};
use genja_core::{NatString, Settings};
use genja_plugin_manager::PluginManager;
use genja_plugin_manager::connection_factory::build_connection_factory;
use genja_plugin_manager::plugin_types::{PluginRunner, Plugins};
use log::info;
use std::sync::Arc;

// GenjaError is re-exported from genja-core.

mod filter;

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
            plugins: Arc::new(crate::plugins::built_in_plugin_manager()),
            plugins_loaded: true,
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
            plugins: Arc::new(crate::plugins::built_in_plugin_manager()),
            plugins_loaded: true,
        }
    }

    /// Creates a `Genja` instance from a settings file path.
    ///
    /// Loads settings, initializes plugins, and loads inventory based on the
    /// settings file. This is equivalent to calling [`Settings::from_file`],
    /// then [`Genja::new`], [`set_settings`](Self::set_settings), and the
    /// internal plugin and inventory loading steps.
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
        match PluginManager::new().activate_plugins() {
            Ok(mut manager) => {
                if manager
                    .get_inventory_plugin("FileInventoryPlugin")
                    .is_none()
                {
                    manager.register_plugin(Plugins::Inventory(Box::new(
                        crate::plugins::DefaultInventoryPlugin,
                    )));
                }
                if manager.get_runner_plugin("serial").is_none() {
                    manager.register_plugin(Plugins::Runner(Box::new(
                        crate::plugins::SerialRunnerPlugin,
                    )));
                }
                if manager.get_runner_plugin("threaded").is_none() {
                    manager.register_plugin(Plugins::Runner(Box::new(
                        crate::plugins::ThreadedRunnerPlugin,
                    )));
                }
                self.plugins = Arc::new(manager);
                self.plugins_loaded = true;
                Ok(())
            }
            Err(err) => {
                let msg = err.to_string();
                if msg.contains("No plugin metadata found in manifest") {
                    let manager = crate::plugins::built_in_plugin_manager();
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

    /// Returns a new `Genja` with the selected runner plugin activated.
    ///
    /// The named plugin must already be loaded in the current plugin manager and
    /// must be registered as a runner plugin.
    ///
    /// # Errors
    ///
    /// Returns `Err(GenjaError::PluginsNotLoaded)` if plugins are not loaded.
    /// Returns `Err(GenjaError::PluginNotFound)` if no plugin with that name exists.
    /// Returns `Err(GenjaError::NotRunnerPlugin)` if the named plugin is not a runner.
    pub fn with_runner(&self, runner: &str) -> Result<Self, GenjaError> {
        self.ensure_plugins_loaded()?;

        let plugin = self
            .plugins
            .get_plugin(runner)
            .ok_or_else(|| GenjaError::PluginNotFound(runner.to_string()))?;

        if !matches!(plugin, Plugins::Runner(_)) {
            return Err(GenjaError::NotRunnerPlugin(runner.to_string()));
        }

        let mut runner_config = RunnerConfig::builder()
            .plugin(runner)
            .options(self.settings.runner().options().clone())
            .max_task_depth(self.settings.runner().max_task_depth())
            .max_connection_attempts(self.settings.runner().max_connection_attempts());

        if let Some(worker_count) = self.settings.runner().worker_count() {
            runner_config = runner_config.worker_count(worker_count);
        }

        let runner_config = runner_config.build();

        let settings = Settings::builder()
            .core(self.settings.core().clone())
            .inventory(self.settings.inventory().clone())
            .ssh(self.settings.ssh().clone())
            .runner(runner_config)
            .logging(self.settings.logging().clone())
            .build();

        Ok(Self {
            inventory: self.inventory.clone(),
            host_ids: Arc::clone(&self.host_ids),
            settings: Arc::new(settings),
            plugins: Arc::clone(&self.plugins),
            plugins_loaded: self.plugins_loaded,
        })
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

    /// Returns a new `Genja` with hosts filtered by key/path existence.
    ///
    /// The key is searched recursively through fixed host fields and arbitrary
    /// nested `data` values. Plain keys match at any object level; dot paths
    /// such as `data.site.role` match from the root or any nested object.
    ///
    /// A key with a `null` value still counts as existing.
    ///
    /// # Parameters
    ///
    /// * `key` - The key or dot-separated path to search for in host data. Can be a simple
    ///   key name (e.g., `"role"`) which matches at any nesting level, or a dot path
    ///   (e.g., `"data.site.role"`) which matches from the root or any nested object.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` containing a new `Genja` instance with the same inventory,
    /// settings, and plugins, but with `host_ids` filtered to only include hosts where
    /// the specified key exists.
    ///
    /// # Errors
    ///
    /// * `GenjaError::InventoryNotLoaded` - No inventory has been loaded
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    /// use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost, Data};
    /// use serde_json::json;
    ///
    /// let mut hosts = Hosts::new();
    /// hosts.add_host(
    ///     "router1",
    ///     Host::builder()
    ///         .hostname("10.0.0.1")
    ///         .data(Data::new(json!({"site": {"role": "core"}})))
    ///         .build()
    /// );
    /// hosts.add_host(
    ///     "router2",
    ///     Host::builder()
    ///         .hostname("10.0.0.2")
    ///         .data(Data::new(json!({"rack": "r1"})))
    ///         .build()
    /// );
    ///
    /// let inventory = Inventory::builder().hosts(hosts).build();
    /// let genja = Genja::from_inventory(inventory);
    ///
    /// // Filter by nested key
    /// let filtered = genja.filter_by_key("site")?;
    /// assert_eq!(filtered.host_ids().len(), 1);
    /// assert_eq!(filtered.host_ids()[0].as_str(), "router1");
    ///
    /// // Filter by dot path
    /// let filtered = genja.filter_by_key("data.site.role")?;
    /// assert_eq!(filtered.host_ids().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn filter_by_key(&self, key: &str) -> Result<Self, GenjaError> {
        let key_filter = filter::KeyFilter::new(key);
        self.filter_hosts(|host| key_filter.matches(host))
    }

    /// Returns a new `Genja` with hosts filtered by a key/path and regex-compatible value.
    ///
    /// The key is searched recursively through fixed host fields and arbitrary
    /// nested `data` values. Plain keys match at any object level; dot paths
    /// such as `data.site.role` match from the root or any nested object.
    ///
    /// # Parameters
    ///
    /// * `key` - The key or dot-separated path to search for in host data. Can be a simple
    ///   key name (e.g., `"role"`) which matches at any nesting level, or a dot path
    ///   (e.g., `"data.site.role"`) which matches from the root or any nested object.
    /// * `value_pattern` - A regex-compatible pattern to match against the value found at
    ///   the specified key. The pattern follows standard regex syntax and is case-sensitive
    ///   unless specified otherwise in the pattern itself.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Self)` containing a new `Genja` instance with the same inventory,
    /// settings, and plugins, but with `host_ids` filtered to only include hosts where
    /// the specified key exists and its value matches the provided pattern.
    ///
    /// # Errors
    ///
    /// * `GenjaError::InventoryNotLoaded` - No inventory has been loaded
    /// * `GenjaError::Message` - Invalid regex pattern in `value_pattern`
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    /// use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost, Data};
    /// use serde_json::json;
    ///
    /// let mut hosts = Hosts::new();
    /// hosts.add_host(
    ///     "router1",
    ///     Host::builder()
    ///         .hostname("10.0.0.1")
    ///         .data(Data::new(json!({"site": {"role": "core"}})))
    ///         .build()
    /// );
    /// hosts.add_host(
    ///     "router2",
    ///     Host::builder()
    ///         .hostname("10.0.0.2")
    ///         .data(Data::new(json!({"site": {"role": "edge"}})))
    ///         .build()
    /// );
    ///
    /// let inventory = Inventory::builder().hosts(hosts).build();
    /// let genja = Genja::from_inventory(inventory);
    ///
    /// // Filter by nested key with regex
    /// let filtered = genja.filter_by_key_value("role", "^core$")?;
    /// assert_eq!(filtered.host_ids().len(), 1);
    /// assert_eq!(filtered.host_ids()[0].as_str(), "router1");
    ///
    /// // Filter by dot path
    /// let filtered = genja.filter_by_key_value("data.site.role", "edge")?;
    /// assert_eq!(filtered.host_ids().len(), 1);
    /// assert_eq!(filtered.host_ids()[0].as_str(), "router2");
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn filter_by_key_value(&self, key: &str, value_pattern: &str) -> Result<Self, GenjaError> {
        let value_filter = filter::ValueFilter::new(key, value_pattern)?;
        self.filter_hosts(|host| value_filter.matches(host))
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

    /// Executes a task against the currently selected hosts using the configured runner plugin.
    ///
    /// This method runs the provided task on all hosts that match the current selection
    /// (after any filtering via [`filter_hosts`](Self::filter_hosts)). It uses the runner
    /// plugin specified in the active settings and respects the maximum task depth for
    /// nested sub-tasks.
    ///
    /// The execution flow:
    /// 1. Retrieves the currently selected hosts
    /// 2. Wraps the task in a `TaskDefinition`
    /// 3. Obtains the configured runner plugin
    /// 4. Executes the task across all selected hosts
    /// 5. Logs a summary of the results
    ///
    /// # Parameters
    ///
    /// * `task` - The task to execute. Must implement the [`Task`] trait and be `'static`.
    ///   The task will be executed once per selected host.
    /// * `max_depth` - Maximum depth for recursive sub-task execution. A value of `0`
    ///   means only the top-level task will run. Higher values allow nested sub-tasks
    ///   to execute up to the specified depth.
    ///
    /// # Returns
    ///
    /// Returns `Ok(TaskResults)` containing the execution results for all hosts, including:
    /// - Individual host results (passed, failed, or skipped)
    /// - Timing information (start time, end time, duration)
    /// - Sub-task results (if `max_depth > 0`)
    /// - Aggregated summary statistics
    ///
    /// # Errors
    ///
    /// * `GenjaError::InventoryNotLoaded` - No inventory has been loaded
    /// * `GenjaError::PluginsNotLoaded` - Plugins have not been loaded
    /// * `GenjaError::PluginNotFound` - The configured runner plugin does not exist
    /// * `GenjaError::NotRunnerPlugin` - The configured plugin is not a runner plugin
    /// * Other errors from the runner plugin's execution
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    /// use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost, ConnectionKey};
    /// use genja_core::task::{Task, TaskInfo, TaskError, HostTaskResult, TaskSuccess, SubTasks};
    /// use serde_json::Value;
    /// use std::sync::Arc;
    ///
    /// struct MyTask;
    ///
    /// impl TaskInfo for MyTask {
    ///     fn name(&self) -> &str { "my-task" }
    ///     fn plugin_name(&self) -> &str { "test" }
    ///     fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
    ///         ConnectionKey::new(hostname, self.plugin_name())
    ///     }
    ///     fn options(&self) -> Option<&Value> { None }
    /// }
    ///
    /// impl SubTasks for MyTask {
    ///     fn sub_tasks(&self) -> Vec<Arc<dyn Task>> { Vec::new() }
    /// }
    ///
    /// impl Task for MyTask {
    ///     fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
    ///         Ok(HostTaskResult::passed(TaskSuccess::new()))
    ///     }
    /// }
    ///
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    /// let inventory = Inventory::builder().hosts(hosts).build();
    /// let genja = Genja::from_inventory(inventory);
    ///
    /// let results = genja.run(MyTask, 0)?;
    /// assert_eq!(results.passed_hosts().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn run<T: Task + 'static>(
        &self,
        task: T,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        let hosts = self.selected_hosts()?;
        let host_count = hosts.len();
        let processor_resolver: Arc<dyn TaskProcessorResolver> = self.plugins.clone();
        let task_definition = TaskDefinition::new(task).with_processor_resolver(processor_resolver);
        let runner_name = self.settings.runner().plugin();
        info!(
            "executing task '{}' with runner='{}' selected_hosts={} max_depth={}",
            task_definition.name(),
            runner_name,
            host_count,
            max_depth
        );
        info!(
            "starting task '{}' for {} host(s)",
            task_definition.name(),
            host_count
        );
        let runner = self.get_runner_plugin(runner_name)?;
        let results = runner.run(&task_definition, &hosts, self.settings.runner(), max_depth)?;
        let summary = results.task_summary();
        log_task_summary(&summary, host_count, 0);
        Ok(results)
    }
}

fn log_task_summary(summary: &TaskResultsSummary, host_count: usize, depth: usize) {
    let hosts = summary.hosts();
    let prefix = if depth == 0 {
        String::new()
    } else {
        format!("{}↳ ", "  ".repeat(depth - 1))
    };
    let duration_ms = summary.duration_ms().unwrap_or(0);
    let duration = summary
        .duration_display()
        .unwrap_or_else(|| "unknown".to_string());

    info!(
        "{}finished task '{}' for {} host(s): passed={}, failed={}, skipped={} duration_ms={} duration={}",
        prefix,
        summary.task_name(),
        host_count,
        hosts.passed(),
        hosts.failed(),
        hosts.skipped(),
        duration_ms,
        duration
    );

    for (_, sub_summary) in summary.sub_tasks().iter() {
        log_task_summary(sub_summary, hosts.total(), depth + 1);
    }
}

impl Genja {
    fn selected_hosts(&self) -> Result<Hosts, GenjaError> {
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(GenjaError::InventoryNotLoaded)?;
        let mut hosts = Hosts::new();

        for host_id in self.host_ids.iter() {
            let host = inventory
                .hosts()
                .get(host_id)
                .ok_or_else(|| GenjaError::Message(format!("host '{}' not found", host_id)))?;
            hosts.add_host(host_id.as_str(), host.clone());
        }

        Ok(hosts)
    }
}

impl Default for Genja {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{Genja, GenjaError};
    use genja_core::Settings;
    use genja_core::inventory::{BaseBuilderHost, ConnectionKey, Data, Host, Hosts, Inventory};
    use genja_core::settings::RunnerConfig;
    use genja_core::task::{HostTaskResult, SubTasks, Task, TaskError, TaskInfo, TaskSuccess};
    use serde_json::{Value, json};
    use std::sync::Arc;

    struct TestTask {
        name: String,
    }

    impl TaskInfo for TestTask {
        fn name(&self) -> &str {
            &self.name
        }

        fn plugin_name(&self) -> &str {
            "test"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, self.plugin_name())
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for TestTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            Vec::new()
        }
    }

    impl Task for TestTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    struct FailedTask;

    impl TaskInfo for FailedTask {
        fn name(&self) -> &str {
            "failed-task"
        }

        fn plugin_name(&self) -> &str {
            "test"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, self.plugin_name())
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for FailedTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            Vec::new()
        }
    }

    impl Task for FailedTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::failed(genja_core::task::TaskFailure::new(
                std::io::Error::other("boom"),
            )))
        }
    }

    struct SkippedTask;

    impl TaskInfo for SkippedTask {
        fn name(&self) -> &str {
            "skipped-task"
        }

        fn plugin_name(&self) -> &str {
            "test"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, self.plugin_name())
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for SkippedTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            Vec::new()
        }
    }

    impl Task for SkippedTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::skipped_with_reason("filtered"))
        }
    }

    struct ChildTask;

    impl TaskInfo for ChildTask {
        fn name(&self) -> &str {
            "child-task"
        }

        fn plugin_name(&self) -> &str {
            "test"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, self.plugin_name())
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for ChildTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            Vec::new()
        }
    }

    impl Task for ChildTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    struct ParentTask;

    #[derive(genja_core_derive::Task)]
    struct DerivedProcessorTask {
        name: &'static str,
        processor_names: Vec<String>,
    }

    #[derive(genja_core_derive::Task)]
    #[task(processors = ["audit", "metrics"])]
    struct DerivedAttributeProcessorTask {
        name: &'static str,
    }

    impl TaskInfo for ParentTask {
        fn name(&self) -> &str {
            "parent-task"
        }

        fn plugin_name(&self) -> &str {
            "test"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, self.plugin_name())
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for ParentTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            vec![Arc::new(ChildTask)]
        }
    }

    impl Task for ParentTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    impl Task for DerivedProcessorTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    impl Task for DerivedAttributeProcessorTask {
        fn start(&self, _host: &Host) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    fn test_inventory() -> Inventory {
        let mut hosts = Hosts::new();
        hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
        hosts.add_host("router2", Host::builder().hostname("10.0.0.2").build());

        Inventory::builder().hosts(hosts).build()
    }

    fn test_inventory_with_data() -> Inventory {
        let mut hosts = Hosts::new();
        hosts.add_host(
            "router1",
            Host::builder()
                .hostname("10.0.0.1")
                .platform("ios-xe")
                .data(Data::new(json!({
                    "site": {
                        "name": "lab-a",
                        "role": "core"
                    },
                    "metadata": {
                        "owner": null
                    },
                    "enabled": true,
                    "priority": 10
                })))
                .build(),
        );
        hosts.add_host(
            "router2",
            Host::builder()
                .hostname("10.0.0.2")
                .platform("junos")
                .data(Data::new(json!({
                    "site": {
                        "name": "lab-b",
                        "role": "edge"
                    },
                    "rack": "r1"
                })))
                .build(),
        );
        hosts.add_host(
            "router3",
            Host::builder()
                .hostname("10.0.0.3")
                .platform("linux")
                .data(Data::new(json!({
                    "rack": "r2"
                })))
                .build(),
        );

        Inventory::builder().hosts(hosts).build()
    }

    fn test_inventory_with_nested_array_data() -> Inventory {
        let mut hosts = Hosts::new();
        hosts.add_host(
            "router1",
            Host::builder()
                .data(Data::new(json!({
                    "site": {
                        "devices": [
                            {"role": "core"},
                            {"role": "edge"}
                        ]
                    }
                })))
                .build(),
        );

        Inventory::builder().hosts(hosts).build()
    }

    #[test]
    fn run_executes_task_for_each_selected_host() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja
            .run(
                TestTask {
                    name: "test-task".to_string(),
                },
                0,
            )
            .expect("task should execute for all hosts");

        assert_eq!(results.task_name(), "test-task");
        assert_eq!(results.passed_hosts().len(), 2);
        assert!(results.host_result("router1").is_some());
        assert!(results.host_result("router2").is_some());
    }

    #[test]
    fn derive_task_exposes_processor_names() {
        let task = DerivedProcessorTask {
            name: "derived",
            processor_names: Vec::new(),
        }
        .with_processor("audit")
        .with_processors(["metrics"]);
        let no_processors = DerivedProcessorTask {
            name: "none",
            processor_names: Vec::new(),
        };
        let attribute_task = DerivedAttributeProcessorTask { name: "attribute" };

        assert_eq!(task.processor_names(), ["audit", "metrics"]);
        assert_eq!(attribute_task.processor_names(), ["audit", "metrics"]);
        assert!(no_processors.processor_names().is_empty());
    }

    #[test]
    fn run_respects_filtered_hosts() {
        let genja = Genja::from_inventory(test_inventory());
        let filtered = genja
            .filter_hosts(|host| host.hostname() == Some("10.0.0.1"))
            .expect("host filtering should succeed");

        let results = filtered
            .run(
                TestTask {
                    name: "filtered-task".to_string(),
                },
                0,
            )
            .expect("task should execute for selected hosts");

        assert_eq!(results.passed_hosts().len(), 1);
        assert!(results.host_result("router1").is_some());
        assert!(results.host_result("router2").is_none());
    }

    #[test]
    fn filter_by_key_filters_hosts_by_nested_key_existence() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key("site")
            .expect("key filtering should succeed");

        assert_eq!(filtered.host_ids().len(), 2);
        assert_eq!(filtered.host_ids()[0].as_str(), "router1");
        assert_eq!(filtered.host_ids()[1].as_str(), "router2");
    }

    #[test]
    fn filter_by_key_filters_hosts_by_dot_path_existence() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key("data.site.name")
            .expect("key filtering should succeed");

        assert_eq!(filtered.host_ids().len(), 2);
        assert_eq!(filtered.host_ids()[0].as_str(), "router1");
        assert_eq!(filtered.host_ids()[1].as_str(), "router2");
    }

    #[test]
    fn filter_by_key_counts_null_as_existing() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key("metadata.owner")
            .expect("key filtering should succeed");

        assert_eq!(filtered.host_ids().len(), 1);
        assert_eq!(filtered.host_ids()[0].as_str(), "router1");
    }

    #[test]
    fn filter_by_key_filters_hosts_by_dot_path_inside_arrays() {
        let genja = Genja::from_inventory(test_inventory_with_nested_array_data());

        let filtered = genja
            .filter_by_key("site.devices.role")
            .expect("key filtering should succeed");

        assert_eq!(filtered.host_ids().len(), 1);
        assert_eq!(filtered.host_ids()[0].as_str(), "router1");
    }

    #[test]
    fn filter_by_key_with_empty_key_matches_no_hosts() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key("")
            .expect("key filtering should succeed");

        assert!(filtered.host_ids().is_empty());
    }

    #[test]
    fn filter_by_key_value_filters_hosts_by_nested_key_and_regex_value() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key_value("role", "^(core|distribution)$")
            .expect("value filtering should succeed");

        assert_eq!(filtered.host_ids().len(), 1);
        assert_eq!(filtered.host_ids()[0].as_str(), "router1");
    }

    #[test]
    fn filter_by_key_value_filters_hosts_by_dot_path() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key_value("data.site.name", "lab-b")
            .expect("value filtering should succeed");

        assert_eq!(filtered.host_ids().len(), 1);
        assert_eq!(filtered.host_ids()[0].as_str(), "router2");
    }

    #[test]
    fn filter_by_key_value_returns_error_for_invalid_regex() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let error = genja
            .filter_by_key_value("role", "*")
            .expect_err("invalid regex should return an error");

        assert!(
            matches!(error, GenjaError::Message(message) if message.contains("invalid value regex"))
        );
    }

    #[test]
    fn filter_by_key_value_with_empty_key_matches_no_hosts() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key_value("", ".*")
            .expect("value filtering should succeed");

        assert!(filtered.host_ids().is_empty());
    }

    #[test]
    fn filter_by_key_value_matches_scalar_values() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let enabled = genja
            .filter_by_key_value("enabled", "^true$")
            .expect("value filtering should succeed");
        let priority = genja
            .filter_by_key_value("priority", "^10$")
            .expect("value filtering should succeed");
        let owner = genja
            .filter_by_key_value("metadata.owner", "^null$")
            .expect("value filtering should succeed");

        assert_eq!(enabled.host_ids().len(), 1);
        assert_eq!(enabled.host_ids()[0].as_str(), "router1");
        assert_eq!(priority.host_ids().len(), 1);
        assert_eq!(priority.host_ids()[0].as_str(), "router1");
        assert_eq!(owner.host_ids().len(), 1);
        assert_eq!(owner.host_ids()[0].as_str(), "router1");
    }

    #[test]
    fn filter_by_key_value_matches_object_value_text() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key_value("site", "lab-b")
            .expect("value filtering should succeed");

        assert_eq!(filtered.host_ids().len(), 1);
        assert_eq!(filtered.host_ids()[0].as_str(), "router2");
    }

    #[test]
    fn filters_can_be_chained() {
        let genja = Genja::from_inventory(test_inventory_with_data());

        let filtered = genja
            .filter_by_key("site")
            .expect("key filtering should succeed")
            .filter_by_key_value("role", "edge")
            .expect("value filtering should succeed");

        assert_eq!(filtered.host_ids().len(), 1);
        assert_eq!(filtered.host_ids()[0].as_str(), "router2");
    }

    #[test]
    fn run_uses_threaded_runner_plugin() {
        let settings = Settings::builder()
            .runner(
                RunnerConfig::builder()
                    .plugin("threaded")
                    .worker_count(2)
                    .build(),
            )
            .build();

        let genja = Genja::builder(test_inventory())
            .with_settings(settings)
            .build()
            .expect("genja should build with threaded runner settings");

        let results = genja
            .run(
                TestTask {
                    name: "threaded-task".to_string(),
                },
                0,
            )
            .expect("threaded runner should execute the task");

        assert_eq!(results.task_name(), "threaded-task");
        assert_eq!(results.passed_hosts().len(), 2);
        assert!(results.started_at().is_some());
        assert!(results.finished_at().is_some());
        assert!(results.duration_ns().is_some());
    }

    #[test]
    fn run_preserves_failed_host_outcomes_and_timing() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja.run(FailedTask, 0).expect("run should succeed");

        assert_eq!(results.failed_hosts().len(), 2);
        let failure = results
            .host_result("router1")
            .and_then(genja_core::task::HostTaskResult::failure)
            .expect("router1 should have a failed result");
        assert!(failure.duration_ns().is_some());
        assert!(failure.duration_display().is_some());
        assert!(results.duration_ns().is_some());
    }

    #[test]
    fn run_preserves_skipped_host_outcomes_in_summary() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja.run(SkippedTask, 0).expect("run should succeed");

        assert_eq!(results.skipped_hosts().len(), 2);
        let summary = results.task_summary();
        assert_eq!(summary.hosts().passed(), 0);
        assert_eq!(summary.hosts().failed(), 0);
        assert_eq!(summary.hosts().skipped(), 2);
        assert!(
            results
                .host_result("router1")
                .expect("router1 result should exist")
                .is_skipped()
        );
    }

    #[test]
    fn run_builds_recursive_sub_task_summary_with_duration() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja.run(ParentTask, 1).expect("run should succeed");

        let summary = results.task_summary();
        assert!(summary.duration_ms().is_some());
        assert!(summary.duration_display().is_some());

        let child = summary
            .sub_tasks()
            .get("child-task")
            .expect("child summary should exist");
        assert_eq!(child.hosts().passed(), 2);
        assert_eq!(child.hosts().failed(), 0);
        assert_eq!(child.hosts().skipped(), 0);
        assert!(child.duration_ms().is_some());
        assert!(child.duration_display().is_some());
    }

    #[test]
    fn with_runner_returns_updated_genja_for_loaded_runner_plugin() {
        let settings = Settings::builder()
            .runner(
                RunnerConfig::builder()
                    .plugin("threaded")
                    .options(json!({"queue": "fast"}))
                    .worker_count(3)
                    .max_task_depth(7)
                    .max_connection_attempts(5)
                    .build(),
            )
            .build();

        let genja = Genja::builder(test_inventory())
            .with_settings(settings)
            .build()
            .expect("genja should build");

        let updated = genja
            .with_runner("serial")
            .expect("serial runner should be available");

        assert_eq!(genja.settings().runner().plugin(), "threaded");
        assert_eq!(updated.settings().runner().plugin(), "serial");
        assert_eq!(
            updated.settings().runner().options(),
            &json!({"queue": "fast"})
        );
        assert_eq!(updated.settings().runner().worker_count(), Some(3));
        assert_eq!(updated.settings().runner().max_task_depth(), 7);
        assert_eq!(updated.settings().runner().max_connection_attempts(), 5);
        assert_eq!(updated.host_ids().len(), genja.host_ids().len());
    }

    #[test]
    fn with_runner_returns_error_for_unknown_runner_plugin() {
        let genja = Genja::from_inventory(test_inventory());

        let error = genja
            .with_runner("missing-runner")
            .expect_err("missing runner should return an error");

        assert!(matches!(error, GenjaError::PluginNotFound(name) if name == "missing-runner"));
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
