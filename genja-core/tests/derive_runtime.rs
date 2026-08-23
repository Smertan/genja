use std::sync::Arc;

use genja_core::genja_task;
use genja_core::inventory::{BaseBuilderHost, Host};
use genja_core::task::{
    BlockingTaskRuntimeContext, HostTaskResult, IdempotencyCheck, IdempotencyMode, Task,
    TaskCatalog, TaskExecutionMode, TaskFactoryRegistry, TaskIdSource, TaskInfo,
    TaskRuntimeContext, TaskSuccess, compiled_task_registry, create_compiled_task_by_identity,
    create_compiled_task_from_spec_str, create_compiled_task_from_spec_str_with_format,
    get_compiled_task_descriptor, get_compiled_task_descriptor_by_identity,
};
use serde_json::{Value, json};

struct LeafTask;

#[genja_task(name = "leaf")]
impl LeafTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

struct AsyncTask {
    options: Option<Value>,
}

#[genja_task(
    name = "async_task",
    connection_plugin_name = "ssh",
    processors = ["audit", "metrics"],
    retry(
        allow = true,
        max_attempts = 3,
        delay_ms = 500
    ),
    session_verification(
        max_attempts = 2,
        delay_ms = 1000
    )
)]
impl AsyncTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn options(&self) -> Option<&Value> {
        self.options.as_ref()
    }

    fn helper(&self) -> bool {
        self.options.is_some()
    }
}

struct IdempotentTask;

#[genja_task(
    name = "idempotent_task",
    idempotency = IdempotencyMode::CheckAndVerify
)]
impl IdempotentTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    async fn check_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<IdempotencyCheck, genja_core::task::TaskError> {
        Ok(IdempotencyCheck::ChangeRequired {
            diff: Some("+configured".to_string()),
            details: Some(json!({"mode": "async"})),
        })
    }
}

struct ParentTask {
    children: Vec<Arc<dyn Task>>,
}

#[genja_task(name = "parent")]
impl ParentTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        self.children.clone()
    }
}

struct BlockingTask;

#[genja_task(name = "blocking_task")]
impl BlockingTask {
    fn start(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

struct AsyncDryRunTask;

#[genja_task(name = "async_dry_run_task", supports_dry_run = true)]
impl AsyncDryRunTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary("started"),
        ))
    }

    async fn dry_run_async(
        &self,
        _host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new()
                .with_changed(context.dry_run())
                .with_summary("planned"),
        ))
    }
}

struct BlockingDryRunTask;

#[genja_task(name = "blocking_dry_run_task", supports_dry_run = true)]
impl BlockingDryRunTask {
    fn start(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary("started"),
        ))
    }

    fn dry_run(
        &self,
        _host: &Host,
        context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new()
                .with_changed(context.dry_run())
                .with_summary("planned"),
        ))
    }
}

#[derive(serde::Deserialize)]
struct RegisteredSerdeTask {
    options: Value,
}

#[genja_task(
    name = "registered_serde_task",
    processors = ["audit"],
    registration(
        id = "acme.tests.derive_runtime.registered_serde_task",
        version = "1.2.3",
        description = "Constructs a task from JSON input"
    )
)]
impl RegisteredSerdeTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn options(&self) -> Option<&Value> {
        Some(&self.options)
    }
}

#[derive(serde::Deserialize)]
struct RegisteredDefaultVersionTask;

#[genja_task(
    name = "registered_default_version_task",
    registration(id = "acme.tests.derive_runtime.registered_default_version_task")
)]
impl RegisteredDefaultVersionTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

#[derive(Default)]
struct RegisteredDefaultFactoryTask;

#[genja_task(
    name = "registered_default_factory_task",
    registration(
        id = "acme.tests.derive_runtime.registered_default_factory_task",
        factory = "default"
    )
)]
impl RegisteredDefaultFactoryTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

struct RegisteredCustomFactoryTask {
    options: Value,
}

