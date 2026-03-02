use plugin_manager::plugin_types::Plugin;
use std::any::Any;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginA;

impl Plugin for PluginA {
    fn name(&self) -> String {
        String::from("plugin_a")
    }

    fn execute(&self, _context: &dyn Any) -> Result<(), Box<dyn std::error::Error>> {
        println!("Executing Plugin A");
        Ok(())
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
