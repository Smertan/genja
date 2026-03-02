use plugin_manager::plugin_types::Plugin;
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
impl TaskA {
    pub fn other_method(&self) {
        println!("Executing other method in Task A");
    }
}
