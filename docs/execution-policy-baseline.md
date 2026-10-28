# Execution policy design: baseline, retry, deadline, and verification

This note records the current execution behavior and the proposed retry,
deadline, and verification contracts from design phases 2 and 3. The baseline
describes the current implementation; the proposals are not yet implemented.
The relevant paths are `Genja::run_task*_with_options` and
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
There is no task-host monotonic execution deadline or cancellation status in
this path. The phase 3 design below distinguishes a deadline signal from
confirmed termination of blocking, Python, or external work.

For invocations without a newly selected execution policy, the compatibility
baseline is the same effective retry fields, decisions, fixed waits, task
attempt counts, convergence outcomes, dry-run behavior, fresh-session
replacement behavior, processor ordering, and serialized result shape.

## Proposed retry contract (phase 2)

### Selection and configuration

Select an `ExecutionPolicy` once for a single-task or ordered-list invocation.
It groups retry, an invocation-selected task timeout, and verification
requirements.
Phase 2 defines its retry selection; phase 3 defines the other fields. The
proposed shape is:

```rust
pub struct ExecutionPolicy {
    pub retry: Option<Arc<dyn RetryPolicy>>,
    pub task_timeout: Option<Duration>,
    pub verification: VerificationRequirement,
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
`TaskFailure.retryable()` is true; and no task-host deadline or cancellation
state that forbids another attempt. A `RetryAfter` decision cannot override
any gate. The runtime rechecks time and cancellation after the wait, before
starting the next attempt. Phase 3 defines those signals below.
Successful, warning, skipped, and non-retryable results stop without policy
evaluation.

`retryable = true` remains the task's explicit assertion that repeating its
operation is safe under the current retry settings. If the task declares
`Check` or `CheckAndVerify`, the existing pre-check runs at the start of each
new attempt; a converged check ends with the existing
`PassedWithWarnings` result. A custom policy cannot bypass that check. If
Genja knows a prior change may have applied and cannot confirm the previous
work has stopped, a new policy must not turn that uncertainty into a repeat
attempt. The phase 3 deadline/cancellation contract must carry this as an
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
remains `attempts > 1`, and `retry_exhausted` retains its current limit
meaning: a retryable final failure at the configured maximum when retries
were enabled and the maximum exceeded one. A policy
that stops earlier does not mark the limit exhausted. Initial connection
failures and fresh-session replacement failures remain outside task retry
decisions. Processor hooks and final result recording remain in their
current positions.

## Proposed deadline and verification contract (phase 3)

### One monotonic deadline per task on each host

`ExecutionPolicy.task_timeout: Option<Duration>` supplies a configurable
fallback duration for **each task-host execution**. It is not one clock for
the whole invocation, the whole host across a task list, or each retry attempt. A task
may select its own `task_timeout` setting. Public task settings are positive
integer **seconds**; the runtime converts them to `Duration` and computes a
monotonic deadline internally. The proposed precedence is:

1. A root `TaskDefinition` runtime override, including a future
   `TaskSpecOverrides.task_timeout` field, applies to that root only.
2. Otherwise, the Rust task's `TaskInfo` metadata or Python `@task` metadata
   applies to that task. The Rust `#[genja_task(...)]` attribute macro and
   Python `@task(...)` decorator must both expose this optional setting.
   Subtasks use their own metadata.
3. Otherwise, use the invocation's `task_timeout`. If no duration is
   selected, the task-host execution has no deadline.

There is an existing Rust invocation API gap for root overrides:
`Genja::run_task*` accepts a `T: Task` and constructs its own
`TaskDefinition`, so callers cannot pass an already configured definition
to those single-task methods. A constructed `TaskDefinition` can be pushed
into `Tasks` and run with `Genja::run_tasks*`; `TaskSpec` construction also
returns a definition for this path. Phase 4 will specify the public
single-definition and list-addition helpers for declarative invocation.

`ExecutionPolicy` is the invocation's grouped configuration, not a trait
that votes on every timeout. The runtime's timeout resolver applies the
precedence above and computes a monotonic cutoff. The `RetryPolicy` trait
makes retry decisions only; it cannot extend or reset that cutoff. For
example, suppose an invocation selects a 30-second `task_timeout` and a
custom retry algorithm:

