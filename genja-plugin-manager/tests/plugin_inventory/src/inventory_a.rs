use genja_core::inventory::Inventory;
use genja_core::InventoryLoadError;
use plugin_manager::plugin_types::{Plugin, PluginInventory};

#[derive(Clone, PartialEq, Eq)]
pub struct InventoryA;

impl Plugin for InventoryA {
    fn name(&self) -> String {
        String::from("inventory_a")
    }
}
impl PluginInventory for InventoryA {
    fn load(&self) -> Result<Inventory, InventoryLoadError> {
        Ok(Inventory::builder().build())
    }
}
