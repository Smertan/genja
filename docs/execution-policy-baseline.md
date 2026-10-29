# Execution policy design: baseline, retry, deadline, and verification

This note records the current execution behavior and the proposed retry,
deadline, verification, API, and validation contracts from design phases 2-5. The
baseline describes the current implementation; the proposals are not yet
implemented.
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
    pub task_timeout: Option<u64>, // whole seconds
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

`ExecutionPolicy.task_timeout: Option<u64>` supplies a configurable
fallback duration for **each task-host execution**. It is not one clock for
the whole invocation, the whole host across a task list, or each retry
attempt. A task may select its own `task_timeout` setting. Public timeout
durations are positive, whole **seconds**; the runtime converts them to
`Duration` and computes a monotonic deadline internally. The proposed
precedence is:

1. A root `TaskDefinition` runtime override, including a future declarative
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
inherits, `Disabled` removes the invocation default, and `AfterSeconds(u64)`
replaces it. Reject a zero-second timeout during configuration validation;
the explicit `Disabled` setting expresses no limit. A root override does not
propagate to subtasks. One duration is resolved for each task on each host
before that task begins; the selected retry algorithm remains separately
inherited across the task tree.

This design issue includes the public task-facing setting for both Rust and
Python. Phase 4 records the compatibility cost of adding the setting to
task descriptors and `TaskSpec` overrides.

The proposed macro and decorator keyword is `task_timeout`, with an integer
number of seconds:

```rust
#[genja_task(name = "deploy", task_timeout = 30)]
```

```python
@task(name="deploy", task_timeout=30)
```

Phase 4 specifies the explicit-disable syntax and descriptor compatibility
boundary below. Adding those APIs and enforcing the deadline belong to
implementation follow-ups. Existing tasks with no setting continue to
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
handle or token, scoped in the phase 4 API contract below. Simply dropping the
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

## Proposed API and compatibility contract (phase 4)

These are design decisions for follow-up implementation, not APIs available
in the current release. The new invocation path must converge on the same
runner and `TaskDefinition::start_with_depth` execution path as existing
calls. It must not duplicate the host or retry loops.

### Constructed definitions and declarative invocation

Add `Tasks::add_definition(TaskDefinition)`. It appends the supplied
definition unchanged, preserving root processor selections and retry,
session verification, or future task-timeout overrides. Keep
`Tasks::add_task(T: Task)` as the convenience that creates
`TaskDefinition::new(task)`. Keep the existing `DerefMut<Vec<TaskDefinition>>`
behavior, including `tasks.push(definition)`, for source compatibility; the
new named method makes constructed definitions clear at call sites.

Add a single-definition family to `Genja`:

| Proposed Rust method | Input | Result |
| --- | --- | --- |
| `run_task_definition` | `TaskDefinition`, `max_depth` | `TaskResults` |
| `run_task_definition_async` | `TaskDefinition`, `max_depth` | `TaskResults` after await |
| `run_task_definition_with_options` | `TaskDefinition`, `TaskRunOptions` | `TaskResults` |
| `run_task_definition_with_options_async` | `TaskDefinition`, `TaskRunOptions` | `TaskResults` after await |

The existing private `run_task_definition_async` helper already feeds a
definition to the configured runner. Rename that helper internally when
adding the public async name, then route all four methods through it.
Synchronous methods retain the existing rule that they cannot run inside
an active Tokio runtime. Do not change the generic `run_task*` signatures;
they continue to accept ordinary `T: Task` and create the definition.

This closes the current `TaskSpec` single-task gap. The construction helper
already returns a `TaskDefinition`, so the proposed call is:

```rust
let definition = create_compiled_task_from_spec_str(source)?;
let result: TaskResults = genja.run_task_definition(definition, 0)?;
```

For an ordered list, callers may mix authored tasks and constructed
definitions without wrapping a definition again:

```rust
let mut tasks = Tasks::new();
tasks.add_task(MyTask);
tasks.add_definition(create_compiled_task_from_spec_str(source)?);
let results: Vec<TaskResults> = genja.run_tasks(tasks, 0)?;
```

