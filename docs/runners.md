# Runners

Runners control how Genja executes tasks across the selected hosts. A runner is
selected by name in settings or with `with_runner(...)`.

The runner does not decide which hosts are selected. Inventory loading and
filtering decide the selected host set, then the runner decides how task work is
scheduled across those hosts.

To create your own runner, see the [Runner Plugins](plugins/runners.md) guide.

## Built-In Runners

Genja includes two built-in runners:

- `threaded`: executes host work concurrently with bounded async workers.
- `serial`: executes host work one host at a time.

The default runner is `threaded`.

## Configure A Runner

Configure the runner in `settings.yaml`:

```yaml
runner:
  plugin: threaded
  worker_count: 10
  max_task_depth: 10
  max_connection_attempts: 3
  task_timeout: null # optional seconds per task and host
  task_retry:
    allow: false
    max_attempts: 1
    delay_ms: 0
```

Runner settings:

- `plugin`: runner plugin name. Defaults to `threaded`.
- `options`: plugin-specific JSON object. Defaults to `{}`.
- `worker_count`: optional concurrency limit for runners that support it.
- `max_task_depth`: maximum nested task depth. Defaults to `10`.
- `max_connection_attempts`: maximum connection attempts. Defaults to `3`.
- `task_timeout`: optional task timeout in positive seconds for each host; fractional values such as `0.5` are accepted. Defaults to `null`.
- `task_retry.allow`: whether task retries are allowed by default. Defaults to `false`.
- `task_retry.max_attempts`: total number of task attempts allowed by default. Defaults to `1`.
- `task_retry.delay_ms`: base delay in milliseconds between retry attempts. Defaults to `0`.
- `task_retry.policy`: optional `no_retry`, `fixed`, or `exponential` algorithm.
- `task_retry.factor` and `task_retry.max_delay_ms`: optional exponential multiplier and per-wait cap; when omitted with exponential retry, they default to `2` and `86400000` ms.

`max_connection_attempts` is part of the shared runner configuration. The
built-in runners pass the task connection resolver into task execution; the
connection layer is responsible for interpreting connection retry behavior.

For `task_timeout`, a value declared by the task takes precedence over an
explicit `TaskRunOptions` invocation value, which overrides
`runner.task_timeout`. Each subtask resolves its own value and does not inherit
a timeout declared by its parent. Without any value, no timeout is configured.
The selected limit starts when that task begins on a host and covers connection
setup, checks, attempts, retry waits, and verification. Retry attempts share
the same deadline. An expired task reports a host-scoped timeout failure and
does not start another attempt or subtask.
In an ordered task list, each later root gets a fresh deadline on each host,
including a host where an earlier root or subtask timed out.

`task_retry.allow`, `task_retry.max_attempts`, and `task_retry.delay_ms` define default retry
policy for task execution. Tasks may override these defaults through task
metadata. Retries are only attempted when the effective policy allows them and
the task returns a failed host result marked as retryable.
Genja does not infer task mutability or idempotency when applying retries; the
decision is driven entirely by configured policy and the task's returned
failure result. Omitting `task_retry.policy` retains fixed-delay behavior;
the default `allow = false` and `max_attempts = 1` still produce one attempt.

### Task Retry Precedence

For `allow`, `max_attempts`, and `delay_ms`, each task first uses its own
declared values, then `runner.task_retry`, then the built-in defaults. A root
`TaskDefinition` retry override, including `TaskSpec.overrides.retry`,
replaces that root's authored retry metadata; any fields omitted from the
override use runner defaults. The root override does not change subtasks.
This resolution is per field, so a task can set `allow` while inheriting
`max_attempts` and `delay_ms` from the runner.

Algorithm selection has its own order. A root uses a custom Rust policy
attached to its `TaskDefinition`, otherwise an explicit selection in its
`TaskDefinition` retry override, otherwise its authored task metadata,
otherwise a Rust invocation fallback, otherwise `runner.task_retry.policy`,
otherwise fixed delay. When a root
override exists without a policy selection, the authored policy is replaced
along with the other authored retry metadata. A subtask uses its own explicit
algorithm selection if present; otherwise it inherits the parent's selected
algorithm. Each subtask still resolves `allow`, `max_attempts`, and
`delay_ms` from its own metadata and the runner defaults.

