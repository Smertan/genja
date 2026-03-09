use genja_core::inventory::{Host, Inventory, TransformFunctionOptions};
use genja_core::{NatString, Settings};
use plugin_manager::plugin_types::Plugins;
use plugin_manager::plugin_types::PluginRunner;
use plugin_manager::PluginManager;
use std::any::Any;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;

#[derive(Debug)]
pub enum GenjaError {
    PluginsNotLoaded,
    InventoryNotLoaded,
    PluginNotFound(String),
    NotRunnerPlugin(String),
    NotTransformPlugin(String),
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
            GenjaError::NotRunnerPlugin(name) => write!(f, "plugin '{name}' is not a runner plugin"),
            GenjaError::NotTransformPlugin(name) => {
                write!(f, "plugin '{name}' is not a transform function plugin")
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
/// 1) `load_plugins()` to discover/register plugins.
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
/// let genja = Genja::from_settings_file("settings.yml");
/// assert!(genja.is_ok());
/// ```
#[derive(Debug, Clone)]
pub struct Genja {
    inventory: Option<Arc<Inventory>>,
    host_ids: Arc<Vec<NatString>>,
    config: Arc<Settings>,
    plugins: Arc<PluginManager>,
    plugins_loaded: bool,
}

impl Default for Genja {
    fn default() -> Self {
        Self::new()
    }
}

impl Genja {
    pub fn new() -> Self {
        Self {
            inventory: None,
            host_ids: Arc::new(Vec::new()),
            config: Arc::new(Settings::default()),
            plugins: Arc::new(PluginManager::new()),
            plugins_loaded: false,
        }
    }

    pub fn from_inventory(inventory: Inventory) -> Self {
        let host_ids = inventory.hosts().keys().cloned().collect();
        Self {
            inventory: Some(Arc::new(inventory)),
            host_ids: Arc::new(host_ids),
            config: Arc::new(Settings::default()),
            plugins: Arc::new(PluginManager::new()),
            plugins_loaded: false,
        }
    }

    pub fn from_settings_file(file_path: &str) -> Result<Self, GenjaError> {
        let settings =
            Settings::from_file(file_path).map_err(|err| GenjaError::ConfigLoad(err.to_string()))?;

        let mut genja = Self::new();
        genja.set_config(settings);
        genja.load_plugins()?;
        genja.load_inventory_from_settings()?;
        Ok(genja)
    }

    pub fn load_inventory_from_settings(&mut self) -> Result<(), GenjaError> {
        self.ensure_plugins_loaded()?;

        let (hosts, groups, defaults) = self
            .config
            .inventory()
            .load_inventory_files()
            .map_err(GenjaError::InventoryLoad)?;

        let mut builder = Inventory::builder().hosts(hosts);
        if let Some(groups) = groups {
            builder = builder.groups(groups);
        }
        if let Some(defaults) = defaults {
            builder = builder.defaults(defaults);
        }

        if let Some(name) = self.config.inventory().transform_function() {
            let plugin = self
                .plugins
                .get_plugin(name)
                .ok_or_else(|| GenjaError::PluginNotFound(name.to_string()))?;

            match plugin {
                Plugins::TransformFunction(transform) => {
                    builder = builder.transform_function(transform.transform_function());
                }
                _ => return Err(GenjaError::NotTransformPlugin(name.to_string())),
            }

            if let Some(options) = self.config.inventory().transform_function_options() {
                builder = builder
                    .transform_function_options(TransformFunctionOptions::new(options.clone()));
            }
        }

        let inventory = builder.build();
        self.load_inventory(inventory);
        Ok(())
    }

    pub fn load_plugins(&mut self) -> Result<(), GenjaError> {
        match PluginManager::new().activate_plugins() {
            Ok(manager) => {
                self.plugins = Arc::new(manager);
                self.plugins_loaded = true;
                Ok(())
            }
            Err(err) => {
                let msg = err.to_string();
                if msg.contains("No plugin metadata found in manifest") {
                    self.plugins = Arc::new(PluginManager::new());
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

    pub fn config(&self) -> &Settings {
        &self.config
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

    pub fn set_config(&mut self, config: Settings) {
        self.config = Arc::new(config);
    }

    pub fn plugin_manager(&self) -> &PluginManager {
        self.plugins.as_ref()
    }

    pub fn execute_plugin(&self, name: &str, context: &dyn Any) -> Result<(), GenjaError> {
        self.ensure_plugins_loaded()?;
        self.plugins
            .execute_plugin(name, context)
            .map_err(GenjaError::PluginLoad)
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
            .filter_map(
                |(name, group)| if group == "Runner" { Some(name) } else { None },
            )
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
        Ok(inventory.hosts().iter().map(|(id, host)| (id.clone(), host)).collect())
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
                inventory.hosts().get(id).and_then(
                    |host| {
                        if pred(&host) { Some(id.clone()) } else { None }
                    },
                )
            })
            .collect();

        Ok(Self {
            inventory: Some(Arc::clone(inventory)),
            host_ids: Arc::new(host_ids),
            config: Arc::clone(&self.config),
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
