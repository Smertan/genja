use crate::plugin_types::{Plugin, PluginInventory, Plugins};
use genja_core::inventory::Inventory;
use genja_core::InventoryLoadError;
use libloading::Library;
use std::collections::{HashMap, hash_map};

pub type PluginCreate = unsafe fn() -> Vec<Plugins>;
pub type PluginResult = Result<(Library, Vec<Plugins>), Box<dyn std::error::Error>>;

pub struct InventoryPlugins {
    plugins: HashMap<String, Box<dyn PluginInventory>>,
}

impl Default for InventoryPlugins {
    fn default() -> Self {
        Self::new()
    }
}

impl InventoryPlugins {
    pub fn new() -> Self {
        InventoryPlugins {
            plugins: HashMap::new(),
        }
    }

    pub fn add_plugin(&mut self, name: String, plugin: Box<dyn PluginInventory>) {
        if let hash_map::Entry::Vacant(entry) = self.plugins.entry(name.clone()) {
            entry.insert(plugin);
        } else {
            let msg = format!("Plugin '{}' already registered", &name);
            log::error!("{msg}");
            panic!("{msg}");
        }
        // self.plugins.push(plugin);
    }

    #[allow(clippy::borrowed_box)]
    pub fn get_plugin(&self, name: &str) -> Option<&Box<dyn PluginInventory>> {
        self.plugins.get(name)
    }

    pub fn get_plugin_names(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    // pub fn get_plugins(&self) -> &[Box<dyn PluginInventory>] {
    //     &self.plugins.
    // }
}

impl PluginInventory for InventoryPlugins {
    fn load(&self) -> Result<Inventory, InventoryLoadError> {
        // Placeholder: return an empty inventory for now.
        Ok(Inventory::builder().build())
    }
}

impl Plugin for InventoryPlugins {
    fn name(&self) -> String {
        String::from("Inventory Plugins")
    }
}
