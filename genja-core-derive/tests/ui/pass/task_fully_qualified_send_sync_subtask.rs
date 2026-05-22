use genja_core::task::Task;
use genja_core_derive::Task;

#[derive(Task)]
struct ParentTask {
    name: &'static str,
    #[task(subtask)]
    child: std::sync::Arc<dyn Task + Send + Sync>,
}

fn main() {}
