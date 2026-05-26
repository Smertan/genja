use std::sync::Arc;

use genja_core::task::Task;
use genja_core_derive::Task;

#[derive(Task)]
struct VecSubtasks {
    name: &'static str,
    #[task(subtask)]
    children: Vec<Arc<dyn Task>>,
}

fn main() {}
