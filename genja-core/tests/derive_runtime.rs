use std::sync::Arc;

use genja_core::genja_task;
use genja_core::inventory::{BaseBuilderHost, Host};
use genja_core::task::{
    BlockingTaskRuntimeContext, HostTaskResult, Task, TaskExecutionMode, TaskInfo,
    TaskRuntimeContext, TaskSuccess,
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
    assert_eq!(task.options(), Some(&json!({"changed": false})));
    assert!(task.helper());
    assert!(!task.supports_dry_run());
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
