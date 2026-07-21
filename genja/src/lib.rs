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
//! // Load settings and inventory from a settings file
//! let genja = Genja::from_settings_file("config.yaml")?;
//!
//! // Or load inventory from programmatic settings
//! let settings = Settings::from_file("config.yaml")?;
//! let genja = Genja::from_settings(settings)?;
//!
//! // Or provide inventory explicitly and use settings for runtime options
//! let settings = Settings::from_file("config.yaml")?;
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
//! ## Plugin Runtime Layout
//!
//! Built-in plugins are always available. Dynamic plugins are loaded from a
//! `plugins` directory beside the running executable. A typical build output
//! layout looks like:
//!
//! ```text
//! target/
//!   debug/
//!     your_app
//!     plugins/
//!       libyour_plugin.so
//! ```
//!
//! When using `genja_plugin_manager::build_support::copy_plugins_from_manifest()`
//! from an end-user application's `build.rs`, plugin artifacts declared in that
//! application's `[package.metadata.plugins]` are copied into this
//! profile-specific `plugins` directory automatically.
//!
//! See [`Genja`] for the main API and [`GenjaBuilder`] for construction patterns.

pub use ::async_trait::async_trait;
pub use genja_core;
pub use genja_core::GenjaError;
use genja_core::inventory::{Host, Hosts, Inventory};
use genja_core::settings::RunnerConfig;
use genja_core::task::{
    Task, TaskConnectionResolver, TaskDefinition, TaskInfo, TaskProcessorResolver, TaskResults,
    TaskResultsSummary, Tasks,
};
use genja_core::{ConfigLoadError, NatString, Settings};
pub use genja_core_derive::genja_task;
pub use genja_plugin_manager;
use genja_plugin_manager::PluginManager;
use genja_plugin_manager::connection_factory::build_connection_factory;
use genja_plugin_manager::plugin_types::{PluginRunner, Plugins};
use log::info;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::runtime::Builder;

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

#[derive(Debug, Clone)]
struct RuntimeTaskConnectionResolver {
    inventory: Arc<Inventory>,
}

impl RuntimeTaskConnectionResolver {
    fn new(inventory: Arc<Inventory>) -> Self {
        Self { inventory }
    }
}

