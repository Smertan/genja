# Tasks

Tasks are the unit of work Genja executes against selected hosts. A task
receives task metadata, the current host, and runtime context, then returns a
structured result for that host.

## Define A Task

=== ":fontawesome-brands-rust: Rust"

    Rust tasks use `#[genja_task(...)]` to define metadata and execution in one
    place.

    ```rust
    use genja::genja_core::inventory::Host;
    use genja::genja_core::task::{
        HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
    };
    use genja::genja_task;
    use serde_json::json;

    struct CollectFacts;

    #[genja_task(name = "collect_facts")]
    impl CollectFacts {
        async fn start_async(
            &self,
            host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
                json!({
                    "hostname": host.hostname(),
                    "platform": host.platform(),
                    "facts_collected": true,
                }),
            )))
        }
    }
    ```

=== ":fontawesome-brands-python: Python"

    Python tasks use the `@task(...)` decorator and define exactly one of
    `start(...)` or `start_async(...)`.

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(name="collect_facts")
    class CollectFacts:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(
                summary=f"collected facts from {host.hostname}",
                metadata={
                    "hostname": host.hostname,
                    "platform": host.platform,
                    "facts_collected": True,
                },
            )
    ```

    ### Async Variant

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(name="collect_facts_async")
    class CollectFactsAsync:
        async def start_async(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            connection = context.connection()
            show_version = None
            if connection is not None:
                show_version = await connection.execute_command("show version")

            return TaskSuccessResult(
                summary=f"collected facts from {host.hostname}",
                metadata={
                    "hostname": host.hostname,
                    "platform": host.platform,
                    "show_version": show_version,
                },
            )
    ```

## Task Inputs

Each task receives:

- task metadata: task name, options, processors, connection plugin, and sub-task
  metadata
- host data: hostname, port, username, password, platform, and inventory `data`
- runtime context: a `TaskRuntimeContext` value created by the runtime for the
  current task execution

## Sync And Async Execution

=== ":fontawesome-brands-rust: Rust"

    Rust task execution has both sync and async entrypoints:

    - `run_task(...)` / `run_tasks(...)` for synchronous callers
    - `run_task_async(...)` / `run_tasks_async(...)` for callers already inside
      a Tokio runtime

    The sync wrappers return an error when called from an active Tokio runtime.
    In that case, use the async variants instead.

=== ":fontawesome-brands-python: Python"

    Python also exposes both sync and async entrypoints:

    - `run_task(...)` / `run_tasks(...)` for synchronous callers
    - `run_task_async(...)` / `run_tasks_async(...)` for async Python callers

    Use the async entrypoints when composing Genja with `asyncio.gather(...)`,
    `asyncio.wait_for(...)`, or other coroutine-based application code.

## Runtime Context

`TaskRuntimeContext` is passed into every task entrypoint call by the runtime.
It describes the current execution state for that specific host run.

The public Python task-facing context surface is intentionally narrow. Use it
when the task needs access to the resolved host connection.

In Python, `TaskRuntimeContext` exposes:

- `connection()`: returns the resolved connection object or `None`
- `has_connection()`: returns `True` when a connection was resolved

Depth bookkeeping stays internal to the runtime.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    async fn start(
        &self,
        host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        println!("host={:?}", host.hostname());
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult


    def start(
        self,
        task: TaskInfo,
        host: Host,
        _context: TaskRuntimeContext,
    ) -> TaskSuccessResult:
        print(f"task={task.name} host={host.hostname}")
        return TaskSuccessResult()
    ```

## Result Types

Tasks return one result per host.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::task::{
        HostTaskResult, TaskFailure, TaskFailureKind, TaskSkip, TaskSuccess,
    };

    let passed = HostTaskResult::passed(TaskSuccess::new().with_summary("ok"));

    let failed = HostTaskResult::failed(
        TaskFailure::new(std::io::Error::other("connection failed"))
            .with_kind(TaskFailureKind::Connection),
    );

    let skipped = HostTaskResult::Skipped(
        TaskSkip::new()
            .with_reason("unsupported_platform")
            .with_message("host platform is not supported"),
    );
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import (
        Host,
        TaskInfo,
        TaskFailureKind,
        TaskFailureResult,
        TaskRuntimeContext,
        TaskSkipResult,
        TaskSuccessResult,
    )

    passed = TaskSuccessResult(summary="ok")

    failed = TaskFailureResult(
        message="connection failed",
        kind=TaskFailureKind.CONNECTION,
        retryable=True,
    )

    skipped = TaskSkipResult(
        reason="unsupported_platform",
        message="host platform is not supported",
    )
    ```

Success results can include result payloads, change status, diffs, summaries,
warnings, messages, and metadata. Failure results include a message, failure
kind, retryability, details, warnings, and messages. Skip results include a
machine-readable reason and human-readable message.

## Failure Kinds

Use failure kinds to make task failures easier to classify:

- `connection`
- `authentication`
- `validation`
- `timeout`
- `command`
- `unsupported`
- `internal`
- `external`

## Run A Task

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;

    fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;
        let results = genja.run_task(CollectFacts { name: "collect_facts" }, 1)?;

        let output = results.to_pretty_json_string().map_err(|err| {
            genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
        })?;
        println!("{output}");

        Ok(())
    }
    ```

    ### Async Variant

    ```rust
    use genja::Genja;

    #[tokio::main]
    async fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;
        let results = genja
            .run_task_async(CollectFacts { name: "collect_facts" }, 1)
            .await?;

        let output = results.to_pretty_json_string().map_err(|err| {
            genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
        })?;
        println!("{output}");

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    genja = genja_lib.Genja.from_settings_file("settings.yaml")
    results = genja.run_task(CollectFacts, max_depth=1)

    print(results.to_json(pretty=True))
    ```

The maximum depth controls nested sub-task execution. Use `0` when only the
top-level task should run, and a higher value when sub-tasks are expected.

## Processors

Processors are lifecycle hooks selected by task metadata. They are documented
in detail in [Processors](processors.md).

Use processors when task results need audit metadata, lightweight tracing, or
centralized result decoration. Tasks opt into them by plugin name:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_task;

    struct BackupConfig;

    #[genja_task(name = "backup_config", processors = ["audit"])]
    impl BackupConfig {
        async fn start_async(
            &self,
            _host: &genja::genja_core::inventory::Host,
            _context: &genja::genja_core::task::TaskRuntimeContext,
        ) -> Result<genja::genja_core::task::HostTaskResult, genja::genja_core::task::TaskError> {
            Ok(genja::genja_core::task::HostTaskResult::passed(
                genja::genja_core::task::TaskSuccess::new(),
            ))
        }
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import task


    @task(name="backup_config", processors=["audit"])
    class BackupConfig:
        ...
    ```

