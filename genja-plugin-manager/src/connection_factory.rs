use crate::plugin_types::{PluginConnection, Plugins};
use crate::PluginManager;
use genja_core::inventory::{Connection, ConnectionFactory, ConnectionKey, ResolvedConnectionParams};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct PluginConnectionAdapter {
    inner: Box<dyn PluginConnection>,
    alive: bool,
}

impl PluginConnectionAdapter {
    fn new(inner: Box<dyn PluginConnection>) -> Self {
        Self { inner, alive: false }
    }
}

impl Connection for PluginConnectionAdapter {
    fn is_alive(&self) -> bool {
        self.alive
    }

    fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String> {
        let result = self.inner.open(params);
        if result.is_ok() {
            self.alive = true;
        }
        result
    }

    fn close(&mut self) -> ConnectionKey {
        let key = self.inner.close();
        self.alive = false;
        key
    }
}

pub fn build_connection_factory(plugins: Arc<PluginManager>) -> Arc<ConnectionFactory> {
    Arc::new(move |key: &ConnectionKey| {
        let plugin = plugins.get_plugin(&key.connection_type)?;
        match plugin {
            Plugins::Connection(connection) => {
                let instance = connection.create(key);
                let adapter = PluginConnectionAdapter::new(instance);
                Some(Arc::new(Mutex::new(adapter)) as Arc<Mutex<dyn Connection>>)
            }
            _ => None,
        }
    })
}
