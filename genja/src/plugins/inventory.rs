use genja_core::inventory::Inventory;
use genja_core::{InventoryLoadError, Settings};
use genja_plugin_manager::PluginManager;
use genja_plugin_manager::plugin_types::{Plugin, PluginInventory, Plugins};

/// Default file-based inventory plugin.
///
/// Loads inventory from JSON or YAML files specified in settings. This plugin
/// is automatically registered when no custom inventory plugin is configured.
///
/// # Examples
///
/// ```no_run
/// use genja::plugins::DefaultInventoryPlugin;
/// use genja_core::Settings;
/// use genja_plugin_manager::PluginManager;
/// use genja_plugin_manager::plugin_types::PluginInventory;
///
/// let settings = Settings::default();
/// let plugins = PluginManager::new();
/// let inventory = DefaultInventoryPlugin
///     .load(&settings, &plugins)
///     .expect("inventory load failed");
/// ```
pub struct DefaultInventoryPlugin;

impl Plugin for DefaultInventoryPlugin {
    fn name(&self) -> String {
        "FileInventoryPlugin".to_string()
    }
}

impl PluginInventory for DefaultInventoryPlugin {
    /// Loads inventory from files specified in settings.
    ///
    /// ```no_run
    /// use genja::plugins::DefaultInventoryPlugin;
    /// use genja_core::Settings;
    /// use genja_plugin_manager::PluginManager;
    /// use genja_plugin_manager::plugin_types::PluginInventory;
    ///
    /// let settings = Settings::from_file("config.yaml")?;
    /// let plugins = PluginManager::new();
    ///
    /// let inventory = DefaultInventoryPlugin.load(&settings, &plugins)?;
    /// println!("Loaded {} hosts", inventory.hosts().len());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn load(
        &self,
        settings: &Settings,
        plugins: &PluginManager,
    ) -> Result<Inventory, InventoryLoadError> {
        let inventory_cfg = settings.inventory();

        let (hosts, groups, defaults) = inventory_cfg
            .load_inventory_files()
            .map_err(InventoryLoadError::from)?;

        let mut builder = Inventory::builder().hosts(hosts);

        if let Some(groups) = groups {
            builder = builder.groups(groups);
        }
        if let Some(defaults) = defaults {
            builder = builder.defaults(defaults);
        }

        if let Some(name) = inventory_cfg.transform_function() {
            let plugin = plugins.get_plugin(name).ok_or_else(|| {
                InventoryLoadError::from(format!("Transform plugin '{}' not found", name))
            })?;

            match plugin {
                Plugins::TransformFunction(transform) => {
                    builder = builder.transform_function(transform.transform_function());
                }
                _ => {
                    return Err(InventoryLoadError::from(format!(
                        "Plugin '{}' is not a transform function plugin",
                        name
                    )));
                }
            }

            if let Some(options) = inventory_cfg.transform_function_options() {
                builder = builder.transform_function_options(options.clone());
            }
        }

        Ok(builder.build())
    }
}
