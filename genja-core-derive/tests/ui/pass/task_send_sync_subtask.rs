use std::sync::Arc;

use genja_core::task::Task;
use genja_core_derive::Task;

#[derive(Task)]
struct ParentTask {
    name: &'static str,
    #[task(subtask)]
    child: Arc<dyn Task + Send + Sync>,
}

fn main() {}
