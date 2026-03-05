use plugin_manager::plugin_types::{Plugin, PluginConnection};
use std::any::Any;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskA;

impl Plugin for TaskA {
    fn name(&self) -> String {
        String::from("task_a")
    }

    fn execute(&self, _context: &dyn Any) -> Result<(), Box<dyn std::error::Error>> {
        println!("Executing Task A");
        Ok(())
    }
}

impl PluginConnection for TaskA {
    fn open(&self) {
        println!("Opening connection in Task A");
    }

    fn close(&self) {
        println!("Closing connection in Task A");
    }

    fn connection(&self) {
        self.open();
        println!("Running connection in Task A");
        self.close();
    }
}

impl TaskA {
    pub fn other_method(&self) {
        println!("Executing other method in Task A");
    }
}