fn create_registered_custom_factory_task(
    input: Value,
) -> Result<RegisteredCustomFactoryTask, genja_core::task::TaskRegistrationError> {
    let encoded = input
        .get("encoded")
        .and_then(Value::as_str)
        .ok_or_else(|| genja_core::task::TaskRegistrationError::InvalidInput {
            id: "acme.tests.derive_runtime.registered_custom_factory_task".to_string(),
            version: "2.1.0".to_string(),
            message: "`encoded` is required".to_string(),
        })?;
    let decoded: String = encoded.chars().rev().collect();

    Ok(RegisteredCustomFactoryTask {
        options: json!({ "decoded": decoded }),
    })
}

#[genja_task(
    name = "registered_custom_factory_task",
    registration(
        id = "acme.tests.derive_runtime.registered_custom_factory_task",
        version = "2.1.0",
        description = "Prepares obfuscated input before constructing the task",
        factory = custom(create_registered_custom_factory_task)
    )
)]
impl RegisteredCustomFactoryTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn options(&self) -> Option<&Value> {
        Some(&self.options)
    }
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct RegisteredSchemaTask {
    acl_name: String,
    rules: Vec<RegisteredSchemaRule>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct RegisteredSchemaRule {
    action: String,
    cidr: String,
}

#[genja_task(
    name = "registered_schema_task",
    registration(
        id = "acme.tests.derive_runtime.registered_schema_task",
        version = "3.0.0",
        description = "Exposes a JSON Schema for task input",
        schema = "schemars"
    )
)]
impl RegisteredSchemaTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        let first_rule = self
            .rules
            .first()
            .map(|rule| format!("{}:{}", rule.action, rule.cidr))
            .unwrap_or_else(|| "none".to_string());
        Ok(HostTaskResult::passed(TaskSuccess::new().with_summary(
            format!("{}:{}:{}", self.acl_name, self.rules.len(), first_rule),
        )))
    }
}

#[derive(serde::Deserialize)]
struct RegisteredOverrideMetadataTask {
    options: Value,
}

#[genja_task(
    name = "registered_override_metadata_task",
    connection_plugin_name = "ssh",
    retry(allow = true, max_attempts = 3, delay_ms = 500),
    session_verification(max_attempts = 3, delay_ms = 2000),
    registration(
        id = "acme.tests.derive_runtime.registered_override_metadata_task",
        version = "1.0.0",
        description = "Provides authored runtime metadata for override tests"
    )
)]
impl RegisteredOverrideMetadataTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn options(&self) -> Option<&Value> {
        Some(&self.options)
    }
}

#[test]
fn genja_task_generates_task_info_from_metadata() {
    let task = AsyncTask {
        options: Some(json!({"changed": false})),
    };

    assert_eq!(task.name(), "async_task");
    assert_eq!(task.connection_plugin_name(), Some("ssh"));
    assert_eq!(task.processor_names(), vec!["audit", "metrics"]);
    let retry_config = task
        .retry_config()
        .expect("retry config should be generated");
    assert_eq!(retry_config.allow(), Some(true));
    assert_eq!(retry_config.max_attempts(), Some(3));
    assert_eq!(retry_config.delay_ms(), Some(500));
    let session_verification_config = task
        .session_verification_config()
        .expect("session verification config should be generated");
    assert_eq!(session_verification_config.max_attempts(), 2);
    assert_eq!(session_verification_config.delay_ms(), 1000);
    assert_eq!(task.options(), Some(&json!({"changed": false})));
    assert!(task.helper());
    assert!(!task.supports_dry_run());
    assert_eq!(task.idempotency_mode(), IdempotencyMode::Disabled);
}

