# Task retry policy contract

This document fixes the public contract for the retry-policy implementation.
It builds on the retry safety and result semantics in the
[execution policy baseline](execution-policy-baseline.md#proposed-retry-contract-phase-2).
The baseline's original one-algorithm-per-invocation proposal is extended here
to permit independent policy selection for each task in an ordered list.

## Policy input and decision

`genja_core::task` exports `TaskRetryPolicy`, `TaskRetryContext`,
`TaskRetrySettings`, and `TaskRetryDecision`. `TaskRetryPolicy::decide` takes a
read-only context and returns `Stop` or `RetryAfter(Duration)`. The context
contains the completed host result, the number of completed attempts starting
at one, resolved task-host retry settings, and the task's idempotency mode.
`TaskRetrySettings` exposes `allow`, total `max_attempts`, and base `delay_ms`.
The policy can inspect the current `TaskFailure`, including `retryable`, but
cannot replace the result or increase the task's attempt limit.

The runtime checks `allow`, remaining attempts, and a failed result marked
`retryable` before asking a policy to decide. It applies the same checks to a
returned `RetryAfter`. Passed, warning, skipped, and non-retryable results
stop without invoking the policy. The runtime owns sleeping, convergence
checks, processor hooks, fresh-session verification, and final result
recording. Connection retry remains a separate mechanism.

An individual policy may be shared safely with `Arc<dyn TaskRetryPolicy>`
across the hosts that execute its task tree. Different roots in one ordered
list may hold different policies. Settings and attempt counters are resolved
separately for each task on each host. A policy implementation must not use
shared mutable state as a cross-task or cross-host attempt budget.

## Selection and precedence

Built-in selection is optional in Rust task `retry(...)` metadata, Python
`TaskRetryConfig`, `runner.task_retry`, and applicable `TaskSpec` root
overrides. When no source selects a policy, fixed delay applies, including
for existing task definitions and run calls.

Resolve the algorithm separately for each task node. For a root task, use the
first available selection in this order:

1. A custom Rust policy attached directly to that root `TaskDefinition`.
2. If a root `TaskDefinition` retry override exists, including
   `TaskSpecOverrides.retry`, use its built-in policy selection when present;
   otherwise continue at step 4. The override replaces authored retry
   metadata for that root.
3. If there is no root override, use the built-in policy selected by the
   task's own Rust or Python retry metadata when present.
4. An optional Rust invocation policy, used as a fallback for roots without
   their own selection.
5. A built-in policy selected by `runner.task_retry`.
6. The built-in fixed-delay policy.

Each subtask uses its own explicit policy selection when present; otherwise it
inherits its parent's selected algorithm. An invocation policy therefore
reaches roots and subtasks that have no more specific selection. Neither an
attached custom policy nor an inherited algorithm replaces a task's `allow`,
`max_attempts`, or `delay_ms`. These fields continue to resolve independently,
with task metadata overriding runner defaults field by field. A root retry
override replaces the root's authored retry metadata; its omitted fields
inherit runner defaults. Subtasks use their own retry metadata.

The additive Rust API will let a caller attach a custom policy to an
individual `TaskDefinition` and place differently configured definitions in
one `Tasks` list. A convenience single-task entry point may accept a policy
directly, with synchronous and asynchronous forms. Existing run methods keep
their signatures. Policy selection must reach both built-in runners; a runner
that cannot carry a selected policy must report that explicitly rather than
silently use another algorithm.

## Built-in policies and exponential parameters

`genja_core::task` exports all three built-in policy types. Rust callers can
construct `ExponentialRetry::new(factor, max_delay_ms)` or use its default
factor of two and 24-hour cap. Invalid parameters return
`ExponentialRetryError`. The existing runtime path currently uses `FixedRetry`;
configuration selection and direct policy attachment are later phases.

- `NoRetry` always returns `Stop`. Selecting it does not change `allow`.
- `FixedRetry` returns `RetryAfter(delay_ms)` for an eligible failure and is
  the compatibility default.
- `ExponentialRetry` uses `delay_ms` as its base delay. Its integer multiplier
  defaults to `2` and may be set to any integer of at least `2`. Its maximum
  delay defaults to `86_400_000` ms (24 hours) and may be set to a positive
  lower value. The maximum is a per-wait cap, not a total retry deadline.

After completed attempt `n >= 1`, exponential retry waits
`min(max_delay_ms, delay_ms * factor^(n - 1))` milliseconds before attempt
`n + 1`. It computes the capped result with checked or saturating arithmetic,
so large values never wrap. With `delay_ms = 1000`, factor `3`, and
`max_attempts = 6`, the waits are 1, 3, 9, 27, and 81 seconds. A zero base
delay stays zero. No policy sleeps after the final allowed attempt.

The multiplier and cap belong to the selected exponential policy. A task
selection can supply its own values; an invocation policy supplies default
algorithm parameters only to tasks without a more specific selection. Each
task retains its own base delay. Configuration must reject a multiplier below
`2`, a zero or above-24-hour cap, and exponential-only parameters attached to
another policy. These additions must preserve existing
`TaskRetryConfig::new` and Python constructor calls that omit policy selection.

## Result compatibility

`execution_metadata.attempts` counts completed task attempts, including a
convergence pre-check. `retried` means more than one attempt occurred.
`retry_exhausted` retains its current meaning: the final failure is
retryable, retries were enabled, and the configured maximum above one was
reached. A policy that stops early does not mark the limit exhausted.
Convergence after an earlier failure still produces `PassedWithWarnings`.
The existing host result and Python result shapes remain unchanged.
