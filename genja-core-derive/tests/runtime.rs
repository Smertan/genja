use std::sync::Arc;

use async_trait::async_trait;
use genja_core::inventory::Host;
use genja_core::task::{
    HostTaskResult, SubTasks, Task, TaskError, TaskInfo, TaskRuntimeContext, TaskSuccess,
};
use genja_core_derive::{DerefMacro, DerefMutMacro, Task as TaskDerive};
use serde_json::{Value, json};

#[derive(TaskDerive)]
struct StringNameTask {
    name: String,
}

#[derive(TaskDerive)]
struct StaticNameTask {
    name: &'static str,
}

#[derive(TaskDerive)]
struct StringConnectionTask {
    name: &'static str,
    connection_plugin_name: String,
}

#[derive(TaskDerive)]
struct StaticConnectionTask {
    name: &'static str,
    connection_plugin_name: &'static str,
}

#[derive(TaskDerive)]
struct OptionStringConnectionTask {
    name: &'static str,
    connection_plugin_name: Option<String>,
}

#[derive(TaskDerive)]
struct OptionStaticConnectionTask {
    name: &'static str,
    connection_plugin_name: Option<&'static str>,
}

#[derive(TaskDerive)]
struct OptionsTask {
    name: &'static str,
    options: Option<Value>,
}

#[derive(TaskDerive)]
struct DynamicProcessorsTask {
    name: &'static str,
    processor_names: Vec<String>,
}

#[derive(TaskDerive)]
#[task(processors = ["audit", "metrics"])]
struct StaticProcessorsTask {
    name: &'static str,
}

#[derive(TaskDerive)]
struct LeafTask {
    name: &'static str,
}

#[async_trait]
impl Task for LeafTask {
    async fn start(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

#[derive(TaskDerive)]
struct ParentTask {
    name: &'static str,
    #[task(subtask)]
    validate_config: Arc<dyn Task>,
    #[task(subtask)]
    verify_health: Arc<dyn Task>,
}

trait DerefTarget {
    type Target;
}

#[derive(DerefMacro, DerefMutMacro)]
struct Values(Vec<String>);

impl DerefTarget for Values {
    type Target = Vec<String>;
}

#[test]
fn task_info_name_supports_string_and_static_str() {
    let string_name = StringNameTask {
        name: "string-name".to_string(),
    };
    let static_name = StaticNameTask {
        name: "static-name",
    };

    assert_eq!(string_name.name(), "string-name");
    assert_eq!(static_name.name(), "static-name");
}

#[test]
fn connection_plugin_name_supports_all_declared_forms() {
    let string_connection = StringConnectionTask {
        name: "task",
        connection_plugin_name: "ssh".to_string(),
    };
    let static_connection = StaticConnectionTask {
        name: "task",
        connection_plugin_name: "netconf",
    };
    let option_string_connection = OptionStringConnectionTask {
        name: "task",
        connection_plugin_name: Some("http".to_string()),
    };
    let option_static_connection = OptionStaticConnectionTask {
        name: "task",
        connection_plugin_name: Some("grpc"),
    };

    assert_eq!(string_connection.connection_plugin_name(), Some("ssh"));
    assert_eq!(static_connection.connection_plugin_name(), Some("netconf"));
    assert_eq!(
        option_string_connection.connection_plugin_name(),
        Some("http")
    );
    assert_eq!(
        option_static_connection.connection_plugin_name(),
        Some("grpc")
    );
}

#[test]
fn connection_plugin_name_treats_empty_values_as_absent() {
    let string_connection = StringConnectionTask {
        name: "task",
        connection_plugin_name: "   ".to_string(),
    };
    let static_connection = StaticConnectionTask {
        name: "task",
        connection_plugin_name: "",
    };
    let option_string_connection = OptionStringConnectionTask {
        name: "task",
        connection_plugin_name: Some("\t".to_string()),
    };
    let option_static_connection = OptionStaticConnectionTask {
        name: "task",
        connection_plugin_name: None,
    };

    assert_eq!(string_connection.connection_plugin_name(), None);
    assert_eq!(static_connection.connection_plugin_name(), None);
    assert_eq!(option_string_connection.connection_plugin_name(), None);
    assert_eq!(option_static_connection.connection_plugin_name(), None);
}

#[test]
fn get_connection_key_reflects_connection_plugin_presence() {
    let no_connection = StringNameTask {
        name: "no-connection".to_string(),
    };
    let with_connection = OptionStringConnectionTask {
        name: "with-connection",
        connection_plugin_name: Some("ssh".to_string()),
    };

    assert!(no_connection.get_connection_key("router1").is_none());

    let key = with_connection.get_connection_key("router1").unwrap();
    assert_eq!(key.hostname, "router1");
    assert_eq!(key.plugin_name, "ssh");
}

#[test]
fn options_returns_absent_or_configured_payload() {
    let without_options = StringNameTask {
        name: "without-options".to_string(),
    };
    let with_options = OptionsTask {
        name: "with-options",
        options: Some(json!({"changed": false, "retries": 3})),
    };

    assert!(without_options.options().is_none());
    assert_eq!(
        with_options.options(),
        Some(&json!({"changed": false, "retries": 3}))
    );
}

#[test]
fn processor_names_support_absent_dynamic_and_static_configuration() {
    let absent = StringNameTask {
        name: "absent".to_string(),
    };
    let dynamic = DynamicProcessorsTask {
        name: "dynamic",
        processor_names: Vec::new(),
    }
    .with_processor("audit")
    .with_processors(["metrics", "trace"]);
    let static_processors = StaticProcessorsTask { name: "static" };

    assert!(absent.processor_names().is_empty());
    assert_eq!(dynamic.processor_names(), vec!["audit", "metrics", "trace"]);
    assert_eq!(
        static_processors.processor_names(),
        vec!["audit", "metrics"]
    );
}

#[test]
fn sub_tasks_returns_cloned_arcs_in_declaration_order() {
    let validate_config: Arc<dyn Task> = Arc::new(LeafTask {
        name: "validate_config",
    });
    let verify_health: Arc<dyn Task> = Arc::new(LeafTask {
        name: "verify_health",
    });
    let parent = ParentTask {
        name: "parent",
        validate_config: Arc::clone(&validate_config),
        verify_health: Arc::clone(&verify_health),
    };

    let sub_tasks = parent.sub_tasks();

    assert_eq!(sub_tasks.len(), 2);
    assert_eq!(sub_tasks[0].name(), "validate_config");
    assert_eq!(sub_tasks[1].name(), "verify_health");
    assert!(Arc::ptr_eq(&sub_tasks[0], &validate_config));
    assert!(Arc::ptr_eq(&sub_tasks[1], &verify_health));
}

#[test]
fn deref_macros_read_and_mutate_wrapped_value() {
    let mut values = Values(vec!["one".to_string()]);

    assert_eq!(values.as_slice(), ["one".to_string()]);

    values.push("two".to_string());

    assert_eq!(values.as_slice(), ["one".to_string(), "two".to_string()]);
}
