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
Most task authors work with the task struct and `#[genja_task]` implementation
directly. `TaskDefinition` is Genja's runtime wrapper around a task instance; it
is mainly used by registry, task-list, and lower-level runtime APIs rather than
ordinary task authoring code.

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
- `session_verification.max_attempts`: optional post-change new-session attempt count
- `session_verification.delay_ms`: optional fixed delay in milliseconds between new-session attempts
- `supports_dry_run`: opt into runtime dry-run dispatch
- `idempotency`: opt into task-authored convergence checks with
  `IdempotencyMode::Check` or `IdempotencyMode::CheckAndVerify`

**Grouped metadata** is written as nested macro arguments, not dotted keys:

```rust
#[genja_task(
    name = "backup_config",
    connection_plugin_name = "ssh",
    retry(
        allow = true,
        max_attempts = 3,
        delay_ms = 500
    ),
    session_verification(
        max_attempts = 3,
        delay_ms = 5000
    )
)]
impl BackupConfig {
    // task methods
}
```

Define exactly one task entrypoint in the macro `impl` block:

- `async fn start_async(...)` for async task bodies
- `fn start(...)` for blocking task bodies

The macro can also read optional helper methods from the same `impl` block, such
as `options(...)` and `sub_tasks(...)`. Use those helpers when a task needs
dynamic JSON options or child tasks in a task tree.

For stable discovery and JSON construction by task ID, see
[Rust Task Registration](task-registration.md).

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
- `session_verification`: optional post-change new-session verification metadata
- `supports_dry_run`: opt into runtime dry-run dispatch
- `idempotency`: opt into task-authored convergence checks with
  `IdempotencyMode.CHECK` or `IdempotencyMode.CHECK_AND_VERIFY`

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

## Session Verification

Session verification is task-authored metadata for changes that can affect
management access, such as ACLs, authentication, routing, interfaces, and
control-plane configuration. It proves that Genja can close the existing
management connection and establish a genuinely new authenticated session after
the task applies a change.

Session verification is separate from retry and idempotency:

- retry controls whether a failed task application may run again
- idempotency checks whether desired managed state is present
- session verification checks whether a new management session can connect

Session verification runs only when all of these are true:

- the task declares session verification
- execution is not dry-run
- the task entrypoint returns a passed result
- the passed result has `changed=true`

