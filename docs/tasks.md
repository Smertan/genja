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

## Rust Task Macro

Rust tasks are usually defined with `#[genja_task(...)]`. The macro is
re-exported from the `genja` crate, so application code should normally import
it from there:

```rust
use genja::genja_task;
```

The macro applies to an inherent `impl` block. It generates the task metadata
and the `Task` implementation from the methods and attributes on that block.

```rust
use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
};
use genja::genja_task;

struct BackupConfig;

#[genja_task(
    name = "backup_config",
    connection_plugin_name = "ssh",
    processors = ["audit"]
)]
impl BackupConfig {
    async fn start_async(
        &self,
        host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary(format!(
                "backed up {}",
                host.hostname().unwrap_or(host.id())
            )),
        ))
    }
}
```

The common macro options are:

- `name`: task name shown in results and task trees
- `connection_plugin_name`: connection plugin to open before task execution
- `processors`: processor plugin names to run around task execution
- `retry.allow`: optional task-level override for whether retries are allowed
- `retry.max_attempts`: optional task-level override for total task attempts
- `retry.delay_ms`: optional fixed in-process delay in milliseconds between retry attempts

Define exactly one task entrypoint in the macro `impl` block:

- `async fn start_async(...)` for async task bodies
- `fn start(...)` for blocking task bodies

The macro can also read optional helper methods from the same `impl` block, such
as `options(...)` and `sub_tasks(...)`. Use those helpers when a task needs
dynamic JSON options or child tasks in a task tree.

### Rust Retry Overrides

Use task metadata when a Rust task should opt into retries or override the
runner defaults for retry count:

```rust
use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, TaskError, TaskFailure, TaskFailureKind, TaskRuntimeContext,
};
use genja::genja_task;

struct RetryableBackup;

#[genja_task(
    name = "retryable_backup",
    connection_plugin_name = "ssh",
    retry(
        allow = true,
        max_attempts = 3,
        delay_ms = 500
    )
)]
impl RetryableBackup {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::failed(
            TaskFailure::new(std::io::Error::other("temporary rate limit"))
                .with_kind(TaskFailureKind::External)
                .with_retryable(true),
        ))
    }
}
```

These values override runner defaults for that task only. Retries still happen
only when the returned failure is explicitly marked `retryable`.

## Python Task Decorator

Python tasks use the `@task(...)` decorator for static task metadata.

Common decorator options:

- `name`: task name shown in results and task trees
- `connection_plugin_name`: connection plugin to open before task execution
- `processors`: processor plugin names to run around task execution
- `sub_tasks`: child tasks to execute beneath the current task
- `retry`: optional grouped task-level retry overrides

### Python Retry Overrides

Use `RetryConfig` when a Python task should opt into retries or override the
runner defaults for retry count or fixed retry delay:

```python
from genja.task import (
    Host,
    RetryConfig,
    TaskFailureResult,
    TaskInfo,
    TaskRuntimeContext,
    task,
)


@task(
    name="retryable_backup",
    connection_plugin_name="ssh",
    retry=RetryConfig(
        allow=True,
        max_attempts=3,
        delay_ms=500,
    ),
)
class RetryableBackup:
    def start(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskFailureResult:
        return TaskFailureResult(
            message=f"temporary rate limit on {host.hostname}",
            kind="external",
            retryable=True,
        )
```

These values override runner defaults for that task only, field by field.
Retries still happen only when the returned failure result is explicitly marked
`retryable`. `delay_ms` is a fixed local delay before retry attempts.

## Task Inputs

Each task receives:

- task metadata: task name, options, processors, connection plugin, and sub-task
  metadata
- host data: hostname, port, username, password, platform, and inventory `data`
- runtime context: a `TaskRuntimeContext` value created by the runtime for the
  current task execution

Use task metadata for values that belong to the task definition. Use host data
for values that vary by inventory target. Use runtime context for values the
runner resolves while executing, such as the current retry attempt or an
optional task connection.

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
when the task needs access to the current retry attempt or resolved host
connection.

