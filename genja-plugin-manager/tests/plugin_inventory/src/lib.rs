pub mod inventory_a;
use plugin_manager::plugin_types::Plugins;

#[unsafe(no_mangle)]
pub fn create_plugins() -> Vec<Plugins> {
    let plugins = vec![Plugins::Inventory(Box::new(inventory_a::InventoryA))];
    plugins
}
