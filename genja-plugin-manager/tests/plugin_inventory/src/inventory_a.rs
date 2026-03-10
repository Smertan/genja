use plugin_manager::plugin_types::{Plugin, PluginInventory};

#[derive(Clone, PartialEq, Eq)]
pub struct InventoryA;

impl Plugin for InventoryA {
    fn name(&self) -> String {
        String::from("inventory_a")
    }
}
impl PluginInventory for InventoryA {
    fn load(&self) {
        println!("Executing other method in Inventory A");
    }
}
