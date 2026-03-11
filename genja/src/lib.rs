use genja_core::inventory::{Host, Inventory};
use genja_core::{NatString, Settings};
use plugin_manager::PluginManager;
use plugin_manager::plugin_types::{PluginRunner, Plugins};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Debug)]
pub enum GenjaError {
    PluginsNotLoaded,
    InventoryNotLoaded,
    PluginNotFound(String),
    NotInventoryPlugin(String),
    NotRunnerPlugin(String),
    PluginLoad(Box<dyn Error>),
    ConfigLoad(String),
    InventoryLoad(String),
}

impl Display for GenjaError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GenjaError::PluginsNotLoaded => write!(f, "plugins have not been loaded"),
            GenjaError::InventoryNotLoaded => write!(f, "inventory has not been loaded"),
            GenjaError::PluginNotFound(name) => write!(f, "plugin '{name}' not found"),
            GenjaError::NotInventoryPlugin(name) => {
                write!(f, "plugin '{name}' is not an inventory plugin")
            }
            GenjaError::NotRunnerPlugin(name) => {
                write!(f, "plugin '{name}' is not a runner plugin")
            }
            GenjaError::PluginLoad(err) => write!(f, "failed to load plugins: {err}"),
            GenjaError::ConfigLoad(err) => write!(f, "failed to load settings: {err}"),
            GenjaError::InventoryLoad(err) => write!(f, "failed to load inventory: {err}"),
        }
    }
}

impl Error for GenjaError {}

/// Runtime composition layer for Genja.
///
/// Lifecycle:
/// 1) (internal) load plugins to discover/register plugins.
/// 2) `load_inventory(...)` to set runtime inventory.
/// 3) call runner-related methods.
///
/// Note: The derived `Debug` output for `Genja` does not apply inventory transform
/// functions. If you print `Genja` for debugging, the inventory data shown is the
/// raw, untransformed inventory state.
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

    pub fn from_settings_file(settings_file_path: &str) -> Result<Self, GenjaError> {
        let settings = Settings::from_file(settings_file_path)
            .map_err(|err| GenjaError::ConfigLoad(err.to_string()))?;

        let mut genja = Self::new();
        genja.set_settings(settings);
        genja.load_plugins()?;
        genja.load_inventory_from_settings()?;
        Ok(genja)
    }

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
                    Err(GenjaError::PluginLoad(err))
                }
            }
        }
    }

    pub fn load_inventory(&mut self, inventory: Inventory) {
        let host_ids = inventory.hosts().keys().cloned().collect();
        self.inventory = Some(Arc::new(inventory));
        self.host_ids = Arc::new(host_ids);
    }

    pub fn plugins_loaded(&self) -> bool {
        self.plugins_loaded
    }

    pub fn inventory_loaded(&self) -> bool {
        self.inventory.is_some()
    }

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

    pub fn set_settings(&mut self, settings: Settings) {
        self.settings = Arc::new(settings);
    }

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

    pub fn host_count(&self) -> usize {
        self.host_ids.len()
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

    pub fn filter_hosts(&self, pred: impl Fn(&Host) -> bool) -> Result<Self, GenjaError> {
        let inventory = self
            .inventory
            .as_ref()
            .ok_or(GenjaError::InventoryNotLoaded)?;

        let host_ids = self
            .host_ids
            .iter()
            .filter_map(|id| {
                inventory
                    .hosts()
                    .get(id)
                    .and_then(|host| if pred(&host) { Some(id.clone()) } else { None })
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
}

impl Default for Genja {
    fn default() -> Self {
        Self::new()
    }
}


/// Builder for constructing `Genja` instances with required inventory.
///
/// This builder provides a fluent interface for creating `Genja` objects with
/// a preloaded inventory and optional settings or plugin manager. Fields that
/// are not explicitly set will use their default values when `build()` is called.
///
/// # Fields
///
/// * `inventory` - Required inventory instance used to initialize `Genja`.
/// * `settings` - Optional settings. When set, the provided settings are used.
/// * `plugin_manager` - Optional plugin manager. When set, the provided manager
///   is used for plugin loading and execution.
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
