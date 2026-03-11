//! Plugin integration layer for Genja.
//!
//! This module wires Genja Core settings to the plugin system and provides
//! the default inventory plugin implementation used by the runtime.
//!
//! **Key points**
//! - Plugins are managed by `plugin_manager::PluginManager`.
//! - Inventory loading is delegated to an inventory plugin.
//! - `DefaultInventoryPlugin` loads files defined in settings and optionally
//!   applies a transform function plugin.
//!
//! # Configuration
//!
//! Inventory loading is driven by `Settings::inventory()`; it specifies the
//! inventory files, optional groups/defaults, and transform function settings.
//!
//! # Examples
//!
//! ```no_run
//! use genja::plugins::DefaultInventoryPlugin;
//! use genja_core::Settings;
//! use plugin_manager::PluginManager;
//!
//! let settings = Settings::default();
//! let plugins = PluginManager::new();
//! let inventory = DefaultInventoryPlugin
//!     .load(&settings, &plugins)
//!     .expect("inventory load failed");
//! ```
use genja_core::inventory::Inventory;
use genja_core::{InventoryLoadError, Settings};
use plugin_manager::PluginManager;
use plugin_manager::plugin_types::{Plugin, PluginInventory, Plugins};

pub struct DefaultInventoryPlugin;

impl Plugin for DefaultInventoryPlugin {
    fn name(&self) -> String {
        "FileInventoryPlugin".to_string()
    }
}

impl PluginInventory for DefaultInventoryPlugin {
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