#[test]
fn genja_task_submits_explicit_constructible_registration() {
    let descriptor = get_compiled_task_descriptor(
        "acme.tests.derive_runtime.registered_serde_task",
        Some("1.2.3"),
    )
    .expect("explicit descriptor should be registered");

    assert_eq!(
        descriptor.id,
        "acme.tests.derive_runtime.registered_serde_task"
    );
    assert_eq!(descriptor.id_source, TaskIdSource::Explicit);
    assert_eq!(descriptor.name, "registered_serde_task");
    assert_eq!(descriptor.version, "1.2.3");
    assert_eq!(
        descriptor.description.as_deref(),
        Some("Constructs a task from JSON input")
    );
    assert_eq!(descriptor.execution_mode, TaskExecutionMode::Async);
    assert_eq!(descriptor.processor_names, vec!["audit"]);
    assert_eq!(descriptor.input_schema, None);
    assert!(descriptor.constructible);
}

#[test]
fn genja_task_compiled_registry_creates_registered_task_from_json() {
    let registry = compiled_task_registry().expect("compiled registry should build");
    let task = registry
        .create(
            "acme.tests.derive_runtime.registered_serde_task",
            Some("1.2.3"),
            json!({ "options": { "source": "json" } }),
        )
        .expect("registered serde factory should create task");

    assert_eq!(task.name(), "registered_serde_task");
    assert_eq!(task.options(), Some(&json!({ "source": "json" })));

    let error = match registry.create(
        "acme.tests.derive_runtime.registered_serde_task",
        Some("1.2.3"),
        json!({ "unexpected": true }),
    ) {
        Ok(_) => panic!("serde factory should reject invalid input"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        genja_core::task::TaskRegistrationError::InvalidInput { .. }
    ));
}

#[test]
fn genja_task_compiled_registry_supports_identity_descriptor_lookup() {
    let descriptor = get_compiled_task_descriptor_by_identity(
        "acme.tests.derive_runtime.registered_serde_task@1.2.3",
    )
    .expect("identity descriptor lookup should succeed");

    assert_eq!(
        descriptor.id,
        "acme.tests.derive_runtime.registered_serde_task"
    );
    assert_eq!(descriptor.version, "1.2.3");
    assert!(descriptor.constructible);
}

#[test]
fn genja_task_compiled_registry_supports_identity_task_creation() {
    let task = create_compiled_task_by_identity(
        "acme.tests.derive_runtime.registered_serde_task@1.2.3",
        json!({ "options": { "source": "identity" } }),
    )
    .expect("identity create should succeed");

    assert_eq!(task.name(), "registered_serde_task");
    assert_eq!(task.options(), Some(&json!({ "source": "identity" })));
}

#[test]
fn genja_task_compiled_registry_creates_task_from_yaml_spec() {
    let task = create_compiled_task_from_spec_str(
        r#"
task: acme.tests.derive_runtime.registered_serde_task@1.2.3
input:
  options:
    source: yaml-spec
"#,
    )
    .expect("YAML task spec should construct task");

    assert_eq!(task.name(), "registered_serde_task");
    assert_eq!(task.options(), Some(&json!({ "source": "yaml-spec" })));
}

#[test]
fn genja_task_compiled_registry_creates_task_from_json_spec() {
    let task = create_compiled_task_from_spec_str_with_format(
        r#"
{
  "task": "acme.tests.derive_runtime.registered_serde_task@1.2.3",
  "input": {
    "options": {
      "source": "json-spec"
    }
  }
}
"#,
        genja_core::task::TaskSpecFormat::Json,
    )
    .expect("JSON task spec should construct task");

    assert_eq!(task.name(), "registered_serde_task");
    assert_eq!(task.options(), Some(&json!({ "source": "json-spec" })));
}

