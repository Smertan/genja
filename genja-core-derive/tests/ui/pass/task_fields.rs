use genja_core::task::TaskInfo;
use genja_core_derive::Task;
use serde_json::json;

#[derive(Task)]
struct FieldTask {
    name: &'static str,
    connection_plugin_name: Option<String>,
    options: Option<serde_json::Value>,
    processor_names: Vec<String>,
}

fn main() {
    let task = FieldTask {
        name: "field-task",
        connection_plugin_name: Some("ssh".to_string()),
        options: Some(json!({"changed": false})),
        processor_names: Vec::new(),
    }
    .with_processor("audit")
    .with_processors(["metrics", "trace"]);

    assert_eq!(task.connection_plugin_name(), Some("ssh"));
    assert!(task.options().is_some());
    assert_eq!(task.processor_names(), vec!["audit", "metrics", "trace"]);
    let key = task.get_connection_key("router1").unwrap();
    assert_eq!(key.hostname, "router1");
    assert_eq!(key.plugin_name, "ssh");
}
