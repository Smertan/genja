use genja_core::task::TaskInfo;
use genja_core_derive::Task;

#[derive(Task)]
struct MinimalTask {
    name: String,
}

fn main() {
    let task = MinimalTask {
        name: "minimal".to_string(),
    };

    assert_eq!(task.name(), "minimal");
    assert_eq!(task.connection_plugin_name(), None);
    assert!(task.options().is_none());
    assert!(task.processor_names().is_empty());
}