It does not run after failed results, skipped results, unchanged passed results,
or dry-run execution. Session establishment attempts are not task application
retries; Genja does not call `start(...)` or `start_async(...)` again when a
replacement session attempt fails.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    #[genja_task(
        name = "replace_management_acl",
        connection_plugin_name = "ssh",
        session_verification(
            max_attempts = 3,
            delay_ms = 5000
        )
    )]
    impl ReplaceManagementAcl {
        // task methods
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import SessionVerificationConfig, task


    @task(
        name="replace_management_acl",
        connection_plugin_name="ssh",
        session_verification=SessionVerificationConfig(
            max_attempts=3,
            delay_ms=5000,
        ),
    )
    class ReplaceManagementAcl:
        # task methods
        ...
    ```

`max_attempts` defaults to `1` and must be greater than `0`. `delay_ms`
defaults to `0` and must be greater than or equal to `0`. Session verification
requires `connection_plugin_name` because there is no management connection to
replace without a declared connection plugin.

When session verification succeeds, Genja preserves the original passed task
result and records session-verification execution metadata. When a new session
cannot be established, Genja records a host-scoped connection failure. The
failure states that the change may already have been applied and that automatic
rollback is unavailable. Other hosts continue according to normal runner
behavior.

When `IdempotencyMode::CheckAndVerify` or
`IdempotencyMode.CHECK_AND_VERIFY` is also enabled, Genja replaces the
connection before running the post-application idempotency check. That post
check therefore runs through the replacement session. With idempotency disabled,
successful replacement session establishment is the verification.

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
warnings, messages, and metadata. Warning-bearing successes may be represented
as `PassedWithWarnings` when the desired state is satisfied but important
non-fatal warnings should be visible in the top-level outcome; these still count
as passed hosts in summaries and `passed_hosts()`. Failure results include a
message, failure kind, retryability, details, warnings, and messages. Skip
results include a machine-readable reason and human-readable message. Per-host
timing and retry data are reported on `HostTaskResult.execution_metadata`, not
inside success or failure payloads.

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

## Idempotency Checks

Idempotency is declared by the task author. When enabled, Genja checks the
current host state before calling the normal task entrypoint. If the host is
already converged, Genja records a passed result with `changed=false` and does
not call `start(...)` or `start_async(...)`.

Modes:

- `Disabled`: default behavior; no convergence check runs
- `Check`: run one pre-execution check
- `CheckAndVerify`: run the pre-execution check, apply when needed, then run the
  same check again after a passed application result

The check hook must match the task execution mode:

- blocking tasks implement `check(...)`
- async tasks implement `check_async(...)`

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja::genja_core::inventory::Host;
    use genja::genja_core::task::{
        HostTaskResult, IdempotencyCheck, IdempotencyMode, TaskError,
        TaskRuntimeContext, TaskSuccess,
    };
    use genja::genja_task;

    struct EnsureNtp;

    #[genja_task(
        name = "ensure_ntp",
        connection_plugin_name = "ssh",
        idempotency = IdempotencyMode::CheckAndVerify,
    )]
    impl EnsureNtp {
        async fn check_async(
            &self,
            _host: &Host,
            context: &TaskRuntimeContext,
        ) -> Result<IdempotencyCheck, TaskError> {
            let connection = context.connection().expect("ssh connection is configured");
            let mut connection = connection.lock().await;
            let running = connection
                .execute_command("show running-config | include ^ntp server")
                .await?;

            let desired = "ntp server 192.0.2.10";
            if running.lines().any(|line| line.trim() == desired) {
                return Ok(IdempotencyCheck::converged(format!(
                    "{desired} is already configured"
                )));
            }

            Ok(IdempotencyCheck::change_required(format!("+{desired}"))
                .with_details(serde_json::json!({
                    "current": running,
                    "desired": desired,
                })))
        }

        async fn start_async(
            &self,
            _host: &Host,
            context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            let desired = "ntp server 192.0.2.10";
            let connection = context.connection().expect("ssh connection is configured");
            let mut connection = connection.lock().await;
            connection
                .execute_command(format!("configure terminal\n{desired}\nend"))
                .await?;

            Ok(HostTaskResult::passed(
                TaskSuccess::new()
                    .with_changed(true)
                    .with_diff(format!("+{desired}"))
                    .with_summary("Configured NTP server"),
            ))
        }
    }

    let genja = Genja::from_settings_file("settings.yaml")?;
    let results = genja.run_task(EnsureNtp, 1)?;
    # Ok::<(), genja::GenjaError>(())
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import asyncio

    import genja as genja_lib
    from genja.task import (
        Host,
        IdempotencyCheckResult,
        IdempotencyMode,
        TaskInfo,
        TaskRuntimeContext,
        TaskSuccessResult,
        task,
    )


    @task(
        name="ensure_ntp",
        connection_plugin_name="ssh",
        idempotency=IdempotencyMode.CHECK_AND_VERIFY,
        options={"server": "192.0.2.10"},
    )
    class EnsureNtp:
        async def check_async(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> IdempotencyCheckResult:
            connection = context.connection()
            running = await connection.execute_command(
                "show running-config | include ^ntp server"
            )

            desired = f"ntp server {task.options['server']}"
            if desired in {line.strip() for line in running.splitlines()}:
                return IdempotencyCheckResult.converged(
                    summary=f"{desired} is already configured",
                )

            return IdempotencyCheckResult.change_required(
                diff=f"+{desired}",
                details={
                    "current": running,
                    "desired": desired,
                },
            )

        async def start_async(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            connection = context.connection()
            desired = f"ntp server {task.options['server']}"
            await connection.execute_command(f"configure terminal\n{desired}\nend")

            return TaskSuccessResult(
                changed=True,
                diff=f"+{desired}",
                summary="Configured NTP server",
            )


    async def main() -> None:
        genja = genja_lib.Genja.from_settings_file("settings.yaml")
        results = await genja.run_task_async(
            EnsureNtp,
            run_options=genja_lib.TaskRunOptions(max_depth=1),
        )

        print(results.to_json(pretty=True))


    asyncio.run(main())
    ```

`CheckAndVerify` reuses the same check hook for post-application verification.
If the second check still reports `ChangeRequired`, Genja records a validation
failure. Verification only runs after the normal task entrypoint returns a
passed result.

If session verification is also enabled, Genja replaces the task connection
before running the `CheckAndVerify` post-check. The post-check therefore proves
state convergence through the newly established management session.

Idempotency checks should be read-only. They may open declared connections,
run inspection commands, normalize current state, calculate diffs, and return
diagnostic details. They should not apply configuration, save configuration,
create or delete resources, or call mutating task entrypoints.

Dry-run remains separate from idempotency. If dry-run is requested, Genja calls
the task dry-run hook and does not automatically call the idempotency check.
Task authors may call shared private inspection code from both hooks, but
dependent sub-tasks should account for parent dry-run behavior explicitly.

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

Dry-run dispatch does not automatically invoke idempotency checks. Idempotency
checks inspect current state, while dry-run does not mutate state; automatically
checking child tasks can be misleading when a parent dry-run would have created
the prerequisite state. Task authors who want dry-run to reuse idempotency logic
can call their check hook from their dry-run hook, but dependent sub-tasks should
account for parent dry-run behavior explicitly.

Dry-run also does not run session verification. Session verification proves
post-change access after an applied change, and dry-run reports planned behavior
without applying that change.

If dry-run is requested for a task that does not declare support, Genja records a
clear host failure before calling `start(...)` or `start_async(...)`. Declaring
dry-run support without the matching dry-run method fails during macro expansion
or Python task decoration.

## Task Lifecycle Composition

Retry, idempotency, dry-run, and session verification are independent controls
that answer different questions during task execution:

| Feature | Question it answers |
| --- | --- |
| Retry | Should a retryable failed application attempt run again? |
| Idempotency | Is the desired managed state already present? |
| Dry-run | What would the task do without applying a change? |
| Session verification | Can a new management session connect after an applied change? |

For normal execution, Genja composes these controls in a fixed order:

1. Run the idempotency pre-check, if enabled.
2. Skip the task entrypoint if the pre-check reports convergence.
3. Run the task entrypoint.
4. Retry only when retry policy allows it and the task result is retryable.
5. If session verification is enabled and the application result is passed with
   `changed=true`, replace the task connection.
6. If `CheckAndVerify` is enabled, run the post-check. When session verification
   is also enabled, this post-check runs through the replacement session.

Dry-run uses a different lifecycle:

1. Resolve and open any declared task connection.
2. Run only `dry_run(...)` or `dry_run_async(...)`.
3. Do not run idempotency checks.
4. Do not run session verification.
5. Return the planned result without applying a change.

The controls do not imply each other. Retry does not prove convergence or
new-session access. Idempotency does not prove that a new management session can
connect. Session verification does not rerun task application. Dry-run reports
planned behavior and does not trigger post-change checks.

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

Raw output preserves the underlying variant names, such as `Passed`,
`PassedWithWarnings`, `Failed`, and `Skipped`:

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
