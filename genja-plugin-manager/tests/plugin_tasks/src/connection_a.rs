use async_trait::async_trait;
use genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
use genja_plugin_manager::plugin_types::{Plugin, PluginConnection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionA {
    key: Option<ConnectionKey>,
    alive: bool,
}

impl ConnectionA {
    fn with_key(key: ConnectionKey) -> Self {
        Self {
            key: Some(key),
            alive: false,
        }
    }
}

impl Plugin for ConnectionA {
    fn name(&self) -> String {
        String::from("connection_a")
    }
}

#[async_trait]
impl PluginConnection for ConnectionA {
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        Box::new(ConnectionA::with_key(key.clone()))
    }

    async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
        println!("Opening connection in Connection A");
        self.alive = true;
        Ok(())
    }

    fn close(&mut self) -> ConnectionKey {
        println!("Closing connection in Connection A");
        self.alive = false;
        self.key
            .clone()
            .unwrap_or_else(|| ConnectionKey::new("connection_a", "connection"))
    }

    fn is_alive(&self) -> bool {
        self.alive
    }
}

impl ConnectionA {
    pub fn new_prototype() -> Self {
        Self {
            key: None,
            alive: false,
        }
    }

    pub fn other_method(&self) {
        println!("Executing other method in Connection A");
    }
}
