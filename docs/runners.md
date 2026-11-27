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
An explicit `task_timeout` on a blocking task, declared by the task or supplied
through `TaskRunOptions`, is rejected before connection setup or execution.
The `runner.task_timeout` default is ignored for blocking tasks, so one runner
can execute both async and blocking tasks. Use the connection plugin's own
timeout settings for blocking work. A synchronous Rust invocation can still
run an async task with a deadline.
For async task bodies, the selected limit starts when that task begins on a
host and covers connection setup, checks, attempts, retry waits, and
verification. Retry attempts share the same deadline. An expired task reports
a host-scoped timeout failure and starts no further attempt or subtask.
In an ordered task list, each eligible later root gets a fresh deadline.
Unconfirmed cleanup prevents later work on that host; the list's failure and
timeout controls can also make a root ineligible.

Cleanup has a maximum **five-second observation period** per affected task-host,
separate from the task deadline. Timeout details in the failed host result's
`details` distinguish deadline expiry, cancellation requested, local completion,
connection close requested, and whether `close()` returned. The
`connection_close_observed` field records that the local close method returned;
the connection API cannot confirm that the network socket closed successfully.
Neither that field nor future polling stopped establishes the device outcome.
Other hosts continue normally. A host with unconfirmed local
cleanup remains unavailable to later invocations on the same `Genja` runtime
and its filtered views. This guard is separate from the connection cache;
closing or evicting a cached connection does not clear an unconfirmed host.
If cleanup is still unconfirmed when the grace period ends, that runtime keeps
the host unavailable; it does not infer completion from elapsed time.

Timeout details include `cleanup_elapsed_seconds`, measured after the task
deadline, and `external_state_known: false`. The Python bridge also reports
`python_coroutine_completion_observed` when it schedules a coroutine. These
diagnostics appear only on deadline timeout results; ordinary result JSON has
the same shape as before.

=== ":fontawesome-brands-rust: Rust"

    On an async task deadline, Genja stops polling the task future immediately.
    After releasing its connection lock, Genja requests closure of the managed
    connection. If closure remains unconfirmed, later work on that host is
    skipped, including in later invocations on the same runtime. A synchronous
    Rust `run_task()` call can run an async task with the same deadline behavior.
    Its temporary Tokio runtime shuts down without waiting indefinitely for
    unrelated blocking workers.

=== ":fontawesome-brands-python: Python"

    `run_task_async()` and `run_tasks_async()` capture the running Python event
    loop. A Python async task without a Genja-managed connection can use a
    deadline: Genja requests coroutine cancellation and observes whether it
    actually exits. This does not confirm that child tasks it spawned stopped.
    If it stays active beyond the cleanup grace, the host remains unavailable
    on the same runtime and its filtered views. Python async tasks that require
    connection setup reject deadlines until that setup bridge can also report
    cancellation completion.
    Synchronous Python invocation methods reject an effective deadline on a
    Python async task because that bridge path cannot observe cancellation.

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

### Concurrent invocations on the same host

Each `run_task` or `run_tasks` call schedules work independently. Runner worker
limits and the `serial` runner apply within one invocation; they do not
serialize separate invocations. Concurrent async calls or calls from separate
OS threads can run task bodies on the same host at the same time. The connection
cache protects individual connection operations, not the full task. Steps may
interleave, and timeout cleanup in one invocation may affect a connection used
by another. Genja does not guarantee ordering or isolation between concurrent
invocations on the same host.

These examples assume both tasks select the same host. The first pair in each
tab can overlap; run potentially conflicting work sequentially as shown after
them.

=== ":fontawesome-brands-rust: Rust"

    Separate OS threads can overlap synchronous calls:

    ```rust
    use std::{sync::Arc, thread};

    let shared = Arc::new(genja);
    let first_runtime = Arc::clone(&shared);
    let first = thread::spawn(move || first_runtime.run_task(FirstTask, 0));
    let second_runtime = Arc::clone(&shared);
    let second = thread::spawn(move || second_runtime.run_task(SecondTask, 0));
    first.join().unwrap()?;
    second.join().unwrap()?;
    ```

    Starting two async calls together can also overlap:

    ```rust
    let (first, second) = tokio::join!(
        genja.run_task_async(FirstTask, 0),
        genja.run_task_async(SecondTask, 0),
    );
    first?;
    second?;
    ```

    Await or finish the first invocation before starting the second:

    ```rust
    // Synchronous caller
    genja.run_task(FirstTask, 0)?;
    genja.run_task(SecondTask, 0)?;

    // Inside an async function
    genja.run_task_async(FirstTask, 0).await?;
    genja.run_task_async(SecondTask, 0).await?;
    ```