#[async_trait]
impl TaskConnectionResolver for RuntimeTaskConnectionResolver {
    async fn resolve_task_connection(
        &self,
        task: &dyn Task,
        hostname: &str,
    ) -> Result<Option<Arc<tokio::sync::Mutex<dyn genja_core::inventory::Connection>>>, GenjaError>
    {
        let Some(key) = task.get_connection_key(hostname) else {
            return Ok(None);
        };

        let params = self
            .inventory
            .resolve_connection_params(hostname, &key.plugin_name)
            .ok_or_else(|| {
                GenjaError::Message(format!(
                    "failed to resolve connection params for host '{}' using plugin '{}'",
                    hostname, key.plugin_name
                ))
            })?;

        self.inventory
            .connections()
            .open_connection(&key, &params)
            .await
            .map_err(GenjaError::Message)
    }
}

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
    /// Initializes default settings and the built-in plugin manager, and derives
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
    /// or parsed.
    ///
    /// Returns `Err(GenjaError::PluginLoad)` if plugin discovery or dynamic plugin
    /// loading fails.
    ///
    /// Returns inventory-related errors from loading the configured inventory plugin,
    /// including `GenjaError::InventoryLoad`, `GenjaError::PluginNotFound`, and
    /// `GenjaError::NotInventoryPlugin`.
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
        let settings = Settings::from_file(settings_file_path).map_err(GenjaError::from)?;
        Self::from_validated_settings(settings)
    }

    /// Creates a `Genja` instance from an already constructed settings object.
    ///
    /// Validates the supplied settings, initializes plugins, and loads inventory
    /// using the inventory plugin configured by `settings.inventory()`. This is
    /// the programmatic equivalent of [`Self::from_settings_file`].
    ///
    /// # Errors
    ///
    /// Returns `Err(GenjaError::ConfigLoad)` if settings validation fails.
    ///
    /// Returns `Err(GenjaError::PluginLoad)` if plugin discovery or dynamic plugin
    /// loading fails.
    ///
    /// Returns inventory-related errors from loading the configured inventory plugin,
    /// including `GenjaError::InventoryLoad`, `GenjaError::PluginNotFound`, and
    /// `GenjaError::NotInventoryPlugin`.
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::Genja;
    /// use genja_core::Settings;
    ///
    /// let genja = Genja::from_settings(Settings::default());
    /// assert!(genja.is_ok());
    /// ```
    pub fn from_settings(settings: Settings) -> Result<Self, GenjaError> {
        settings
            .validate()
            .map_err(|err| GenjaError::ConfigLoad(ConfigLoadError::SshConfig(err)))?;
        Self::from_validated_settings(settings)
    }

    /// Creates a `Genja` instance from an already constructed settings object using async inventory loading.
    ///
    /// Validates the supplied settings, initializes plugins, and loads inventory
    /// using the async inventory plugin configured by `settings.inventory()`. This
    /// is the strict async programmatic equivalent of [`Self::from_settings`].
    ///
    /// This constructor does not fall back to synchronous inventory plugins. If
    /// the configured inventory plugin is sync-only, runtime construction fails with
    /// `GenjaError::SyncInventoryPluginRequiresSyncConstruction`.
    ///
    /// # Errors
    ///
    /// Returns `Err(GenjaError::ConfigLoad)` if settings validation fails.
    ///
    /// Returns `Err(GenjaError::PluginLoad)` if plugin discovery or dynamic plugin
    /// loading fails.
    ///
    /// Returns inventory-related errors from loading the configured async inventory
    /// plugin, including `GenjaError::InventoryLoad`, `GenjaError::PluginNotFound`,
    /// `GenjaError::NotInventoryPlugin`, and
    /// `GenjaError::SyncInventoryPluginRequiresSyncConstruction`.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja::Genja;
    /// use genja_core::settings::InventoryConfig;
    /// use genja_core::Settings;
    ///
    /// # async fn build_runtime() -> Result<(), Box<dyn std::error::Error>> {
    /// let settings = Settings::builder()
    ///     .inventory(
    ///         InventoryConfig::builder()
    ///             .plugin("api_inventory")
    ///             .build(),
    ///     )
    ///     .build();
    ///
    /// let genja = Genja::from_settings_async(settings).await?;
    /// assert!(genja.inventory_loaded());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn from_settings_async(settings: Settings) -> Result<Self, GenjaError> {
        settings
            .validate()
            .map_err(|err| GenjaError::ConfigLoad(ConfigLoadError::SshConfig(err)))?;
        Self::from_validated_settings_async(settings).await
    }

    fn from_validated_settings(settings: Settings) -> Result<Self, GenjaError> {
        let mut genja = Self::new();
        genja.set_settings(settings);
        genja.load_plugins()?;
        genja.load_inventory_from_settings()?;
        Ok(genja)
    }

    async fn from_validated_settings_async(settings: Settings) -> Result<Self, GenjaError> {
        let mut genja = Self::new();
        genja.set_settings(settings);
        genja.load_plugins()?;
        genja.load_inventory_from_settings_async_strict().await?;
        Ok(genja)
    }

    /// Creates a `Genja` instance from a settings file path using async inventory loading.
    ///
    /// This follows the same flow as [`Self::from_settings_file`], but requires
    /// the selected inventory plugin to be registered as an async inventory
    /// plugin. If the configured inventory plugin, or default
    /// `FileInventoryPlugin`, is sync-only, runtime construction fails with
    /// `GenjaError::SyncInventoryPluginRequiresSyncConstruction`.
    pub async fn from_settings_file_async(settings_file_path: &str) -> Result<Self, GenjaError> {
        let settings = Settings::from_file(settings_file_path).map_err(GenjaError::from)?;
        let mut genja = Self::new();
        genja.set_settings(settings);
        genja.load_plugins()?;
        genja.load_inventory_from_settings_async_strict().await?;
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
        let plugin_name = self.settings.inventory().plugin();

        if let Some(plugin) = self.plugins.get_inventory_plugin(plugin_name) {
            let inventory = plugin
                .load(&self.settings, &self.plugins)
                .map_err(GenjaError::from)?;
            self.load_inventory(inventory);
            return Ok(());
        }

        if self
            .plugins
            .get_async_inventory_plugin(plugin_name)
            .is_some()
        {
            return Err(GenjaError::AsyncInventoryPluginRequiresAsyncConstruction(
                plugin_name.to_string(),
            ));
        }

        if self.plugins.get_plugin(plugin_name).is_some() {
            return Err(GenjaError::NotInventoryPlugin(plugin_name.to_string()));
        }

        Err(GenjaError::PluginNotFound(plugin_name.to_string()))
    }

    /// Loads inventory from settings using only async inventory plugins.
    ///
    /// This helper enforces the strict async contract used by
    /// [`Self::from_settings_async`]. It rejects sync-only inventory plugins with
    /// `GenjaError::SyncInventoryPluginRequiresSyncConstruction` instead of
    /// falling back to synchronous loading.
    async fn load_inventory_from_settings_async_strict(&mut self) -> Result<(), GenjaError> {
        self.ensure_plugins_loaded()?;
        let plugin_name = self.settings.inventory().plugin();

        if let Some(plugin) = self.plugins.get_async_inventory_plugin(plugin_name) {
            let inventory = plugin
                .load_async(&self.settings, &self.plugins)
                .await
                .map_err(GenjaError::from)?;
            self.load_inventory(inventory);
            return Ok(());
        }

        if self.plugins.get_inventory_plugin(plugin_name).is_some() {
            return Err(GenjaError::SyncInventoryPluginRequiresSyncConstruction(
                plugin_name.to_string(),
            ));
        }

        if self.plugins.get_plugin(plugin_name).is_some() {
            return Err(GenjaError::NotInventoryPlugin(plugin_name.to_string()));
        }

        Err(GenjaError::PluginNotFound(plugin_name.to_string()))
    }

    /// Loads plugins from the executable-relative plugin directory.
    ///
    /// Built-in plugins are always registered first. Then any dynamic plugin
    /// libraries found in a sibling `plugins` directory next to the current
    /// executable are loaded and registered.
    fn load_plugins(&mut self) -> Result<(), GenjaError> {
        let plugin_dir =
            current_plugin_directory().map_err(|err| GenjaError::PluginLoad(err.to_string()))?;
        let manager = crate::plugins::built_in_plugin_manager()
            .load_plugins_from_directory(&plugin_dir)
            .map_err(|err| GenjaError::PluginLoad(err.to_string()))?;
        self.plugins = Arc::new(manager);
        self.plugins_loaded = true;
        Ok(())
    }

    /// Loads an `Inventory` into the runtime and caches host identifiers.
    ///
    /// This replaces any previously loaded inventory, wires the inventory's
    /// connection factory from the current plugin manager, and updates the internal
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
    /// must be registered as a runner plugin. The returned instance preserves the
    /// current runner options and limits, while changing only the selected runner
    /// plugin name.
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
    /// 3. Attaches the plugin manager as a processor resolver
    /// 4. Builds a runtime connection resolver from the loaded inventory
    /// 5. Obtains the configured runner plugin
    /// 6. Builds a Tokio runtime and executes the task across all selected hosts
    /// 7. Logs a summary of the results
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
    /// * `GenjaError::Message` - The internal async runtime could not be created
    /// * Other errors from the runner plugin's execution
    ///
    /// # Examples
    ///
    /// ```
    /// use genja::{Genja, genja_task};
    /// use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};
    /// use genja_core::task::{
    ///     HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
    /// };
    ///
    /// struct MyTask;
    ///
    /// #[genja_task(name = "my-task")]
    /// impl MyTask {
    ///     async fn start_async(
    ///         &self,
    ///         _host: &Host,
    ///         _context: &TaskRuntimeContext,
    ///     ) -> Result<HostTaskResult, TaskError> {
    ///         Ok(HostTaskResult::passed(TaskSuccess::new()))
    ///     }
    /// }
    ///
    /// let mut hosts = Hosts::new();
    /// hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
    /// let inventory = Inventory::builder().hosts(hosts).build();
    /// let genja = Genja::from_inventory(inventory);
    ///
    /// let results = genja.run_task(MyTask, 0)?;
    /// assert_eq!(results.passed_hosts().len(), 1);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn run_task<T: Task + 'static>(
        &self,
        task: T,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        ensure_sync_execution_outside_tokio("run_task()", "run_task_async()")?;
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|err| GenjaError::Message(format!("failed to build async runtime: {err}")))?;
        runtime.block_on(self.run_task_async(task, max_depth))
    }

    /// Executes a task against the currently selected hosts using the configured runner plugin.
    ///
    /// This is the async counterpart to [`Self::run_task`]. Use it when Genja is called
    /// from an existing Tokio runtime or when task execution should compose with other
    /// async application work.
    pub async fn run_task_async<T: Task + 'static>(
        &self,
        task: T,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        self.run_task_definition_async(TaskDefinition::new(task), max_depth)
            .await
    }

    async fn run_task_definition_async(
        &self,
        task_definition: TaskDefinition,
        max_depth: usize,
    ) -> Result<TaskResults, GenjaError> {
        let hosts = self.selected_hosts()?;
        let host_count = hosts.len();
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(GenjaError::InventoryNotLoaded)?;
        let processor_resolver: Arc<dyn TaskProcessorResolver> = self.plugins.clone();
        let connection_resolver: Arc<dyn TaskConnectionResolver> =
            Arc::new(RuntimeTaskConnectionResolver::new(Arc::clone(inventory)));
        let task_definition = task_definition.with_processor_resolver(processor_resolver);
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
        let results = runner
            .run_task(
                &task_definition,
                &hosts,
                Some(connection_resolver),
                self.settings.runner(),
                max_depth,
            )
            .await?;
        let summary = results.task_summary();
        log_task_summary(&summary, host_count, 0);
        Ok(results)
    }

    /// Executes an ordered list of root task trees using the configured runner plugin.
    ///
    /// Each root task in `tasks` may have its own nested sub-tasks. The returned vector
    /// preserves the order of the root task list, so `results[n]` corresponds to
    /// `tasks[n]`.
    pub fn run_tasks(
        &self,
        tasks: Tasks,
        max_depth: usize,
    ) -> Result<Vec<TaskResults>, GenjaError> {
        ensure_sync_execution_outside_tokio("run_tasks()", "run_tasks_async()")?;
        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|err| GenjaError::Message(format!("failed to build async runtime: {err}")))?;
        runtime.block_on(self.run_tasks_async(tasks, max_depth))
    }

    /// Executes an ordered list of root task trees using the configured runner plugin.
    ///
    /// This is the async counterpart to [`Self::run_tasks`]. Use it when Genja is called
    /// from an existing Tokio runtime or when task execution should compose with other
    /// async application work.
    pub async fn run_tasks_async(
        &self,
        mut tasks: Tasks,
        max_depth: usize,
    ) -> Result<Vec<TaskResults>, GenjaError> {
        let hosts = self.selected_hosts()?;
        let host_count = hosts.len();
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(GenjaError::InventoryNotLoaded)?;
        let processor_resolver: Arc<dyn TaskProcessorResolver> = self.plugins.clone();
        let connection_resolver: Arc<dyn TaskConnectionResolver> =
            Arc::new(RuntimeTaskConnectionResolver::new(Arc::clone(inventory)));
        for task_definition in tasks.iter_mut() {
            *task_definition = task_definition
                .clone()
                .with_processor_resolver(processor_resolver.clone());
        }

        let runner_name = self.settings.runner().plugin();
        let task_names = tasks
            .iter()
            .map(|task| task.name())
            .collect::<Vec<_>>()
            .join(", ");
        info!(
            "executing {} task(s) with runner='{}' selected_hosts={} max_depth={} tasks=[{}]",
            tasks.len(),
            runner_name,
            host_count,
            max_depth,
            task_names
        );
        let runner = self.get_runner_plugin(runner_name)?;
        let results = runner
            .run_tasks(
                &tasks,
                &hosts,
                Some(connection_resolver),
                self.settings.runner(),
                max_depth,
            )
            .await?;
        for result in &results {
            let summary = result.task_summary();
            log_task_summary(&summary, host_count, 0);
        }
        Ok(results)
    }
}

