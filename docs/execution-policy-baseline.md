# Execution policy design: baseline and retry contract

This note records the current execution behavior and the proposed phase 2 retry
contract. The baseline describes the current implementation; the proposal is
not yet implemented. The relevant paths are `Genja::run_task*_with_options` and
`Genja::run_tasks*_with_options` in
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

## Proposed retry contract (phase 2)

### Selection and configuration

Select an `ExecutionPolicy` once for a single-task or ordered-list invocation.
It is the intended grouped home for retry, total deadline, and verification
requirements. Phase 2 defines only its retry selection; the deadline and
verification fields belong to phase 3. A sketch of the retry portion is:

```rust
pub struct ExecutionPolicy {
    pub retry: Option<Arc<dyn RetryPolicy>>,
    // Phase 3: total deadline and verification requirement.
}
```

`retry: None` means **no new retry algorithm was selected**. The runtime
resolves it to a built-in `FixedDelayRetryPolicy`, so the same `RetryPolicy`
interface handles every eligible decision. This does not mean retries are
disabled. `retry: Some(...)` selects a different algorithm; it does not
change `allow` or the attempt limit. An explicit `allow = false` or
`max_attempts = 1` still results in one task attempt.
The initial contract selects one algorithm for the whole invocation; it has
no per-root algorithm override.

For every task node on every host, resolve effective settings exactly as in
the baseline. The precedence is per field where `RetryConfig` fields are
optional:

| Source | Effect |
| --- | --- |
| Core defaults | `allow = false`, `max_attempts = 1`, `delay_ms = 0` when no runner configuration is supplied |
| `runner.retry` | Supplies normalized invocation defaults to runner-mediated execution |
| Rust `retry(...)` or Python `RetryConfig` task metadata | Supplies only the fields the task declares; other fields inherit runner defaults |
| Root `TaskDefinition` override, including `TaskSpecOverrides.retry` | Replaces that root's authored retry metadata; unset fields still inherit runner defaults; does not apply to subtasks |

Selecting an algorithm does not become another precedence layer for these
settings. In particular, a retry-disabled root in the same ordered list is
still disabled, and a subtask can have a different maximum from its parent.

### Object-safe interface

The minimum public interface should resemble the following. Names are
provisional; the data and ownership contract are the design decision.

```rust
pub trait RetryPolicy: Send + Sync {
    fn decide(&self, context: &RetryContext<'_>) -> RetryDecision;
}

// The policy module owns these legacy defaults and the resolver that merges
// runner and task settings before each task-host execution.
pub struct RetrySettings {
    pub allow: bool,
    pub max_attempts: usize,
    pub delay: Duration,
}

pub struct RetryContext<'a> {
    pub result: &'a HostTaskResult,
    pub attempt: usize,             // completed task attempts, starting at 1
    pub settings: &'a RetrySettings, // resolved for this task-host execution
    pub idempotency_mode: IdempotencyMode,
}

pub enum RetryDecision {
    Stop,
    RetryAfter(Duration),
}
```

The policy sees the actual `HostTaskResult` after the task call and any
immediate convergence post-check. It can inspect `result.failure()` and
`TaskFailure::retryable()`; it does not need an invented error taxonomy.
`RetrySettings` is a resolved, read-only value for this task and host.
`RetryDecision` only controls whether to continue the existing loop and how
long to wait. It cannot rewrite a result, start a separate loop, or change
the maximum. The built-in `FixedDelayRetryPolicy` returns `RetryAfter` using
the resolved delay when eligible. A custom algorithm may choose another delay
or stop early. The runtime performs the async wait.

The policy module owns both `FixedDelayRetryPolicy` and the default
`RetrySettings` (`allow = false`, `max_attempts = 1`, zero delay). It resolves
runner and task overrides into a new settings value for each task-host
execution before calling the policy. Keeping common settings in that value,
rather than in the shared policy object, preserves task-specific overrides
and independent limits. The handoff has three stages:

1. At the invocation boundary, resolve `retry: None` to the built-in policy
   or retain the selected custom policy. Share this algorithm handle across
   the invocation.
