use std::sync::Arc;

use genja_core::genja_task;
use genja_core::inventory::{BaseBuilderHost, Host};
use genja_core::task::{
    BlockingTaskRuntimeContext, HostTaskResult, IdempotencyCheck, IdempotencyMode, Task,
    TaskCatalog, TaskExecutionMode, TaskFactoryRegistry, TaskIdSource, TaskInfo,
    TaskRuntimeContext, TaskSuccess, compiled_task_registry, get_compiled_task_descriptor,
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
