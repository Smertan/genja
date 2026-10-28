# Execution policy: current behavior baseline

This note records the execution behavior that an execution-policy design must
preserve for invocations that select no new policy. It describes the current
implementation; it does not define a new public interface. The relevant paths
are `Genja::run_task*_with_options` and `Genja::run_tasks*_with_options` in
`genja/src/lib.rs`, `PluginRunner` in
`genja-plugin-manager/src/plugin_types.rs`, and `TaskDefinition::start_with_depth`
in `genja-core/src/task.rs`.

## Invocation and runner boundary

1. `Genja` selects hosts, creates a task connection resolver, attaches a
   processor resolver to each root `TaskDefinition`, and calls the selected
   runner plugin with `RunnerConfig` and `TaskRunOptions`. `TaskRunOptions`
   currently contains only maximum subtask depth and dry-run state. It is
   `Copy`.
2. A single-task call runs one root task tree. A task-list call runs roots in
   input order and returns one `TaskResults` per root in that order. The
   `PluginRunner::run_tasks` default delegates to `run_task` for each root;
   built-in runners also preserve root order.
3. The serial runner executes hosts sequentially through `TaskExecutor`. The
   threaded runner schedules bounded concurrent host jobs and merges their
   results. Both enter the same `TaskDefinition` host execution path. Aggregate
   `on_task_start` runs before a root's hosts; `on_task_finish` runs after host
   results are collected. These hooks can modify `TaskResults`.
4. Python `Genja.run_task*` and `run_tasks*` extract Python task metadata and
   delegate to the corresponding Rust `Genja` options methods. Python
   `TaskDefinition.run_on_host(s)` has a direct core execution path, without a
   runner configuration; it uses the core retry defaults.

## One task on one host

`TaskDefinition::start_with_depth` performs these steps for each task node and
host. Each recursive subtask call starts its own attempt count.

1. Check the maximum depth. An overflow records an internal failed host
   result without calling the task. Otherwise, resolve the task's effective
   retry settings and processor plugins.
2. Validate a requested fresh-session verification configuration. A validation
   failure is recorded before connection or task execution. Resolve or open the
   task connection once before the task attempt loop. An initial connection
   failure records a connection failure and does not enter that loop.
3. If dry-run was requested and the task does not support it, record an
   unsupported failure. A supported dry-run uses `dry_run` or `dry_run_async`
   inside the ordinary attempt loop. It bypasses convergence checks and does
   not request fresh-session verification.
4. For each task attempt, increment the task-local counter, run
   `on_instance_start`, and create a context carrying `current_attempt` and
   dry-run state. Async task methods are awaited directly. Blocking methods,
   including their checks, run in `spawn_blocking`. Python-backed task methods
   enter through these same Rust task modes.
5. In normal execution, `IdempotencyMode::Disabled` calls the task start
   method. `Check` and `CheckAndVerify` first call the task's convergence
   check. A converged pre-check returns success without applying the change;
   when this occurs after an earlier failed attempt, it returns
   `PassedWithWarnings` with metadata warning that an earlier attempt may have
   changed state or skipped finalization. A `ChangeRequired` pre-check calls
   the task start method. `CheckAndVerify` checks again after a passed result,
   except when its post-check is deferred until a replacement session exists.
   A remaining diff becomes a non-retryable validation failure with details
   that application completed and configuration may have changed.
6. Convert a returned `TaskError` into a failed `HostTaskResult`. Decide whether
   to retry that result. On retry, wait for the configured fixed delay and
   repeat from step 4; otherwise leave the task attempt loop.
7. After the loop, a configured fresh-session check runs only for a passed,
   changed, non-dry-run result. It replaces the connection up to its own
   configured attempt limit, with its own delay. It does not rerun the task.
   `CheckAndVerify` then runs its deferred post-check on the replacement
   connection. Failed replacement reports that the change may already have
   been applied and rollback is unavailable.
8. Record timing and task retry metadata, run `on_instance_finish` once on
   the final host result, then insert that result. Process `sub_tasks()` in
   order after recording the parent, including when the parent result failed
   or was skipped. Each subtask resolves its own processors and retry settings.

Early validation, connection, and unsupported dry-run failures also receive
timing and metadata, run `on_instance_finish`, and are recorded. They do not
enter the task attempt loop.

## Retry settings and decision baseline

`RetryConfig` has optional `allow`, `max_attempts`, and `delay_ms` fields.
The effective values are resolved field by field from the task's
`TaskInfo::retry_config()` over `RunnerConfig` retry defaults. Without a
runner, core defaults are `allow = false`, `max_attempts = 1`, and
`delay_ms = 0`. The effective maximum is at least one and includes the first
attempt. A `TaskDefinition` retry override replaces the root task's authored
`RetryConfig`; it is not applied to subtasks. `TaskSpecOverrides.retry` is
implemented through that root override. Rust `#[genja_task(..., retry(...))]`
and Python `@task(..., retry=RetryConfig(...))` supply authored task metadata;
Python runner retry settings feed the same `RunnerConfig` path.

Today the task retries only when all three conditions hold: effective
`allow` is true, the current task attempt is below `max_attempts`, and the
final attempt result is `Failed(TaskFailure)` with `retryable = true`.
Passed, `PassedWithWarnings`, skipped, and non-retryable failed results are
not retried. `retry_exhausted` is true only for a final retryable failure with
retries allowed, `max_attempts > 1`, and attempts at or above that maximum.
The retry decision is made before fresh-session verification. A failure from
replacement or its deferred post-check does not re-enter the task retry loop.

The three attempt domains are distinct:

| Domain | Current counter and scope | Repeats task application? |
| --- | --- | --- |
| Task attempts | Local to one task node on one host; exposed as `current_attempt` and final `execution_metadata.attempts` | Yes, subject to the retry decision and convergence check |
| Initial connection opening | Before the task attempt loop; managed by the connection resolver and connection manager | No |
| Fresh-session replacement | Separate `session_verification.attempts` after a passed changed task | No |

## Result and compatibility baseline

`HostTaskResult` contains `HostTaskOutcome` (`Passed`,
`PassedWithWarnings`, `Failed`, or `Skipped`) and
`HostExecutionMetadata`. The latter serializes timing, `attempts`,
`retried`, `retry_exhausted`, `dry_run`, and optional
`session_verification` metadata. `TaskFailure` carries a kind, message,
error type, `retryable`, details, warnings, and messages. Its captured Rust
error object is not serialized. Fresh-session metadata reports whether
verification was requested, its replacement attempt count, and whether a
new session was established. Raw and human-readable JSON paths expose these
fields; Python result conversion also depends on their current shape.

Current timing metadata uses wall-clock `SystemTime` and elapsed durations.
There is no invocation-level monotonic execution deadline or cancellation
status in this path. A later deadline design must distinguish a deadline
signal from confirmed termination of blocking, Python, or external work.

For invocations without a newly selected execution policy, the compatibility
baseline is the same effective retry fields, decisions, fixed waits, task
attempt counts, convergence outcomes, dry-run behavior, fresh-session
replacement behavior, processor ordering, and serialized result shape.
