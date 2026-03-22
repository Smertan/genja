use genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
use genja_plugin_manager::plugin_types::{Plugin, PluginConnection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginB {
    key: Option<ConnectionKey>,
    alive: bool,
}

impl PluginB {
    fn with_key(key: ConnectionKey) -> Self {
        Self {
            key: Some(key),
            alive: false,
        }
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
        self.alive = true;
        Ok(())
    }

    fn close(&mut self) -> ConnectionKey {
        println!("Closing connection in Plugin B");
        self.alive = false;
        self.key
            .clone()
            .unwrap_or_else(|| ConnectionKey::new("plugin_b", "connection"))
    }

    fn is_alive(&self) -> bool {
        self.alive
    }
}

impl PluginB {
    pub fn new_prototype() -> Self {
        Self {
            key: None,
            alive: false,
        }
    }

    pub fn other_method(&self) {
        println!("Executing other method in Plugin B");
    }
}
