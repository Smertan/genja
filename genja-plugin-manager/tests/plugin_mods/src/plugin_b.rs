use plugin_manager::plugin_types::{Plugin, PluginConnection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginB;

impl Plugin for PluginB {
    fn name(&self) -> String {
        String::from("plugin_b")
    }
}

impl PluginConnection for PluginB {
    fn open(&self) {
        println!("Opening connection in Plugin B");
    }

    fn close(&self) {
        println!("Closing connection in Plugin B");
    }

    fn connection(&self) {
        self.open();
        println!("Running connection in Plugin B");
        self.close();
    }
}

impl PluginB {
    pub fn other_method(&self) {
        println!("Executing other method in Plugin B");
    }
}