2. At the start of each task on each host, merge the policy module's default
   settings with runner defaults and that task's metadata or root override.
   Keep the resulting `RetrySettings` and attempt counter local to this
   task-host execution.
3. After an eligible failed attempt, pass those settings by read-only context
   to `RetryPolicy::decide`. The policy returns a decision; it does not receive
   ownership of the settings or retain them for later tasks.

In outline:

```rust
let policy = selected_retry_policy.unwrap_or_else(default_fixed_delay_policy);
// Repeated separately for each task-host execution:
let settings = resolve_retry_settings(runner_config, task.retry_config());
// After an attempt, and only when the runtime safety gates allow a retry:
let context = RetryContext { result: &result, attempt, settings: &settings, idempotency_mode };
let decision = policy.decide(&context);
```

The selected policy may hold algorithm-specific parameters, such as a backoff
multiplier. The common `allow`, maximum-attempt, and base-delay settings stay
in each task-host's resolved settings value throughout the workflow. They
are never loaded into the shared policy object.

The runtime applies non-overridable gates before accepting `RetryAfter`:
effective `allow = true`; `attempt < max_attempts`; a failed result whose
`TaskFailure.retryable()` is true; and no invocation deadline or cancellation
state that forbids another attempt. A `RetryAfter` decision cannot override
any gate. The runtime rechecks time and cancellation after the wait, before
starting the next attempt. Phase 3 will define those signals precisely.
Successful, warning, skipped, and non-retryable results stop without policy
evaluation.

`retryable = true` remains the task's explicit assertion that repeating its
operation is safe under the current retry settings. If the task declares
`Check` or `CheckAndVerify`, the existing pre-check runs at the start of each
new attempt; a converged check ends with the existing
`PassedWithWarnings` result. A custom policy cannot bypass that check. If
Genja knows a prior change may have applied and cannot confirm the previous
work has stopped, a new policy must not turn that uncertainty into a repeat
attempt. The future deadline/cancellation contract must carry this as an
explicit runtime signal; policy code must not infer certainty from arbitrary
`TaskFailure.details` JSON. Until that signal exists, no selected algorithm
may claim stronger cancellation safety than the current task path provides.

### Propagation and state ownership

The invocation resolves its optional selection to one `Arc<dyn RetryPolicy>`
and passes a clone of the handle through the runner to every root task and
through `start_with_depth` to every subtask. This is inheritance of the
algorithm only. The attempt counter is a local variable in each task-host
execution, never a field of the invocation or policy object.

`RetryPolicy::decide` takes `&self` and receives no mutable attempt budget.
Implementations must use the supplied context for attempt accounting;
putting a cross-task or cross-host attempt budget behind interior mutability
would violate this contract. A first version does not expose policy-owned
per-task state. If a later algorithm needs state, it must be created for
each task-host execution, not stored in the shared policy handle.

For example, with one selected algorithm and runner defaults allowing three
attempts: root A may use three attempts on host 1, root B with `allow = false`
uses one on the same host, and A's subtask with `max_attempts = 2` uses at
most two. Host 2 gets independent counters for all three tasks. Neither
root order nor host scheduling changes these limits.

### Legacy path and result compatibility

When no new policy is selected, the built-in `FixedDelayRetryPolicy` must
reproduce the existing `EffectiveTaskRetryPolicy` decision and delay exactly,
whether execution enters through Rust, Python, a built-in runner, or the
direct core task path. The runtime retains the current `retry_exhausted`
calculation. A retry refactor should switch the legacy path to this built-in
policy only after equivalence is demonstrated. The current serialized
`HostTaskResult` and Python result shape stay unchanged for these calls.

For a selected algorithm, `execution_metadata.attempts` still counts task
attempts, including an attempt settled by a convergence pre-check. `retried`
remains `attempts > 1`, and `retry_exhausted` retains
its current limit meaning: a retryable final failure at the configured
maximum when retries were enabled and the maximum exceeded one. A policy
that stops earlier does not mark the limit exhausted. Initial connection
failures and fresh-session replacement failures remain outside task retry
decisions. Processor hooks and final result recording remain in their
current positions.
