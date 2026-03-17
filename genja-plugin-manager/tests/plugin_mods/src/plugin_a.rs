use genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
use plugin_manager::plugin_types::{Plugin, PluginConnection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginA {
    key: Option<ConnectionKey>,
}

impl PluginA {
    fn with_key(key: ConnectionKey) -> Self {
        Self { key: Some(key) }
    }
}

impl Plugin for PluginA {
    fn name(&self) -> String {
        String::from("plugin_a")
    }
}

impl PluginConnection for PluginA {
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        Box::new(PluginA::with_key(key.clone()))
    }

    fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
        println!("Opening connection in Plugin A");
        Ok(())
    }

    fn close(&mut self) -> ConnectionKey {
        println!("Closing connection in Plugin A");
        self.key
            .clone()
            .unwrap_or_else(|| ConnectionKey::new("plugin_a", "connection"))
    }

    fn connection(&self) {
        println!("Running connection in Plugin A");
    }
}

impl PluginA {
    pub fn new_prototype() -> Self {
        Self { key: None }
    }

    pub fn other_method(&self) {
        println!("Executing other method in Plugin A");
    }
}

#[unsafe(no_mangle)]
pub fn create_plugin() -> Box<dyn Plugin> {
    Box::new(PluginA::new_prototype())
}