| Execution | Resolved timeout | Clock start |
| --- | --- | --- |
| Root A declares `task_timeout = 10` on host 1 | 10 seconds | When A begins on host 1 |
| Root A on host 2 | 10 seconds | When A begins on host 2, even if it waited for a worker |
| A's subtask declares `task_timeout = 5` on host 1 | 5 seconds | When the subtask begins after A's result |
| Root B has no task timeout on host 1 | 30 seconds | When B begins on host 1 |

Each row gets one cutoff for its connection, task attempts, retry waits,
and verification. A retry failure from A is passed to the selected
`RetryPolicy` with A's own attempt count and settings. Its next wait must
fit within A's remaining 10-second budget; the policy cannot grant more
time. Root B and host 2 do not spend A's time budget.

An optional task setting should distinguish *inherit* from *disable the
default*. A concrete shape is `Option<TaskTimeoutOverride>`, where `None`
inherits, `Disabled` removes the invocation default, and `After(Duration)`
replaces it. Reject a zero-second timeout during configuration validation;
the explicit `Disabled` setting expresses no limit. A root override does not
propagate to subtasks. One duration is resolved for each task on each host before that
task begins; the selected retry algorithm remains separately inherited
across the task tree.

This design issue includes the public task-facing setting for both Rust and
Python, plus its representation in task descriptors and `TaskSpec` overrides.
The proposed macro and decorator keyword is `task_timeout`, with an integer
number of seconds:

```rust
#[genja_task(name = "deploy", task_timeout = 30)]
```

```python
@task(name="deploy", task_timeout=30)
```

Phase 4 must settle the exact binding shape for explicitly disabling an
inherited default and how the seconds value appears in task descriptors.
Adding those APIs and enforcing the deadline belong to implementation
follow-ups. Existing tasks with no setting continue to
inherit the invocation default, or have no deadline when no default exists.

At entry to `TaskDefinition::start_with_depth` for a task-host pair, capture a
monotonic `Instant` and derive one cutoff from the resolved duration. This
starts after a runner schedules that host job, so threaded queue time and
earlier roots do not consume this task's budget. The cutoff covers that
task's validation, initial connection, dry-run or normal entrypoint,
convergence checks, all task retry attempts and waits, fresh-session
replacement attempts and waits, deferred post-check, instance processor
hooks, and final host result recording. It is never reset for a retry or
replacement attempt. Each subtask starts a fresh task-host clock after the
parent result is recorded. Aggregate `on_task_start` and `on_task_finish`
hooks run outside individual task-host budgets.

For example, with five threaded host slots and a 30-second default, hosts
1-5 start their own 30-second clocks when their task-host jobs begin. Host 6
waits for a slot; it receives a fresh 30 seconds when its job begins, even
if the first five hosts used most of their budgets. A later root in a task
list, and each subtask on each host, also receives a fresh clock unless it
has a different task setting. This makes the limit fair to queued hosts,
but it does not impose a 30-second upper bound on the whole task or run.
If an uninterruptible blocking worker remains occupied after expiry, Genja
cannot promise when a queued host will obtain a worker slot.

Check remaining time before starting another step and after each awaited
operation. Bound retry and replacement waits by the remaining time; expiry
during a wait prevents the next attempt. Existing `SystemTime` values remain
for human-readable result timestamps, while `Instant` governs expiry. This
scope works the same for serial and threaded runners, Rust blocking and
async tasks, Python-backed tasks, and direct core task execution. An absent
deadline leaves the current timing and result behavior unchanged.

The cutoff is a request to stop this task-host execution and cancel work
that can be cancelled. It is not a guarantee that the call returns by the
cutoff. Synchronous processor hooks, connection/plugin calls without
cancellation support, running `spawn_blocking` closures, Python methods,
and external commands may continue past it. Expiry on one host does not
stop other hosts or roots. If the parent has unconfirmed active work when
its deadline expires, do not start its subtasks on that host while it might
still be changing external state. If local termination is confirmed, the
current subtask behavior after a failed parent can continue, with each
subtask receiving its own deadline.

### Cancellation and knowledge of termination

The report must keep these facts separate:

| Fact | Meaning |
| --- | --- |
| `deadline_expired` | This task-host's monotonic cutoff was reached. It says nothing by itself about whether its active work stopped. |
| `cancellation_requested` | Genja asked owned work to stop or stopped scheduling it. The trigger may be deadline expiry or an explicit caller request. |
| `local_termination_confirmed` | Genja observed that the relevant local future, thread work, or Python call actually finished or acknowledged cancellation. |
| `external_state` | The task's remote or external change is known only from a completed task result or later verification; otherwise it remains unknown. |

