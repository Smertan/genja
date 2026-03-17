use genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
use plugin_manager::plugin_types::{Plugin, PluginConnection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginB {
    key: Option<ConnectionKey>,
}

impl PluginB {
    fn with_key(key: ConnectionKey) -> Self {
        Self { key: Some(key) }
    }
}

impl Plugin for PluginB {
    fn name(&self) -> String {
        String::from("plugin_b")
    }
}

impl PluginConnection for PluginB {
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        Box::new(PluginB::with_key(key.clone()))
    }

    fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
        println!("Opening connection in Plugin B");
        Ok(())
    }

    fn close(&mut self) -> ConnectionKey {
        println!("Closing connection in Plugin B");
        self.key
            .clone()
            .unwrap_or_else(|| ConnectionKey::new("plugin_b", "connection"))
    }

    fn connection(&self) {
        println!("Running connection in Plugin B");
    }
}

impl PluginB {
    pub fn new_prototype() -> Self {
        Self { key: None }
    }

    pub fn other_method(&self) {
        println!("Executing other method in Plugin B");
    }
}
