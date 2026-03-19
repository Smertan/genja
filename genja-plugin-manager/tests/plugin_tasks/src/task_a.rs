use genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
use plugin_manager::plugin_types::{Plugin, PluginConnection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskA {
    key: Option<ConnectionKey>,
    alive: bool,
}

impl TaskA {
    fn with_key(key: ConnectionKey) -> Self {
        Self {
            key: Some(key),
            alive: false,
        }
    }
}

impl Plugin for TaskA {
    fn name(&self) -> String {
        String::from("task_a")
    }
}

impl PluginConnection for TaskA {
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        Box::new(TaskA::with_key(key.clone()))
    }

    fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
        println!("Opening connection in Task A");
        self.alive = true;
        Ok(())
    }

    fn close(&mut self) -> ConnectionKey {
        println!("Closing connection in Task A");
        self.alive = false;
        self.key
            .clone()
            .unwrap_or_else(|| ConnectionKey::new("task_a", "connection"))
    }

    fn is_alive(&self) -> bool {
        self.alive
    }
}

impl TaskA {
    pub fn new_prototype() -> Self {
        Self {
            key: None,
            alive: false,
        }
    }

    pub fn other_method(&self) {
        println!("Executing other method in Task A");
    }
}