`run_task_definition*` returns the same task-tree result as `run_task*`.
`run_tasks*` still returns an outer vector with one `TaskResults` per root,
in input order. No extra `tasks` node appears inside either result. A
`TaskSpec` remains one registered-task construction request, rather than a
task-list or controller workflow format.

### Invocation policy and runner transport

Keep `TaskRunOptions` focused on depth and dry-run. It currently implements
`Copy`; storing an `Arc<dyn RetryPolicy>` in it would break that public
property. Pass the grouped `ExecutionPolicy` separately through new
policy-aware invocation methods for a raw task, one constructed definition,
and an ordered `Tasks` list. Each needs sync and async forms. The proposed
policy-aware form takes `TaskRunOptions` and `ExecutionPolicy`. The method
families are `run_task_with_policy`, `run_task_definition_with_policy`, and
`run_tasks_with_policy`, each with an `_async` counterpart. The simple
depth-only entrypoints above can remain policy-free. For example:

```rust
genja.run_task_definition_with_policy(definition, run_options, policy)?;
genja.run_tasks_with_policy(tasks, run_options, policy)?;
```

Keep all existing `Genja::run_task*` and `Genja::run_tasks*` invocation
signatures and behavior. The `PluginRunner` signatures change below.
`ExecutionPolicy::default()` selects no custom retry algorithm, no timeout,
and no verification requirement, and must produce the same results as the
current methods. A selected policy handle is shared through an invocation;
the runner passes it to each root and the core recursion passes it to each
subtask. Task-host retry settings, attempt counters, and deadlines are
resolved locally as described above.

Change the existing `PluginRunner::run_task` signature to take an
`&ExecutionPolicy`; change `run_tasks` to take the same argument and forward
it to `run_task` in its default implementation. Do not add parallel
policy-aware runner methods. Define `ExecutionPolicy` in `genja-core` so
both the plugin-manager trait and the runtime can use the same public type.
In outline, the runner contract becomes:

```rust
async fn run_task(
    &self,
    task: &TaskDefinition,
    hosts: &Hosts,
    connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
    runner_config: &RunnerConfig,
    run_options: TaskRunOptions,
    policy: &ExecutionPolicy,
) -> Result<TaskResults, GenjaError>;
// run_tasks receives the same trailing policy argument and passes it to
// run_task for each root in its default implementation.

fn supports_execution_policy(&self) -> bool { false }
```

The invocation owns the policy. Make the policy a required argument to
`TaskExecutor::new` and `TaskExecutor::run_host`; the serial executor stores
a reference and forwards it for each host, while the threaded runner moves
an owned policy handle into each spawned job and borrows it for `run_host`.
There is no implicit default inside `TaskExecutor`. This makes a missing
handoff a compile error for the built-in runners. Each task-host execution
still resolves its own settings, attempt counter, and deadline.

`TaskExecutor` is crate-private, so its signature cannot enforce forwarding
by third-party runners. Add a policy-aware `TaskDefinition` host-execution
entry point in `genja-core` that requires `&ExecutionPolicy` and funnels into
`start_with_depth`; pass the same policy through recursive subtask calls.
Keep existing public `TaskDefinition::start*` entry points as compatibility
wrappers that call this path with `ExecutionPolicy::default()`. A custom
runner claiming policy support must use the policy-aware core entry point
or provide equivalent enforcement. The capability check rejects runners
that have not opted in; an opt-in is the runner author's promise to enforce
the selected features, not a runtime proof of their implementation.

This is a deliberate source-breaking change for third-party Rust runner
implementations: add the policy parameter to `run_task` and to any
overridden `run_tasks` method. Existing invocation methods pass
`ExecutionPolicy::default()`, preserving their runtime behavior and result
shape. An implementation may initially leave its existing execution logic
unchanged for that default value. The defaulted capability method above
means the runtime rejects a non-default invocation policy before any task
starts unless the runner opts in. Non-default means a custom retry algorithm,
an invocation timeout, or a verification requirement was selected. Runners
that forward and enforce all selected policy features opt in. Accepting the
new argument without support must never silently ignore a selected policy.

### Rust task metadata and declarative formats

