use plugin_manager::plugin_types::{Plugin, PluginConnection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginA;

impl Plugin for PluginA {
    fn name(&self) -> String {
        String::from("plugin_a")
    }
}

impl PluginConnection for PluginA {
    fn open(&self) {
        println!("Opening connection in Plugin A");
    }

    fn close(&self) {
        println!("Closing connection in Plugin A");
    }

    fn connection(&self) {
        self.open();
        println!("Running connection in Plugin A");
        self.close();
    }
}

impl PluginA {
    pub fn other_method(&self) {
        println!("Executing other method in Plugin A");
    }
}

#[unsafe(no_mangle)]
pub fn create_plugin() -> Box<dyn Plugin> {
    Box::new(PluginA)
}
