use std::sync::Arc;

use genja_core::task::Task as CoreTask;
use genja_core_derive::Task;

#[derive(Task)]
struct AliasSubtask {
    name: &'static str,
    #[task(subtask)]
    child: Arc<dyn CoreTask>,
}

fn main() {}