Add a default `TaskInfo` timeout getter so existing hand-written `TaskInfo`
implementations compile unchanged. The Rust attribute macro records
`task_timeout` in whole seconds and generates the getter; the Python task
adapter implements the same getter from decorator metadata. A root
`TaskDefinition` timeout override is an optional runtime value and does not
alter the authored task or subtask metadata. The invocation fallback is
`ExecutionPolicy.task_timeout`, which is resolved after the root override
and authored metadata.

Use `Option<TaskTimeoutOverride>` for the getter and root override:
`None` means inherit, `AfterSeconds(positive_u64)` sets the duration, and
`Disabled` suppresses the invocation fallback. The attribute macro accepts
`task_timeout = 30` or `task_timeout = "disabled"`. The Python decorator
accepts `task_timeout=30` or a typed `TaskTimeout.DISABLED` value; omitting
the argument inherits. Zero seconds is invalid in all public forms. The
`ExecutionPolicy.task_timeout` field is an optional whole-second fallback,
with `None` meaning no invocation timeout. Validate values before running
the task. The runtime creates an `Instant` cutoff only after resolving the
value for a specific task and host.

`TaskDescriptor` and `TaskDescriptorMetadata` are public Rust structs with
public fields. `TaskSpecOverrides` is also a public struct and currently
rejects unknown serialized fields. Adding `task_timeout` to any of these
now would break Rust callers that use struct literals; adding it to
serialized descriptors would also change catalog output. Preserve those
shapes in the first implementation. Document that task timeout is available
at runtime but not yet in descriptor output or declarative `TaskSpec`
overrides. A later descriptor/spec change needs its own compatibility and
migration decision. Existing `TaskSpec` retry and session verification
overrides continue to map to the built-in behavior and are preserved by
`run_task_definition*` and `Tasks::add_definition`.

### Python invocation and runner APIs

Keep existing Python `Genja.run_task*` and `run_tasks*` calls accepting task
classes and `Tasks`, respectively. Add an optional policy keyword to those
entrypoints and to direct `TaskDefinition.run_on_host(s)` execution. When
omitted, Python follows the existing Rust path and returns the same result
shape. Expose a Python `ExecutionPolicy` value with a `task_timeout` in
whole seconds and the verification requirement. A Python-selected custom
retry algorithm requires a synchronous adapter for the object-safe Rust
`RetryPolicy::decide`; async policy callbacks are outside the initial
interface. Update the Python `.pyi` stubs and top-level re-exports with
the runtime binding in the same implementation change.

The Python `Genja.run_task*` and `run_tasks*` bindings pass the selected or
default policy to the Rust invocation, then through the runner and core
task-host path. The Python `TaskDefinition.run_on_host(s)` bindings bypass
the runner; their shared host-execution helper must pass the optional
policy to the policy-aware `genja-core` entry point. The Python task adapter
continues to call `start` or `start_async` as it does today: retry and
deadline decisions remain in Rust around that call, so task authors do not
need another method parameter.

Add the same trailing `ExecutionPolicy` argument to Python
`RunnerPluginBase.run_task` and `BatchRunnerPluginBase.run_tasks`; update the
PyO3 runner adapter, base class documentation, and typing signatures. This
is a deliberate source break for Python runner plugins. A plugin can accept
the argument without changing its current execution logic for the default
policy. The adapter rejects a selected non-default policy before dispatch
unless the Python runner's default-false `supports_execution_policy()` hook
declares support, so an ignored argument cannot silently disable an execution
requirement. Supporting runners must forward
the policy into task execution. The existing Python
`Tasks.add_task` stores decorated task classes, so a Python
`Tasks.add_definition` would also require changing its internal collection
representation; that is separate from the additive Rust method above.
There is no Python `TaskSpec` construction path to unblock in this phase.

The first policy-aware API reports cancellation *requested by deadline
expiry* and whether local termination was confirmed. An explicit caller
cancellation handle is a separate follow-up API: it must preserve a way to
await the final report, rather than relying on dropping a Rust future or
Python coroutine. Reserve `CallerCancellation` as a future trigger value;
do not claim that ordinary future dropping produces a report.

### Results and migration compatibility