## Sub-Tasks

Sub-tasks let a task define child work that runs after the parent task. Use
sub-tasks for execution trees such as deploy, validate, and collect logs.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use std::sync::Arc;
    use genja::genja_core::task::Task;
    use genja::genja_task;

    struct ValidateConfig;

    #[genja_task(name = "validate_config")]
    impl ValidateConfig {
        async fn start_async(
            &self,
            _host: &genja::genja_core::inventory::Host,
            _context: &genja::genja_core::task::TaskRuntimeContext,
        ) -> Result<genja::genja_core::task::HostTaskResult, genja::genja_core::task::TaskError> {
            Ok(genja::genja_core::task::HostTaskResult::passed(
                genja::genja_core::task::TaskSuccess::new(),
            ))
        }
    }

    struct DeployConfig;

    #[genja_task(name = "deploy_config")]
    impl DeployConfig {
        async fn start_async(
            &self,
            _host: &genja::genja_core::inventory::Host,
            _context: &genja::genja_core::task::TaskRuntimeContext,
        ) -> Result<genja::genja_core::task::HostTaskResult, genja::genja_core::task::TaskError> {
            Ok(genja::genja_core::task::HostTaskResult::passed(
                genja::genja_core::task::TaskSuccess::new(),
            ))
        }

        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            vec![Arc::new(ValidateConfig)]
        }
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(name="validate_config")
    class ValidateConfig:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(summary=f"validated {host.hostname}")


    @task(name="deploy_config", sub_tasks=[ValidateConfig])
    class DeployConfig:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(summary=f"deployed {host.hostname}")
    ```

## Task Options

Task options are JSON-serializable metadata passed into task execution.

=== ":fontawesome-brands-rust: Rust"

    Rust tasks expose dynamic options by defining an `options()` helper method
    inside the `#[genja_task(...)]` impl.

    ```rust
    use genja::genja_core::inventory::Host;
    use genja::genja_core::task::{
        HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
    };
    use genja::genja_task;
    use serde_json::json;

    struct BackupConfig {
        options: serde_json::Value,
    }

    #[genja_task(name = "backup_config")]
    impl BackupConfig {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            let backup_path = self
                .options()
                .and_then(|options| options.get("backup_path"))
                .and_then(|value| value.as_str())
                .unwrap_or("/tmp/configs");

            Ok(HostTaskResult::passed(
                TaskSuccess::new()
                    .with_summary(format!("backup path is {backup_path}")),
            ))
        }

        fn options(&self) -> Option<&serde_json::Value> {
            Some(&self.options)
        }
    }

    let task = BackupConfig {
        options: json!({"backup_path": "/tmp/configs", "compress": true}),
    };
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(
        name="backup_config",
        options={"backup_path": "/tmp/configs", "compress": True},
    )
    class BackupConfig:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(
                summary=f"backup path is {task.options['backup_path']}",
            )
    ```

## Repository Examples

The repository includes task examples for both languages:

=== ":fontawesome-brands-rust: Rust"

    ```bash
    cargo run -p genja --example run_task
    cargo run -p genja --example run_task_tree
    ```

=== ":fontawesome-brands-python: Python"

    ```bash
    python genja/examples/python/run_task.py
    python genja/examples/python/run_task_tree.py
    ```
