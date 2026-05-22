use std::sync::Arc;

use genja_core::task::{SubTasks, Task};
use genja_core_derive::Task;

#[derive(Task)]
struct ParentTask {
    name: &'static str,
    #[task(subtask)]
    child: Arc<dyn Task>,
}

fn main() {
    fn assert_sub_tasks<T: SubTasks>() {}
    assert_sub_tasks::<ParentTask>();
}