=== ":fontawesome-brands-python: Python"

    Separate OS threads can overlap synchronous calls:

    ```python
    from concurrent.futures import ThreadPoolExecutor

    with ThreadPoolExecutor(max_workers=2) as pool:
        first = pool.submit(runtime.run_task, FirstTask)
        second = pool.submit(runtime.run_task, SecondTask)
        first.result()
        second.result()
    ```

    Starting two async calls together can also overlap:

    ```python
    import asyncio

    await asyncio.gather(
        runtime.run_task_async(FirstTask),
        runtime.run_task_async(SecondTask),
    )
    ```

    Await or finish the first invocation before starting the second:

    ```python
    # Synchronous caller
    runtime.run_task(FirstTask)
    runtime.run_task(SecondTask)

    # Inside an async function
    await runtime.run_task_async(FirstTask)
    await runtime.run_task_async(SecondTask)
    ```

If timeout cleanup remains unconfirmed, later work that reaches the host check
is skipped, including in separate invocations sharing the runtime. This does
not stop another invocation's work that had already started. Sequential calls
order the invocations, but a timeout result does not establish the outcome of
remote work. Inspect timeout results before starting another task on that host.

### List failure and timeout controls

The [Tasks guide control table](tasks.md#runtime-invocation-options) lists both
options for each control and their combined effect on ordinary roots.

Timeout control defaults to `SkipRemainingForHost`: a deadline expiry skips
later ordinary roots on that host. A recovery root can still run after timeout
cleanup is confirmed. If its full tree passes, later ordinary roots become
eligible again. Unconfirmed cleanup prevents recovery. Failure control
separately blocks ordinary roots after any failed result in the tree, so a
timeout can set both blocks.

Deadline timeout details report the configured limit and elapsed seconds, the
phase, and the number of attempts that actually started. An expiry before the
first attempt reports attempt `0`. `local_completion_observed` says whether
the awaited local work finished before the result was recorded; it does not
confirm the outcome of a remote operation. Genja task deadlines are unsupported
for blocking task bodies. Cancellation and cleanup grace periods are separate
from async task deadlines.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::{TaskListRunOptions, TaskRunOptions};
    use std::time::Duration;

    let options = TaskListRunOptions::new(
        TaskRunOptions::new(1).with_task_timeout(Duration::from_secs(30)),
    );
    let results = genja.run_tasks_with_options(tasks, options)?;
    ```

    Existing calls that pass `TaskRunOptions` use both blocking defaults. The
    async and custom-retry-policy list methods accept `TaskListRunOptions` too.

    To let later ordinary roots run after failures or deadlines, set both
    controls to `Continue`:

    ```rust
    use genja::{
        TaskListFailureControl, TaskListRunOptions, TaskListTimeoutControl,
        TaskRunOptions,
    };

    let options = TaskListRunOptions::new(TaskRunOptions::new(1))
        .with_failure_control(TaskListFailureControl::Continue)
        .with_timeout_control(TaskListTimeoutControl::Continue);
    let results = genja.run_tasks_with_options(tasks, options)?;
    ```

    Unconfirmed timeout cleanup still keeps the host unavailable. With either
    control set to `SkipRemainingForHost`, a recovery root can restore
    eligibility after its full tree passes, provided timeout cleanup was
    confirmed.

=== ":fontawesome-brands-python: Python"

    ```python
    import genja

    options = genja.TaskListRunOptions(
        task_options=genja.TaskRunOptions(task_timeout=30),
    )
    results = runtime.run_tasks(tasks, run_options=options)
    ```

    `run_tasks_async(...)` accepts the same options. Omitting `task_options`
    uses the runner's default depth and timeout settings.

    To let later ordinary roots run after failures or deadlines, set both
    controls to `CONTINUE`:

    ```python
    options = genja.TaskListRunOptions(
        failure_control=genja.TaskListFailureControl.CONTINUE,
        timeout_control=genja.TaskListTimeoutControl.CONTINUE,
    )
    results = runtime.run_tasks(tasks, run_options=options)
    ```

    Unconfirmed timeout cleanup still keeps the host unavailable. With either
    control set to `SKIP_REMAINING_FOR_HOST`, a recovery root can restore
    eligibility after its full tree passes, provided timeout cleanup was
    confirmed.

Rust's default failure-blocking mode calls the runner's `run_task` method for
each root to make host eligibility decisions between roots. A custom runner's
`run_tasks` override is used only when both failure and timeout controls are
explicitly `Continue`.

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