In Python, `TaskRuntimeContext` exposes:

- `current_attempt`: the 1-based attempt currently running
- `connection()`: returns the resolved connection object or `None`
- `has_connection()`: returns `True` when a connection was resolved

Depth bookkeeping stays internal to the runtime. The first execution has
`current_attempt == 1`, the first retry has `current_attempt == 2`, and so on.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    async fn start(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        println!(
            "host={:?} attempt={}",
            host.hostname(),
            context.current_attempt()
        );
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
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult:
        print(
            f"task={task.name} "
            f"host={host.hostname} "
            f"attempt={context.current_attempt}"
        )
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

    let skipped = HostTaskResult::skipped_with_detail(
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
machine-readable reason and human-readable message. Per-host timing and retry
data are reported on `HostTaskResult.execution_metadata`, not inside success or
failure payloads.

For Rust consumers, a good pattern is:

```rust
if let Some(failure) = host_result.failure() {
    println!("failed: {}", failure.message());
}

if let Some(duration) = host_result.execution_metadata().duration_display() {
    println!("duration: {duration}");
}
```

Prefer returning an explicit failure or skip result when the task can classify
the outcome. Reserve raised errors for unexpected internal errors that should be
treated as task execution failures by the runtime.

When task retries are enabled by runner settings or task metadata, Genja only
retries failures explicitly marked as retryable. Return a failed host result
with `retryable=true` when the failure is transient and safe to retry.
Genja does not infer whether a task is mutable, safe to repeat, or idempotent;
retry behavior is always controlled by explicit policy plus the returned
failure classification.

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

## Error Behavior

A task can finish a host in three normal states:

- passed: the task completed successfully
- failed: the task ran and determined that the host failed
- skipped: the task intentionally did not apply to that host

Rust task entrypoints return `Result<HostTaskResult, TaskError>`. Returning
`Ok(HostTaskResult::failed(...))` records a classified host failure. Returning
`Err(TaskError)` reports an execution error from the task implementation.

Python task entrypoints return `TaskSuccessResult`, `TaskFailureResult`, or
`TaskSkipResult`. If the method raises an exception, the runtime records it as
a failed task execution for that host.

The `core.raise_on_error` setting is not currently used as a task execution
policy. Task failures and task entrypoint errors are recorded in the result tree
regardless of that setting.

Sub-tasks run after their parent task for the same host. The runner enforces
the configured maximum depth, so a task tree can be defined once and run with
different depth limits depending on the workflow.

## Run A Task

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;

    fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;
        let results = genja.run_task(CollectFacts, 1)?;

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
            .run_task_async(CollectFacts, 1)
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
    results = genja.run_task(
        CollectFacts,
        run_options=genja_lib.TaskRunOptions(max_depth=1),
    )

    print(results.to_json(pretty=True))
    ```

The maximum depth controls nested sub-task execution. Use `0` when only the
top-level task should run, and a higher value when sub-tasks are expected.

## Dry-Run Execution

Dry-run is an execution mode requested by the runtime. Task authors opt in by
declaring support and implementing the matching dry-run entrypoint.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja::genja_core::inventory::Host;
    use genja::genja_core::task::{
        HostTaskResult, TaskError, TaskRunOptions, TaskRuntimeContext, TaskSuccess,
    };
    use genja::genja_task;

    struct ConfigureInterface;

    #[genja_task(
        name = "configure_interface",
        connection_plugin_name = "ssh",
        supports_dry_run = true,
    )]
    impl ConfigureInterface {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new().with_changed(true)))
        }

        async fn dry_run_async(
            &self,
            host: &Host,
            context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            assert!(context.dry_run());

            Ok(HostTaskResult::passed(
                TaskSuccess::new()
                    .with_changed(true)
                    .with_diff("- shutdown\n+ no shutdown")
                    .with_summary(format!("would update {}", host.hostname().unwrap_or("host"))),
            ))
        }
    }

    let genja = Genja::from_settings_file("settings.yaml")?;
    let results = genja.run_task_with_options(
        ConfigureInterface,
        TaskRunOptions::new(1).with_dry_run(true),
    )?;
    # Ok::<(), genja::GenjaError>(())
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(
        name="configure_interface",
        connection_plugin_name="ssh",
        supports_dry_run=True,
    )
    class ConfigureInterface:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(changed=True)

        def dry_run(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            assert context.dry_run

            return TaskSuccessResult(
                changed=True,
                diff="- shutdown\n+ no shutdown",
                summary=f"would update {host.hostname}",
            )


    genja = genja_lib.Genja.from_settings_file("settings.yaml")
    results = genja.run_task(
        ConfigureInterface,
        run_options=genja_lib.TaskRunOptions(max_depth=1, dry_run=True),
    )
    ```

During dry-run, Genja resolves the task normally and opens any declared task
connection before calling `dry_run(...)` or `dry_run_async(...)`. This validates
inventory, settings, plugin lookup, credentials, and connection establishment,
but opening a connection can still create external side effects such as login
audit records, sessions, locks, or rate-limit usage.

Dry-run entrypoints return the same result types as normal execution. Use
`changed=True` when normal execution is expected to change managed state, use
`changed=False` when the target is already in the desired state, and use `diff`,
`summary`, and `metadata` to describe the planned change. Serialized host
execution metadata includes `dry_run`, so consumers can distinguish a planned
change from an applied change.

If dry-run is requested for a task that does not declare support, Genja records a
clear host failure before calling `start(...)` or `start_async(...)`. Declaring
dry-run support without the matching dry-run method fails during macro expansion
or Python task decoration.

## Inspect Results

`run_task(...)` returns a task result tree. Check the host summary before
assuming the task succeeded for every host.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let summary = results.task_summary();
    let hosts = summary.hosts();

    println!(
        "passed={} failed={} skipped={}",
        hosts.passed(),
        hosts.failed(),
        hosts.skipped(),
    );

    for host_id in results.failed_hosts() {
        println!("failed host: {host_id}");
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    print(results.host_summary())

    for host_id in results.failed_hosts:
        print(f"failed host: {host_id}")
    ```

Use `task_summary()` when you need recursive counts for a task tree with
sub-tasks.

Full results have two useful output shapes:

- normalized output: default, stable field names for reports and scripts
- raw output: internal enum-shaped data for debugging and bridge/plugin code

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let json = results.to_pretty_json_string().map_err(|err| {
        genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
    })?;
    println!("{json}");

    let raw_json = results.to_raw_pretty_json_string().map_err(|err| {
        genja::GenjaError::Message(format!("failed to serialize raw task results: {err}"))
    })?;
    println!("{raw_json}");
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    print(results.to_json(pretty=True))
    data = results.to_dict()

    print(results.to_json(raw=True, pretty=True))
    raw = results.to_dict(raw=True)
    ```

Normalized output stores each host result with fields such as `status`,
`summary`, `metadata`, and `messages`:

```json
{
  "task_name": "collect_facts",
  "hosts": {
    "router1": {
      "status": "passed",
      "summary": "collected facts from 10.0.0.1",
      "metadata": {
        "platform": "ios"
      }
    }
  },
  "sub_tasks": {}
}
```

Raw output preserves the underlying variant names, such as `Passed`, `Failed`,
and `Skipped`:

```json
{
  "task_name": "collect_facts",
  "hosts": {
    "router1": {
      "Passed": {
        "summary": "collected facts from 10.0.0.1",
        "metadata": {
          "platform": "ios"
        }
      }
    }
  },
  "sub_tasks": {}
}
```

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
They are task-authored metadata and are separate from runtime
`TaskRunOptions`, which operators use for controls such as `max_depth` and
`dry_run`.

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
