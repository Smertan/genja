use std::sync::Arc;

use genja_core::task::Task;
use genja_core_derive::Task;

#[derive(Task)]
struct OptionalSubtask {
    name: &'static str,
    #[task(subtask)]
    child: Option<Arc<dyn Task>>,
}

fn main() {}