An invocation that does not select a new policy still passes
`ExecutionPolicy::default()` internally. This does **not** disable existing
task, runner, Python, or `TaskSpec` retry settings. Those settings must
produce the same decisions, delays, attempt counts, and results as today.
For existing task definitions without a new `task_timeout` setting, raw and
human JSON must be identical, including `HostTaskOutcome`, retry flags,
`PassedWithWarnings`, dry-run, and session verification metadata. No new
interruption field is serialized when there was no deadline or interruption.

The runner-plugin migration is a separate source change. Document it as
breaking in the top-level `CHANGELOG.md` when implemented: Rust runner
implementations add the policy parameter to `run_task` and any `run_tasks`
override; Python runner plugins add it to their corresponding callbacks.
Existing `Genja` invocation call sites keep their signatures and need no
changes when they do not select a policy.

For a call with an effective task-host deadline, add optional interruption
facts to that host's execution metadata. Omit the field when absent in both
raw and human JSON. An expired host remains a host-level result inside the
same `TaskResults` tree; it does not change the invocation return type.
Python result conversion must expose the same optional facts. The timeout
case can use existing `TaskFailureKind::Timeout`; do not add a new
`HostTaskOutcome` variant merely to report expiry. If a new public enum
variant becomes necessary for caller cancellation, record that source
break and migration guidance before implementation.

The declarative helper additions are independently implementable from
retry refactoring, deadline enforcement, and verification integration.
They should be a focused first follow-up: add the Rust methods, document
them, and verify that a `TaskSpec`-constructed single task reaches the
same runner and returns one `TaskResults`.

Remaining follow-ups have separate boundaries: refactor the existing retry
decision behind the built-in and custom `RetryPolicy` path; enforce the
task-host timeout across async, blocking, and Python work; add cancellation
reporting that distinguishes requested from confirmed local termination;
and integrate verification requirements over the existing checks. Each
follow-up must preserve the no-policy compatibility baseline above.

## Validation walkthrough and test matrix (phase 5)

This phase maps the design to current tests and specifies the new cases for
implementation follow-ups. It does not claim that the proposed APIs or
deadline behavior already pass tests. Keep existing tests as the baseline;
compare their observable results before and after each implementation step.

### Current regression anchors

| Behavior | Existing tests and what they establish |
| --- | --- |
| Retry defaults, precedence, delay, and safety | `genja-core/src/task.rs`: `start_uses_task_retry_config_over_default_policy`, `start_merges_task_retry_config_over_runner_retry_config_field_by_field`, `start_delays_before_retry_attempts`, `start_does_not_retry_without_effective_retry_policy`, `start_does_not_retry_or_delay_non_retryable_failures`, and `start_does_not_retry_or_delay_skipped_results`. These anchor the built-in policy when no new algorithm is selected. |
| Attempt metadata and convergence | `genja-core/src/task.rs`: `task_runtime_context_exposes_current_retry_attempt`, `start_marks_retry_exhausted_after_final_retryable_failure`, `retry_converged_pre_check_returns_passed_with_warnings`, `idempotent_async_converged_check_skips_start_and_passes_unchanged`, and `check_and_verify_with_session_verification_uses_replacement_connection_for_post_check`. |
| Dry-run | `genja/src/lib.rs`: `run_task_with_options_dry_run_calls_async_dry_run_not_start`, `run_task_with_options_dry_run_calls_blocking_dry_run_not_start`, and `run_task_with_options_dry_run_uses_existing_retry_policy`; `genja-core/src/task.rs`: `dry_run_does_not_invoke_idempotency_check` and `session_verification_does_not_replace_during_dry_run`. |
| Replacement versus task attempts | `genja-core/src/task.rs`: `session_verification_succeeds_after_configured_retry_without_rerunning_task`, `session_verification_exhaustion_returns_connection_failure_without_rerunning_task`, and the validation tests for zero attempts and missing connection. `genja/src/lib.rs`: `run_verifies_replacement_sessions_across_serial_and_threaded_runners`. |
| Rust execution and runner ordering | `genja/src/lib.rs`: `run_preserves_retry_behavior_across_serial_and_threaded_runners`, `run_task_async_executes_task_inside_existing_tokio_runtime`, `run_task_async_offloads_blocking_task_work`, `run_tasks_accepts_ordered_task_list_with_nested_subtasks`, and `run_tasks_async_preserves_root_task_order`. |
| Python bridge and results | `genja-core-python/python/tests/test_async_tasks.py`: async task, subtask, dry-run, convergence, verification, and task-list tests; `test_task_definition.py`: retry delay and direct execution; `test_runtime.py`: result-shape conversion; `test_plugin_manager.py`: Python runner callback. Python `asyncio.wait_for` tests only caller-side timeout, not Genja's proposed structured task deadline. |