#[test]
fn genja_task_compiled_registry_applies_task_spec_runtime_overrides() {
    let task = create_compiled_task_from_spec_str(
        r#"
task: acme.tests.derive_runtime.registered_override_metadata_task@1.0.0
input:
  options:
    source: overrides
overrides:
  retry:
    allow: false
    max_attempts: 2
    delay_ms: 250
  session_verification:
    max_attempts: 2
    delay_ms: 1000
"#,
    )
    .expect("task spec with runtime overrides should construct task");

    assert_eq!(task.name(), "registered_override_metadata_task");
    assert_eq!(task.options(), Some(&json!({ "source": "overrides" })));
    let retry = task
        .retry_config()
        .expect("retry override should be present");
    assert_eq!(retry.allow(), Some(false));
    assert_eq!(retry.max_attempts(), Some(2));
    assert_eq!(retry.delay_ms(), Some(250));
    let session_verification = task
        .session_verification_config()
        .expect("session verification override should be present");
    assert_eq!(session_verification.max_attempts(), 2);
    assert_eq!(session_verification.delay_ms(), 1000);
}

#[test]
fn genja_task_compiled_registry_preserves_authored_metadata_without_spec_overrides() {
    let task = create_compiled_task_from_spec_str(
        r#"
task: acme.tests.derive_runtime.registered_override_metadata_task@1.0.0
input:
  options:
    source: authored
"#,
    )
    .expect("task spec without runtime overrides should construct task");

    let retry = task
        .retry_config()
        .expect("authored retry config should be present");
    assert_eq!(retry.allow(), Some(true));
    assert_eq!(retry.max_attempts(), Some(3));
    assert_eq!(retry.delay_ms(), Some(500));
    let session_verification = task
        .session_verification_config()
        .expect("authored session verification should be present");
    assert_eq!(session_verification.max_attempts(), 3);
    assert_eq!(session_verification.delay_ms(), 2000);
}

#[test]
fn genja_task_compiled_registry_rejects_invalid_task_spec_override() {
    let error = create_compiled_task_from_spec_str(
        r#"
task: acme.tests.derive_runtime.registered_override_metadata_task@1.0.0
input:
  options: {}
overrides:
  processors: ["audit"]
"#,
    )
    .expect_err("unsupported override should be rejected");

    assert!(matches!(
        error,
        genja_core::task::TaskSpecConstructionError::InvalidSpec(
            genja_core::task::TaskSpecError::InvalidOverride { .. }
        )
    ));
}

#[test]
fn genja_task_compiled_registry_preserves_spec_parse_errors() {
    let error = create_compiled_task_from_spec_str(
        r#"
## Starting

* bullets garbage
* points dnd
"#,
    )
    .expect_err("invalid spec text should be rejected");

    assert!(matches!(
        error,
        genja_core::task::TaskSpecConstructionError::InvalidSpec(
            genja_core::task::TaskSpecError::InvalidAuto { .. }
        )
    ));
}

#[test]
fn genja_task_compiled_registry_preserves_unknown_task_errors_from_spec() {
    let error = create_compiled_task_from_spec_str(
        r#"
task: acme.tests.derive_runtime.missing_task@1.0.0
input: {}
"#,
    )
    .expect_err("unknown task should be rejected by registry");

    assert!(matches!(
        error,
        genja_core::task::TaskSpecConstructionError::Registration(
            genja_core::task::TaskRegistrationError::NotFound { .. }
        )
    ));
}

#[test]
fn genja_task_compiled_descriptor_serializes_to_json_contract() {
    let descriptor = get_compiled_task_descriptor(
        "acme.tests.derive_runtime.registered_serde_task",
        Some("1.2.3"),
    )
    .expect("explicit descriptor should be registered");
    let serialized = serde_json::to_value(&descriptor).expect("descriptor should serialize");

    assert_eq!(
        serialized,
        json!({
            "id": "acme.tests.derive_runtime.registered_serde_task",
            "id_source": "explicit",
            "name": "registered_serde_task",
            "version": "1.2.3",
            "description": "Constructs a task from JSON input",
            "execution_mode": "async",
            "connection_plugin_name": null,
            "processor_names": ["audit"],
            "retry": null,
            "input_schema": null,
            "constructible": true
        })
    );
}