Use a distinct stop trigger such as `DeadlineExpired` or
`CallerCancellation` and an independent local termination state such as
`Confirmed` or `Unconfirmed`. Do not call an expired deadline a confirmed
termination. If an async task returns a result before the cutoff is observed,
retain that result. If expiry wins the race, request cancellation, retain any
later result actually observed during cleanup, and report uncertainty for
work that never produces a result. Dropping a Tokio future does not prove
that spawned work or a remote operation stopped.

An explicit caller cancellation request needs a reportable cancellation
handle or token, to be defined with the phase 4 API. Simply dropping the
async invocation future may prevent Genja from returning any report at all;
it must not be described as a reported, confirmed cancellation.

Async Rust work can usually stop at an await or cooperative cancellation
point, but completion or acknowledgement is needed to confirm local
termination. Tokio cannot forcibly stop a running `spawn_blocking` closure.
Blocking Rust and synchronous Python methods therefore remain unconfirmed
until their worker finishes. Python async methods run through the PyO3
bridge into `asyncio`; cancelling the Rust-side future alone does not prove
the Python coroutine has stopped. Where the bridge falls back to
`asyncio.run()`, the call may block the Tokio worker until Python returns.
None of these local observations proves that a remote device or external
process rolled back or stopped changing state.

A deadline or cancellation interruption of active work is never marked
`retryable` merely because its local wait ended. The phase 2 safety gate
rejects another task attempt while prior local work is unconfirmed or
external effects are uncertain. Reconciliation may use the existing
convergence or fresh-session checks only when their own execution can be
observed safely; an uncompleted check does not establish convergence.

### Result shape for deadline-aware calls

Report expiry on the affected task-host result, with the trigger,
cancellation request, local termination state, and external-state certainty
kept distinct. For active work without a completed result, use a non-retryable
timeout failure with explicit uncertainty metadata; the existing
`TaskFailureKind::Timeout` is available. If the deadline expires before the
first task attempt, do not fabricate `attempts = 1`: the policy-aware result
must report zero task attempts. An interrupted parent with unconfirmed work
must also make its unstarted subtasks distinguishable from completed ones.
Expiry after an application result but before a required post-check or fresh
session completes records verification as incomplete and preserves the fact
that the change may already have been applied; it must not turn that result
into an ordinary verified success or retry the application.
The exact optional result fields and Rust/Python wrappers are phase 4 API
decisions. Existing calls with no selected policy retain their raw and human
JSON shape; Python result conversion must expose the same interruption facts
when a deadline is selected.

### Verification is a requirement over existing checks

Start with a value setting, not a public verification trait:

```rust
pub struct VerificationRequirement {
    pub require_post_check: bool,
    pub require_fresh_session: bool,
}
```

Both fields default to `false`, meaning **as configured today**. A required
post-check needs the task to declare `IdempotencyMode::CheckAndVerify` and
uses its existing check hook before application and after a passed result.
A converged pre-check already proves the desired state and does not need a
second post-check because no application ran.
A required fresh session needs `SessionVerificationConfig`, a connection
plugin, and a resolver. Its existing replacement loop still runs only after
a passed, changed, non-dry-run result. When both requirements are selected,
the post-check runs on that replacement connection as it does today. Neither
requirement creates another execution loop, changes task retry eligibility,
or retries failed verification as a task application.

Apply requirements to every task node in the invocation, including roots in
an ordered list and nested subtasks. Validate a known root before dispatch;
validate each dynamically supplied subtask before its connection or task
entrypoint. A missing capability records a validation failure for that
task-host node without applying its change. Because subtasks can be produced
during execution, preflight cannot promise that a whole invocation will
succeed without earlier tasks having run. A dry-run invocation with a
required post-check or fresh session is rejected before execution: dry-run
currently skips automatic checks and replacement, so silently claiming a
requirement was met would be false.

Without a verification requirement, retain the current behavior exactly:
`Check` and `CheckAndVerify` operate only when task-authored, fresh-session
verification operates only when configured, a converged retry pre-check can
return `PassedWithWarnings`, and dry-run bypasses both automatic convergence
and fresh-session checks. The requirement setting asserts that existing
checks are present and observed; it does not introduce a new public trait
or infer verification from a successful task result alone.