These tests leave three design-specific gaps: no shared selected algorithm
across independently configured task-host nodes; no per-task-host deadline
and interruption report; and no migration test for the changed Rust and
Python runner signatures. The follow-ups below close those gaps.

### Required retry propagation case

Run one ordered list on two hosts with both the serial and threaded runners.
Set runner retry defaults to `allow = true`, `max_attempts = 3`, and a known
delay. Select one custom retry algorithm for the whole invocation; have it
return an immediate retry when the runtime safety gates permit one. Use
host-specific scripted task results so the assertions are independent of
runner scheduling:

| Task node | Host 1 | Host 2 | Expected settings and result |
| --- | --- | --- | --- |
| Root A, first in list | Fails retryably twice, then passes | Fails retryably once, then passes | Inherits runner limit 3; `attempts` are 3 and 2 respectively. |
| A's subtask | Fails retryably once, then passes | Fails retryably once, then passes | Authored `max_attempts = 2`; each host reports 2 attempts, independent of A. |
| Root B, second in list | Fails retryably | Fails retryably | Authored `allow = false`; each host reports 1 attempt despite runner defaults and the selected algorithm. |

Assert that the custom policy observes root A and subtask decisions with
their resolved limits, and is not asked to retry B. Record policy calls for
observation only; do not implement a shared budget in the test policy.
Assert the result vector remains `[A, B]`, A's subtask is nested under A,
retry flags match each host's own attempts, and host 2's attempts are not
reduced by host 1. Repeat without selecting a new algorithm and compare
decisions, configured delays, metadata, and serialized field sets to the
existing fixed-delay baseline; compare timestamps by presence and type,
not by their run-specific values. Include a root `TaskDefinition` retry override and a
`TaskSpec`-constructed definition in separate focused cases to verify their
current precedence and the new single-definition helper.

### Deadline, verification, and compatibility cases

| Boundary | Follow-up assertion |
| --- | --- |
| Task-host clock | A queued threaded host gets its full timeout when scheduled; a later root and each subtask get fresh clocks. Connection setup, retry waits, and replacement waits spend the same task-host budget; no retry resets it. Use controlled barriers or a test clock, not tight wall-time thresholds. |
| Attempt domains | Expiry before the first task call reports zero task attempts. Initial connection failures do not enter the task retry loop. Replacement retries retain their own count and never rerun the applied task. |
| Rust async and blocking | Async work that acknowledges cancellation may report confirmed local termination. A still-running `spawn_blocking` closure reports cancellation requested with termination unconfirmed; it cannot start another task attempt or an unsafe subtask while work remains active. |
| Python async and blocking | Test both `start_async` through the asyncio bridge and blocking `start` through the Rust adapter. A timed-out Python coroutine or blocking call is not reported as stopped until Genja observes completion or acknowledgement. Direct Python `TaskDefinition.run_on_host(s)` uses the same policy semantics. |
| Verification | A required post-check or fresh session fails validation before applying a task that lacks the needed hook, connection, or configuration. With both requirements available, preserve the existing replacement-session post-check order. Reject dry-run plus a verification requirement before execution. |
| Outcome and serialization | Preserve `PassedWithWarnings`, retry metadata, processor ordering, and exact raw/human JSON for existing no-policy calls. When a deadline applies, omit interruption metadata if none occurred; otherwise distinguish trigger, cancellation request, local termination, and unknown external state without changing the outer `TaskResults` type. |
| Runner migration | A Rust runner updated only to accept the new argument still runs default-policy calls; a selected policy is rejected before any task if its capability method remains false. Check the same boundary through the Python runner adapter and callback. An opted-in runner must pass policy to the core host entry point. |

Implement these cases in the corresponding retry, deadline, cancellation,
verification, and Python-binding follow-ups rather than adding tests for
unimplemented APIs in this design issue.