#[test]
fn genja_task_registration_version_defaults_to_package_version() {
    let descriptor = get_compiled_task_descriptor(
        "acme.tests.derive_runtime.registered_default_version_task",
        Some(env!("CARGO_PKG_VERSION")),
    )
    .expect("explicit descriptor should be registered with package version");

    assert_eq!(descriptor.id_source, TaskIdSource::Explicit);
    assert_eq!(descriptor.version, env!("CARGO_PKG_VERSION"));
    assert!(descriptor.constructible);
}

#[test]
fn genja_task_submits_input_schema_when_registration_opts_in() {
    let descriptor = get_compiled_task_descriptor(
        "acme.tests.derive_runtime.registered_schema_task",
        Some("3.0.0"),
    )
    .expect("explicit descriptor should be registered");
    let schema = descriptor
        .input_schema
        .expect("schemars input schema should be included");

    assert!(descriptor.constructible);
    assert_eq!(schema["title"], "RegisteredSchemaTask");
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["acl_name", "rules"]));
    assert_eq!(schema["properties"]["acl_name"]["type"], "string");
    assert_eq!(schema["properties"]["rules"]["type"], "array");
    assert!(
        schema["properties"]["rules"]["items"]["$ref"]
            .as_str()
            .is_some_and(|reference| reference.contains("RegisteredSchemaRule"))
    );
}

#[test]
fn genja_task_compiled_registry_creates_default_factory_task_from_empty_input() {
    let registry = compiled_task_registry().expect("compiled registry should build");
    let task = registry
        .create(
            "acme.tests.derive_runtime.registered_default_factory_task",
            Some(env!("CARGO_PKG_VERSION")),
            json!({}),
        )
        .expect("registered default factory should create task from empty object");

    assert_eq!(task.name(), "registered_default_factory_task");

    let error = match registry.create(
        "acme.tests.derive_runtime.registered_default_factory_task",
        Some(env!("CARGO_PKG_VERSION")),
        json!({ "unexpected": true }),
    ) {
        Ok(_) => panic!("default factory should reject non-empty input"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        genja_core::task::TaskRegistrationError::InvalidInput { .. }
    ));
}

#[test]
fn genja_task_compiled_registry_creates_custom_factory_task_from_prepared_input() {
    let registry = compiled_task_registry().expect("compiled registry should build");
    let task = registry
        .create(
            "acme.tests.derive_runtime.registered_custom_factory_task",
            Some("2.1.0"),
            json!({ "encoded": "terces" }),
        )
        .expect("registered custom factory should create task");

    assert_eq!(task.name(), "registered_custom_factory_task");
    assert_eq!(task.options(), Some(&json!({ "decoded": "secret" })));

    let error = match registry.create(
        "acme.tests.derive_runtime.registered_custom_factory_task",
        Some("2.1.0"),
        json!({}),
    ) {
        Ok(_) => panic!("custom factory should reject invalid input"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        genja_core::task::TaskRegistrationError::InvalidInput { .. }
    ));
}