fn current_plugin_directory() -> Result<PathBuf, std::io::Error> {
    let executable = std::env::current_exe()?;
    let directory = executable
        .parent()
        .ok_or_else(|| std::io::Error::other("executable has no parent directory"))?;
    Ok(directory.join("plugins"))
}

fn ensure_sync_execution_outside_tokio(sync_api: &str, async_api: &str) -> Result<(), GenjaError> {
    if tokio::runtime::Handle::try_current().is_ok() {
        let message = format!(
            "{sync_api} cannot be called from an active Tokio runtime; use {async_api} instead"
        );
        log::error!("{message}");
        return Err(GenjaError::Message(message));
    }

    Ok(())
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
                .ok_or_else(|| GenjaError::Message(format!("host '{host_id}' not found")))?;
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
    use super::{Genja, GenjaError, genja_task};
    use async_trait::async_trait;
    use genja_core::Settings;
    use genja_core::inventory::{
        BaseBuilderHost, Connection, ConnectionKey, Data, Host, Hosts, Inventory,
        ResolvedConnectionParams,
    };
    use genja_core::settings::{InventoryConfig, OptionsConfig, RunnerConfig, SSHConfig};
    use genja_core::task::RetryConfig;
    use genja_core::task::{
        BlockingTaskRuntimeContext, HostTaskResult, Task, TaskDefinition, TaskError,
        TaskExecutionMode, TaskFailure, TaskInfo, TaskRuntimeContext, TaskSuccess, Tasks,
    };
    use genja_plugin_manager::PluginManager;
    use genja_plugin_manager::plugin_types::{
        AsyncPluginInventory, Plugin, PluginConnection, Plugins,
    };
    use serde_json::{Value, json};
    use std::fs;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    struct TestTask {
        name: String,
    }

    #[derive(Debug)]
    struct TestAsyncInventoryPlugin;

    impl Plugin for TestAsyncInventoryPlugin {
        fn name(&self) -> String {
            "async_inventory".to_string()
        }
    }

    #[async_trait]
    impl AsyncPluginInventory for TestAsyncInventoryPlugin {
        async fn load_async(
            &self,
            _settings: &Settings,
            _plugins: &genja_plugin_manager::PluginManager,
        ) -> Result<Inventory, genja_core::InventoryLoadError> {
            let mut hosts = Hosts::new();
            hosts.add_host(
                "router1",
                Host::builder().hostname("10.0.0.1").platform("ios").build(),
            );
            Ok(Inventory::builder().hosts(hosts).build())
        }
    }

    impl TaskInfo for TestTask {
        fn name(&self) -> &str {
            &self.name
        }

        fn connection_plugin_name(&self) -> Option<&str> {
            None
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    #[async_trait]
    impl Task for TestTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    struct FailedTask;

    impl TaskInfo for FailedTask {
        fn name(&self) -> &str {
            "failed-task"
        }

        fn connection_plugin_name(&self) -> Option<&str> {
            None
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    #[async_trait]
    impl Task for FailedTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::failed(genja_core::task::TaskFailure::new(
                std::io::Error::other("boom"),
            )))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    struct SkippedTask;

    struct FlakyRetryTask {
        attempts: Arc<AtomicUsize>,
        succeed_on_attempt: usize,
    }

    impl TaskInfo for SkippedTask {
        fn name(&self) -> &str {
            "skipped-task"
        }

        fn connection_plugin_name(&self) -> Option<&str> {
            None
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    #[async_trait]
    impl Task for SkippedTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::skipped_with_reason("filtered"))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    impl TaskInfo for FlakyRetryTask {
        fn name(&self) -> &str {
            "flaky-retry-task"
        }

        fn connection_plugin_name(&self) -> Option<&str> {
            None
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    #[async_trait]
    impl Task for FlakyRetryTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            let attempt = self.attempts.fetch_add(1, Ordering::SeqCst) + 1;
            if attempt < self.succeed_on_attempt {
                return Ok(HostTaskResult::failed(
                    TaskFailure::new(std::io::Error::other("temporary failure"))
                        .with_retryable(true),
                ));
            }

            Ok(HostTaskResult::passed(
                TaskSuccess::new().with_summary("recovered after retry"),
            ))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    struct ChildTask;

    impl TaskInfo for ChildTask {
        fn name(&self) -> &str {
            "child-task"
        }

        fn connection_plugin_name(&self) -> Option<&str> {
            None
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    #[async_trait]
    impl Task for ChildTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    struct ParentTask;

    struct RecordingTask {
        name: &'static str,
        sub_tasks: Vec<Arc<dyn Task>>,
        order: Arc<Mutex<Vec<&'static str>>>,
    }

    #[derive(Debug)]
    struct TestConnectionPlugin;

    #[derive(Debug)]
    struct TestRuntimeConnection {
        key: ConnectionKey,
        alive: bool,
    }

    struct ConnectionAwareTask {
        saw_connection: Arc<AtomicBool>,
    }

    struct BlockingSuccessTask;

    struct SlowBlockingTask {
        started: Arc<AtomicBool>,
        finished: Arc<AtomicBool>,
        release: Arc<std::sync::Barrier>,
    }

    struct DerivedProcessorTask;

    struct DerivedAttributeProcessorTask;

    #[genja_task(name = "attribute", processors = ["audit", "metrics"])]
    impl DerivedAttributeProcessorTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    impl TaskInfo for ParentTask {
        fn name(&self) -> &str {
            "parent-task"
        }

        fn connection_plugin_name(&self) -> Option<&str> {
            None
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    #[async_trait]
    impl Task for ParentTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }

        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            vec![Arc::new(ChildTask)]
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    impl RecordingTask {
        fn leaf(name: &'static str, order: Arc<Mutex<Vec<&'static str>>>) -> Self {
            Self {
                name,
                sub_tasks: Vec::new(),
                order,
            }
        }

        fn parent(
            name: &'static str,
            sub_tasks: Vec<Arc<dyn Task>>,
            order: Arc<Mutex<Vec<&'static str>>>,
        ) -> Self {
            Self {
                name,
                sub_tasks,
                order,
            }
        }
    }

    impl TaskInfo for RecordingTask {
        fn name(&self) -> &str {
            self.name
        }

        fn connection_plugin_name(&self) -> Option<&str> {
            None
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    #[async_trait]
    impl Task for RecordingTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            self.order
                .lock()
                .expect("order mutex should not be poisoned")
                .push(self.name);
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }

        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            self.sub_tasks.clone()
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    impl Plugin for TestConnectionPlugin {
        fn name(&self) -> String {
            "test".to_string()
        }
    }

    #[async_trait]
    impl PluginConnection for TestConnectionPlugin {
        fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
            Box::new(TestRuntimeConnection {
                key: key.clone(),
                alive: false,
            })
        }

        async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
            Err("factory should not be opened directly".to_string())
        }

        fn close(&mut self) -> ConnectionKey {
            ConnectionKey::new("", "test")
        }

        fn is_alive(&self) -> bool {
            false
        }
    }

    impl Plugin for TestRuntimeConnection {
        fn name(&self) -> String {
            "test".to_string()
        }
    }

    #[async_trait]
    impl PluginConnection for TestRuntimeConnection {
        fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
            Box::new(Self {
                key: key.clone(),
                alive: false,
            })
        }

        async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
            self.alive = true;
            Ok(())
        }

        fn close(&mut self) -> ConnectionKey {
            self.alive = false;
            self.key.clone()
        }

        fn is_alive(&self) -> bool {
            self.alive
        }
    }

    #[async_trait]
    impl Connection for TestRuntimeConnection {
        fn create(&self, key: &ConnectionKey) -> Box<dyn Connection> {
            Box::new(Self {
                key: key.clone(),
                alive: false,
            })
        }

        fn is_alive(&self) -> bool {
            self.alive
        }

        async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
            self.alive = true;
            Ok(())
        }

        fn close(&mut self) -> ConnectionKey {
            self.alive = false;
            self.key.clone()
        }
    }

    impl TaskInfo for ConnectionAwareTask {
        fn name(&self) -> &str {
            "connection-aware"
        }

        fn connection_plugin_name(&self) -> Option<&str> {
            Some("test")
        }
    }

    #[async_trait]
    impl Task for ConnectionAwareTask {
        async fn start_async(
            &self,
            _host: &Host,
            context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            let alive = if let Some(connection) = context.connection() {
                let guard = connection.lock().await;
                guard.is_alive()
            } else {
                false
            };
            self.saw_connection.store(alive, Ordering::SeqCst);
            Ok(HostTaskResult::passed(
                TaskSuccess::new().with_changed(alive),
            ))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    #[genja_task(name = "derived")]
    impl DerivedProcessorTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }
    }

    impl TaskInfo for BlockingSuccessTask {
        fn name(&self) -> &str {
            "blocking-success"
        }
    }

    impl Task for BlockingSuccessTask {
        fn start(
            &self,
            _host: &Host,
            _context: &BlockingTaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Blocking
        }
    }

    impl TaskInfo for SlowBlockingTask {
        fn name(&self) -> &str {
            "slow-blocking"
        }
    }

    impl Task for SlowBlockingTask {
        fn start(
            &self,
            _host: &Host,
            _context: &BlockingTaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            self.started.store(true, Ordering::SeqCst);
            self.release.wait();
            self.finished.store(true, Ordering::SeqCst);
            Ok(HostTaskResult::passed(TaskSuccess::new()))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Blocking
        }
    }

    fn test_inventory() -> Inventory {
        let mut hosts = Hosts::new();
        hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
        hosts.add_host("router2", Host::builder().hostname("10.0.0.2").build());

        Inventory::builder().hosts(hosts).build()
    }

    fn single_host_inventory() -> Inventory {
        let mut hosts = Hosts::new();
        hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());

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

    fn temp_test_dir(name: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "genja-runtime-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp test dir should be created");
        dir
    }

    #[test]
    fn run_executes_task_for_each_selected_host() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja
            .run_task(
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
        let task = TaskDefinition::new(DerivedProcessorTask)
            .with_processor("audit")
            .with_processors(["metrics"]);
        let no_processors = DerivedProcessorTask;
        let attribute_task = DerivedAttributeProcessorTask;

        assert_eq!(task.processor_names(), vec!["audit", "metrics"]);
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
            .run_task(
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
    fn from_settings_loads_file_inventory_from_programmatic_settings() {
        let temp_dir = temp_test_dir("from-settings-file-inventory");
        let hosts_path = temp_dir.join("hosts.yaml");
        fs::write(
            &hosts_path,
            "router1:\n  hostname: 10.0.0.1\n  platform: ios\n",
        )
        .expect("hosts file should be written");

        let settings = Settings::builder()
            .inventory(
                InventoryConfig::builder()
                    .options(
                        OptionsConfig::builder()
                            .hosts_file(hosts_path.to_string_lossy())
                            .build(),
                    )
                    .build(),
            )
            .runner(RunnerConfig::builder().plugin("serial").build())
            .build();

        let genja = Genja::from_settings(settings).expect("runtime should build from settings");

        assert!(genja.plugins_loaded());
        assert!(genja.inventory_loaded());
        assert_eq!(genja.host_count(), 1);
        assert_eq!(genja.host_ids()[0].as_str(), "router1");
        fs::remove_dir_all(&temp_dir).unwrap_or(());
    }

    #[test]
    fn from_settings_uses_omitted_inventory_defaults() {
        let genja = Genja::from_settings(Settings::default())
            .expect("default settings should build an empty file inventory");

        assert!(genja.inventory_loaded());
        assert_eq!(genja.host_count(), 0);
    }

    #[test]
    fn from_settings_validates_programmatic_settings() {
        let settings = Settings::builder()
            .ssh(
                SSHConfig::builder()
                    .config_file("/nonexistent/genja/ssh_config")
                    .build(),
            )
            .build();

        let error = Genja::from_settings(settings)
            .expect_err("invalid programmatic settings should be rejected");

        assert!(matches!(error, GenjaError::ConfigLoad(_)));
    }

    #[test]
    fn sync_inventory_loading_rejects_async_only_inventory_plugin() {
        let mut plugin_manager = genja_plugin_manager::PluginManager::new();
        plugin_manager.register_plugin(Plugins::AsyncInventory(Box::new(TestAsyncInventoryPlugin)));

        let settings = Settings::builder()
            .inventory(InventoryConfig::builder().plugin("async_inventory").build())
            .build();

        let mut genja = Genja::new();
        genja.set_settings(settings);
        genja.plugins = Arc::new(plugin_manager);
        genja.plugins_loaded = true;

        let error = genja
            .load_inventory_from_settings()
            .expect_err("sync settings load should reject async-only inventory plugins");

        assert!(matches!(
            error,
            GenjaError::AsyncInventoryPluginRequiresAsyncConstruction(name)
                if name == "async_inventory"
        ));
    }

    #[test]
    fn async_inventory_loading_supports_async_inventory_plugin() {
        let mut plugin_manager = genja_plugin_manager::PluginManager::new();
        plugin_manager.register_plugin(Plugins::AsyncInventory(Box::new(TestAsyncInventoryPlugin)));

        let settings = Settings::builder()
            .inventory(InventoryConfig::builder().plugin("async_inventory").build())
            .build();

        let mut genja = Genja::new();
        genja.set_settings(settings);
        genja.plugins = Arc::new(plugin_manager);
        genja.plugins_loaded = true;

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(genja.load_inventory_from_settings_async_strict())
            .expect("async settings load should support async inventory plugins");

        assert!(genja.inventory_loaded());
        assert_eq!(genja.host_ids().len(), 1);
        assert_eq!(genja.host_ids()[0].as_str(), "router1");
    }

    #[test]
    fn from_settings_async_loads_async_inventory_from_programmatic_settings() {
        let mut plugin_manager = genja_plugin_manager::PluginManager::new();
        plugin_manager.register_plugin(Plugins::AsyncInventory(Box::new(TestAsyncInventoryPlugin)));

        let settings = Settings::builder()
            .inventory(InventoryConfig::builder().plugin("async_inventory").build())
            .runner(RunnerConfig::builder().plugin("serial").build())
            .build();

        let mut genja = Genja::new();
        genja.set_settings(settings);
        genja.plugins = Arc::new(plugin_manager);
        genja.plugins_loaded = true;

        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(genja.load_inventory_from_settings_async_strict())
            .expect("strict async settings load should support async inventory plugins");

        assert!(genja.inventory_loaded());
        assert_eq!(genja.host_ids().len(), 1);
        assert_eq!(genja.host_ids()[0].as_str(), "router1");
        assert_eq!(genja.settings().inventory().plugin(), "async_inventory");
        assert_eq!(genja.settings().runner().plugin(), "serial");
    }

    #[test]
    fn from_settings_async_rejects_sync_only_inventory_plugin() {
        let settings = Settings::default();

        let error = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(Genja::from_settings_async(settings))
            .expect_err("strict async settings load should reject sync-only default inventory");

        assert!(matches!(
            error,
            GenjaError::SyncInventoryPluginRequiresSyncConstruction(name)
                if name == "FileInventoryPlugin"
        ));
    }

    #[test]
    fn from_settings_async_returns_missing_inventory_plugin_error() {
        let settings = Settings::builder()
            .inventory(
                InventoryConfig::builder()
                    .plugin("missing_inventory")
                    .build(),
            )
            .build();

        let error = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(Genja::from_settings_async(settings))
            .expect_err("missing async inventory plugin should be rejected");

        assert!(matches!(error, GenjaError::PluginNotFound(name) if name == "missing_inventory"));
    }

    #[test]
    fn from_settings_async_returns_not_inventory_plugin_error() {
        let settings = Settings::builder()
            .inventory(InventoryConfig::builder().plugin("serial").build())
            .build();

        let error = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(Genja::from_settings_async(settings))
            .expect_err("non-inventory plugin should be rejected");

        assert!(matches!(error, GenjaError::NotInventoryPlugin(name) if name == "serial"));
    }

    #[test]
    fn from_settings_async_validates_programmatic_settings() {
        let settings = Settings::builder()
            .ssh(
                SSHConfig::builder()
                    .config_file("/nonexistent/genja/ssh_config")
                    .build(),
            )
            .build();

        let error = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(Genja::from_settings_async(settings))
            .expect_err("invalid programmatic settings should be rejected");

        assert!(matches!(error, GenjaError::ConfigLoad(_)));
    }

    #[test]
    fn from_settings_file_async_rejects_sync_only_inventory_plugin() {
        let temp_dir = temp_test_dir("from-settings-file-async-strict");
        let settings_path = temp_dir.join("settings.yaml");
        fs::write(&settings_path, "").expect("settings file should be written");

        let error = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(Genja::from_settings_file_async(
                settings_path.to_str().unwrap(),
            ))
            .expect_err(
                "settings-file async constructor should reject sync-only default inventory",
            );

        assert!(matches!(
            error,
            GenjaError::SyncInventoryPluginRequiresSyncConstruction(name)
                if name == "FileInventoryPlugin"
        ));
        fs::remove_dir_all(&temp_dir).unwrap_or(());
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
            .run_task(
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
    fn run_preserves_retry_behavior_across_serial_and_threaded_runners() {
        for runner_plugin in ["serial", "threaded"] {
            let attempts = Arc::new(AtomicUsize::new(0));
            let settings = Settings::builder()
                .runner(
                    RunnerConfig::builder()
                        .plugin(runner_plugin)
                        .worker_count(2)
                        .retry(RetryConfig::builder().allow(true).max_attempts(3).build())
                        .build(),
                )
                .build();

            let genja = Genja::builder(single_host_inventory())
                .with_settings(settings)
                .build()
                .expect("genja should build with retry settings");

            let results = genja
                .run_task(
                    FlakyRetryTask {
                        attempts: Arc::clone(&attempts),
                        succeed_on_attempt: 2,
                    },
                    0,
                )
                .expect("runner should retry and succeed");

            assert_eq!(
                attempts.load(Ordering::SeqCst),
                2,
                "{runner_plugin} runner should execute exactly two attempts"
            );
            let host_result = results
                .host_result("router1")
                .expect("router1 should have a host result");
            assert!(host_result.is_passed());
            assert_eq!(host_result.execution_metadata().attempts(), 2);
            assert!(host_result.execution_metadata().retried());
            assert!(!host_result.execution_metadata().retry_exhausted());
        }
    }

    #[tokio::test]
    async fn run_task_async_executes_task_inside_existing_tokio_runtime() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja
            .run_task_async(
                TestTask {
                    name: "async-task".to_string(),
                },
                0,
            )
            .await
            .expect("async task execution should succeed");

        assert_eq!(results.task_name(), "async-task");
        assert_eq!(results.passed_hosts().len(), 2);
    }

    #[test]
    fn run_task_executes_blocking_task() {
        let genja = Genja::from_inventory(single_host_inventory());

        let results = genja
            .run_task(BlockingSuccessTask, 0)
            .expect("blocking task execution should succeed");

        assert_eq!(results.task_name(), "blocking-success");
        assert_eq!(results.passed_hosts().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_task_async_executes_blocking_task() {
        let genja = Genja::from_inventory(single_host_inventory());

        let results = genja
            .run_task_async(BlockingSuccessTask, 0)
            .await
            .expect("blocking task execution should succeed");

        assert_eq!(results.task_name(), "blocking-success");
        assert_eq!(results.passed_hosts().len(), 1);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn run_task_async_offloads_blocking_task_work() {
        let genja = Genja::from_inventory(single_host_inventory());
        let started = Arc::new(AtomicBool::new(false));
        let finished = Arc::new(AtomicBool::new(false));
        let release = Arc::new(std::sync::Barrier::new(2));

        let task = SlowBlockingTask {
            started: Arc::clone(&started),
            finished: Arc::clone(&finished),
            release: Arc::clone(&release),
        };

        let task_handle = tokio::spawn(async move { genja.run_task_async(task, 0).await });

        tokio::time::timeout(Duration::from_secs(1), async {
            while !started.load(Ordering::SeqCst) {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("blocking task should start");

        assert!(!finished.load(Ordering::SeqCst));
        assert!(!task_handle.is_finished());

        tokio::task::spawn_blocking(move || release.wait())
            .await
            .expect("blocking task should be released");

        let results = task_handle
            .await
            .expect("blocking task join should succeed");

        let results = results.expect("blocking task execution should succeed");
        assert_eq!(results.task_name(), "slow-blocking");
        assert_eq!(results.passed_hosts().len(), 1);
    }

    #[tokio::test]
    async fn run_task_errors_inside_existing_tokio_runtime() {
        let genja = Genja::from_inventory(test_inventory());

        let error = genja
            .run_task(
                TestTask {
                    name: "sync-task".to_string(),
                },
                0,
            )
            .expect_err("sync task execution should error inside Tokio");

        assert!(matches!(
            error,
            GenjaError::Message(message)
                if message.contains("run_task() cannot be called from an active Tokio runtime")
                    && message.contains("run_task_async()")
        ));
    }

    #[test]
    fn run_tasks_accepts_ordered_task_list_with_nested_subtasks() {
        let genja = Genja::from_inventory(single_host_inventory());
        let order = Arc::new(Mutex::new(Vec::new()));
        let validate_config = Arc::new(RecordingTask::leaf("validate_config", order.clone()));
        let upload_artifact = Arc::new(RecordingTask::leaf("upload_artifact", order.clone()));

        let mut tasks = Tasks::new();
        tasks.add_task(RecordingTask::leaf("collect_facts", order.clone()));
        tasks.add_task(RecordingTask::parent(
            "deploy_changes",
            vec![validate_config, upload_artifact],
            order.clone(),
        ));
        tasks.add_task(RecordingTask::leaf("collect_logs", order.clone()));

        let results = genja
            .run_tasks(tasks, 10)
            .expect("task list should execute");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].task_name(), "collect_facts");
        assert_eq!(results[1].task_name(), "deploy_changes");
        assert!(results[1].sub_task("validate_config").is_some());
        assert!(results[1].sub_task("upload_artifact").is_some());
        assert_eq!(results[2].task_name(), "collect_logs");

        let order = order.lock().expect("order mutex should not be poisoned");
        assert_eq!(
            order.as_slice(),
            [
                "collect_facts",
                "deploy_changes",
                "validate_config",
                "upload_artifact",
                "collect_logs"
            ]
        );
    }

    #[tokio::test]
    async fn run_tasks_async_preserves_root_task_order() {
        let genja = Genja::from_inventory(single_host_inventory());
        let order = Arc::new(Mutex::new(Vec::new()));
        let validate_config = Arc::new(RecordingTask::leaf("validate_config", order.clone()));
        let upload_artifact = Arc::new(RecordingTask::leaf("upload_artifact", order.clone()));

        let mut tasks = Tasks::new();
        tasks.add_task(RecordingTask::leaf("collect_facts", order.clone()));
        tasks.add_task(RecordingTask::parent(
            "deploy_changes",
            vec![validate_config, upload_artifact],
            order.clone(),
        ));
        tasks.add_task(RecordingTask::leaf("collect_logs", order.clone()));

        let results = genja
            .run_tasks_async(tasks, 10)
            .await
            .expect("async task list should execute");

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].task_name(), "collect_facts");
        assert_eq!(results[1].task_name(), "deploy_changes");
        assert!(results[1].sub_task("validate_config").is_some());
        assert!(results[1].sub_task("upload_artifact").is_some());
        assert_eq!(results[2].task_name(), "collect_logs");

        let order = order.lock().expect("order mutex should not be poisoned");
        assert_eq!(
            order.as_slice(),
            [
                "collect_facts",
                "deploy_changes",
                "validate_config",
                "upload_artifact",
                "collect_logs"
            ]
        );
    }

    #[tokio::test]
    async fn run_tasks_errors_inside_existing_tokio_runtime() {
        let genja = Genja::from_inventory(single_host_inventory());
        let mut tasks = Tasks::new();
        tasks.add_task(RecordingTask::leaf(
            "collect_facts",
            Arc::new(Mutex::new(Vec::new())),
        ));

        let error = genja
            .run_tasks(tasks, 0)
            .expect_err("sync task list execution should error inside Tokio");

        assert!(matches!(
            error,
            GenjaError::Message(message)
                if message.contains("run_tasks() cannot be called from an active Tokio runtime")
                    && message.contains("run_tasks_async()")
        ));
    }

    #[test]
    fn run_preserves_failed_host_outcomes_and_timing() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja.run_task(FailedTask, 0).expect("run should succeed");

        assert_eq!(results.failed_hosts().len(), 2);
        let host_result = results
            .host_result("router1")
            .expect("router1 should have a failed result");
        assert!(host_result.is_failed());
        assert!(host_result.execution_metadata().duration_ns().is_some());
        assert!(
            host_result
                .execution_metadata()
                .duration_display()
                .is_some()
        );
        assert!(results.duration_ns().is_some());
    }

    #[test]
    fn run_preserves_skipped_host_outcomes_in_summary() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja.run_task(SkippedTask, 0).expect("run should succeed");

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
    fn run_passes_open_connection_into_task_runtime_context() {
        let saw_connection = Arc::new(AtomicBool::new(false));
        let mut plugin_manager = PluginManager::new();
        plugin_manager.register_plugin(Plugins::Connection(Box::new(TestConnectionPlugin)));

        let genja = Genja::builder(test_inventory())
            .with_plugin_manager(plugin_manager)
            .build()
            .expect("genja should build with connection plugin");

        let results = genja
            .run_task(
                ConnectionAwareTask {
                    saw_connection: Arc::clone(&saw_connection),
                },
                0,
            )
            .expect("run should succeed");

        assert!(saw_connection.load(Ordering::SeqCst));
        assert_eq!(results.passed_hosts().len(), 2);
        assert!(
            results
                .host_result("router1")
                .expect("router1 result should exist")
                .is_passed()
        );
    }

    #[test]
    fn builder_with_custom_plugin_manager_still_loads_builtin_runners() {
        let mut plugin_manager = PluginManager::new();
        plugin_manager.register_plugin(Plugins::Connection(Box::new(TestConnectionPlugin)));

        let genja = Genja::builder(test_inventory())
            .with_plugin_manager(plugin_manager)
            .build()
            .expect("genja should build with merged built-in plugins");

        let updated = genja
            .with_runner("serial")
            .expect("serial runner should still be available");

        assert_eq!(updated.settings().runner().plugin(), "serial");
    }

    #[test]
    fn run_builds_recursive_sub_task_summary_with_duration() {
        let genja = Genja::from_inventory(test_inventory());

        let results = genja.run_task(ParentTask, 1).expect("run should succeed");

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
    /// Returns `Err(GenjaError::PluginLoad)` if plugin discovery or dynamic plugin
    /// loading fails.
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
            let mut merged_plugins = crate::plugins::built_in_plugin_manager();
            merged_plugins.merge(plugin_manager);
            genja.plugins = Arc::new(merged_plugins);
            genja.plugins_loaded = true;
        } else {
            genja.load_plugins()?;
        }

        genja.load_inventory(self.inventory);
        Ok(genja)
    }
}
