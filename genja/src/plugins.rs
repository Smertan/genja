use genja_core::inventory::Inventory;
use genja_core::{InventoryLoadError, Settings};
use plugin_manager::plugin_types::{Plugin, PluginInventory, Plugins};
use plugin_manager::PluginManager;

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
            let plugin = plugins
                .get_plugin(name)
                .ok_or_else(|| {
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