#[test]
fn genja_task_submits_generated_discovery_descriptor() {
    let task_id = format!("auto:{}", std::any::type_name::<AsyncTask>());
    let descriptor = get_compiled_task_descriptor(&task_id, Some(env!("CARGO_PKG_VERSION")))
        .expect("generated descriptor should be registered");

    assert_eq!(descriptor.id, task_id);
    assert_eq!(descriptor.id_source, TaskIdSource::Generated);
    assert_eq!(descriptor.name, "async_task");
    assert_eq!(descriptor.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(descriptor.description, None);
    assert_eq!(descriptor.execution_mode, TaskExecutionMode::Async);
    assert_eq!(descriptor.connection_plugin_name.as_deref(), Some("ssh"));
    assert_eq!(descriptor.processor_names, vec!["audit", "metrics"]);
    assert_eq!(descriptor.input_schema, None);
    assert!(!descriptor.constructible);

    let retry_config = descriptor
        .retry
        .expect("retry metadata should be included in descriptor");
    assert_eq!(retry_config.allow(), Some(true));
    assert_eq!(retry_config.max_attempts(), Some(3));
    assert_eq!(retry_config.delay_ms(), Some(500));
}

#[test]
fn genja_task_compiled_registry_lists_generated_descriptors_deterministically() {
    let registry = compiled_task_registry().expect("compiled registry should build");
    let descriptors = registry.list().expect("compiled descriptors should list");
    let task_id = format!("auto:{}", std::any::type_name::<BlockingTask>());

    assert!(descriptors.windows(2).all(|window| {
        (&window[0].id, &window[0].version) <= (&window[1].id, &window[1].version)
    }));
    assert!(descriptors.iter().any(|descriptor| {
        descriptor.id == task_id
            && descriptor.version == env!("CARGO_PKG_VERSION")
            && descriptor.id_source == TaskIdSource::Generated
            && descriptor.execution_mode == TaskExecutionMode::Blocking
            && !descriptor.constructible
    }));
}

#[test]
fn genja_task_generates_idempotency_mode_metadata() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should build");
    let task = IdempotentTask;

    assert_eq!(task.idempotency_mode(), IdempotencyMode::CheckAndVerify);

    let host = Host::builder().hostname("router1").build();
    let context = TaskRuntimeContext::new(genja_core::task::TaskExecutionContext::new(0, 0), None);
    let check = runtime
        .block_on(task.check_async(&host, &context))
        .expect("generated async check delegate should run");
    assert_eq!(
        check,
        IdempotencyCheck::ChangeRequired {
            diff: Some("+configured".to_string()),
            details: Some(json!({"mode": "async"})),
        }
    );
}

#[test]
fn genja_task_generates_sub_tasks_delegate() {
    let child: Arc<dyn Task> = Arc::new(LeafTask);
    let task = ParentTask {
        children: vec![Arc::clone(&child)],
    };

    let sub_tasks = task.sub_tasks();
    assert_eq!(sub_tasks.len(), 1);
    assert_eq!(sub_tasks[0].name(), "leaf");
    assert!(Arc::ptr_eq(&sub_tasks[0], &child));
}

#[test]
fn genja_task_sets_execution_mode_from_method_shape() {
    assert_eq!(
        AsyncTask { options: None }.execution_mode(),
        TaskExecutionMode::Async
    );
    assert_eq!(BlockingTask.execution_mode(), TaskExecutionMode::Blocking);
}

#[test]
fn genja_task_generates_dry_run_metadata_and_delegates() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("test runtime should build");
    let host = Host::builder().hostname("router1").build();
    let async_context =
        TaskRuntimeContext::new(genja_core::task::TaskExecutionContext::new(0, 1), None)
            .with_dry_run(true);

    assert!(AsyncDryRunTask.supports_dry_run());
    let async_result = runtime
        .block_on(AsyncDryRunTask.dry_run_async(&host, &async_context))
        .expect("async dry-run should delegate");
    assert_eq!(
        async_result.success().and_then(|success| success.summary()),
        Some("planned")
    );
    assert!(
        async_result
            .success()
            .is_some_and(|success| success.changed())
    );

    let blocking_context = runtime.block_on(async {
        BlockingTaskRuntimeContext::new(
            genja_core::task::TaskExecutionContext::new(0, 1),
            None,
            tokio::runtime::Handle::current(),
        )
        .with_dry_run(true)
    });

    assert!(BlockingDryRunTask.supports_dry_run());
    let blocking_result = BlockingDryRunTask
        .dry_run(&host, &blocking_context)
        .expect("blocking dry-run should delegate");
    assert_eq!(
        blocking_result
            .success()
            .and_then(|success| success.summary()),
        Some("planned")
    );
    assert!(
        blocking_result
            .success()
            .is_some_and(|success| success.changed())
    );
}