Rust callers can add a root with its own policy using
`tasks.add_task_with_retry_policy(task, policy)`.
`Genja::run_task_with_retry_policy(...)` accepts a policy
for one task tree, while `run_tasks_with_retry_policy(...)` supplies a fallback
for roots in a list. Both have async forms. The built-in serial and threaded
runners carry custom policies; other runner plugins must declare support or
Genja reports an error when a custom policy is selected.

See [Tasks](tasks.md) for Rust `#[genja_task(...)]` and Python `@task(...)`
examples showing how to set task-level retry overrides.

## Threaded Runner

The `threaded` runner executes a task across multiple hosts concurrently. It is
usually the best default for network or I/O-bound automation.

The effective worker count is chosen in this order:

1. Use `runner.worker_count` when set.
2. Otherwise use system available parallelism.
3. Never run more workers than selected hosts.
4. Clamp configured worker counts to at least `1`.

Use `threaded` when hosts can be processed independently:

```yaml
runner:
  plugin: threaded
  worker_count: 20
```

The threaded runner keeps up to `worker_count` host executions in flight. As
soon as one host finishes, the next pending host is scheduled until the selected
host list is exhausted.

Host results are merged as workers finish, so the order of host keys in a
threaded result should not be used as an execution-order guarantee. Use the
status lists and host IDs in the result data when making automation decisions.

## Serial Runner

The `serial` runner executes work sequentially. It is useful for debugging,
small inventories, strict ordering, or tasks that should not run concurrently.

```yaml
runner:
  plugin: serial
```

`worker_count` does not affect the built-in `serial` runner.

Serial execution follows the selected host iteration order. That makes logs and
result inspection easier while you are developing a task or diagnosing an
inventory issue.

## Select A Runner In Code

`with_runner(...)` returns a new runtime with the selected runner. It preserves
the existing runner options and limits while changing only the runner plugin
name.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;

    fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;
        let genja = genja.with_runner("serial")?;

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    genja = genja_lib.Genja.from_settings_file("settings.yaml")
    genja = genja.with_runner("serial")
    ```

The named runner must already be registered. Unknown plugins fail, and plugins
registered under a different plugin type are rejected.

## Task Depth

`max_task_depth` limits nested sub-task execution.

```yaml
runner:
  plugin: threaded
  max_task_depth: 1
```

When running a task directly, the call may also provide a maximum depth.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let results = genja.run_task(task, 1)?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    results = genja.run_task(
        CollectFacts,
        run_options=genja_lib.TaskRunOptions(max_depth=1),
    )
    ```

Use `0` for only the top-level task. Use a higher value when sub-tasks should
run.

Rust `run_task(...)` and `run_tasks(...)` require the depth argument. Python
uses `run_options=TaskRunOptions(...)` for runtime execution controls; when
`run_options` is omitted or `TaskRunOptions.max_depth` is `None`, the runtime
uses `runner.max_task_depth` from settings.

## Multiple Tasks

When running an ordered task list, root tasks are executed in list order. The
runner controls host execution for each root task.

- `serial`: each root task runs host-by-host before the next root task starts.
- `threaded`: each root task runs selected hosts concurrently before the next
  root task starts.

Sub-tasks belong to their parent task tree and are controlled by the same task
depth limit.

This means the built-in threaded runner parallelizes hosts within a root task;
it does not run separate root tasks from the same `Tasks` list at the same time.

## Empty Host Selections

If filtering selects no hosts, runners still return a task result object for the
requested task. The host result map is empty, and lifecycle processors can still
observe task start and finish hooks.

This is useful for workflows that treat "nothing matched" as a reportable
outcome instead of a runtime crash. If an empty host set is unexpected, check
`host_ids()` before running the task.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    if genja.host_ids().is_empty() {
        eprintln!("no hosts selected");
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    if not genja.host_ids():
        print("no hosts selected")
    ```

## Choosing A Runner

Use `threaded` when:

- hosts can be processed independently
- tasks spend time waiting on network or remote systems
- inventory is large enough that concurrency matters

Use `serial` when:

- debugging task logic
- output ordering matters
- remote systems should not be touched concurrently
- a workflow has external sequencing constraints
