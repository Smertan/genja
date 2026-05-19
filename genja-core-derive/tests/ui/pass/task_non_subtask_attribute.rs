use genja_core::task::SubTasks;
use genja_core_derive::Task;

#[derive(Task)]
struct NonSubtaskAttributeTask {
    name: &'static str,
    #[task(not_subtask)]
    label: String,
}

fn main() {
    let task = NonSubtaskAttributeTask {
        name: "task",
        label: "not a child task".to_string(),
    };

    assert!(task.sub_tasks().is_empty());
}
