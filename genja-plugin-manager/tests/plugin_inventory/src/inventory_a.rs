use genja_core::inventory::Inventory;
use genja_core::{InventoryLoadError, Settings};
use plugin_manager::PluginManager;
use plugin_manager::plugin_types::{Plugin, PluginInventory};

#[derive(Clone, PartialEq, Eq)]
pub struct InventoryA;

impl Plugin for InventoryA {
    fn name(&self) -> String {
        String::from("inventory_a")
    }
}
impl PluginInventory for InventoryA {
    fn load(
        &self,
        _settings: &Settings,
        _plugins: &PluginManager,
    ) -> Result<Inventory, InventoryLoadError> {
        Ok(Inventory::builder().build())
    }
}
