//! Task retry policy contract and internal resolution of retry settings.

use super::{HostTaskResult, IdempotencyMode, Task};
use crate::settings::RunnerConfig;
use std::time::Duration;

/// Resolved retry settings for one task on one host.
///
/// These values are independent of the selected retry algorithm. The runtime
/// merges runner defaults with the task's retry metadata before each task-host
/// execution, then keeps the resulting values and attempt count local to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaskRetrySettings {
    allow: bool,
    max_attempts: usize,
    delay_ms: u64,
}

impl TaskRetrySettings {
    /// Return whether this task is authorized to retry a retryable failure.
    pub fn allow(&self) -> bool {
        self.allow
    }

    /// Return the maximum total task attempts, including the initial attempt.
    pub fn max_attempts(&self) -> usize {
        self.max_attempts
    }

    /// Return the task's base delay in milliseconds.
    ///
    /// Fixed retry uses this delay directly; exponential retry scales it.
    pub fn delay_ms(&self) -> u64 {
        self.delay_ms
    }
}

/// Read-only input to a task retry policy after a completed task attempt.
///
/// The result includes any immediate convergence post-check. A policy can
/// inspect [`HostTaskResult::failure`] and the failure's retryability, but it
/// cannot change the result or the task's attempt limit.
#[derive(Debug)]
pub struct TaskRetryContext<'a> {
    result: &'a HostTaskResult,
    completed_attempts: usize,
    settings: &'a TaskRetrySettings,
    idempotency_mode: IdempotencyMode,
}

impl<'a> TaskRetryContext<'a> {
    /// Return the host result from the completed attempt.
    pub fn result(&self) -> &'a HostTaskResult {
        self.result
    }

    /// Return the number of completed task attempts, starting at one.
    pub fn completed_attempts(&self) -> usize {
        self.completed_attempts
    }

    /// Return the settings resolved for this task and host.
    pub fn settings(&self) -> &'a TaskRetrySettings {
        self.settings
    }

    /// Return the task's idempotency mode.
    pub fn idempotency_mode(&self) -> IdempotencyMode {
        self.idempotency_mode
    }
}

/// Whether to stop or schedule another task attempt.
///
/// A retry decision never overrides the runtime's authorization, attempt
/// limit, or retryable-failure checks. The runtime performs the asynchronous
/// wait and owns final result recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskRetryDecision {
    /// Keep the completed attempt as the final result.
    Stop,
    /// Wait for the given duration before the next task attempt.
    RetryAfter(Duration),
}

/// Select the delay for another attempt after a retryable task failure.
///
/// Implementations must be safe to share across hosts and task trees. The
/// runtime keeps attempt counters per task and host; policies must use the
/// supplied context rather than a shared attempt budget. The runtime calls
/// this method only after its retry safety checks pass.
pub trait TaskRetryPolicy: Send + Sync {
    /// Decide whether to stop or when to retry this task-host execution.
    fn decide(&self, context: &TaskRetryContext<'_>) -> TaskRetryDecision;
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RetryDefaults {
    allow: bool,
    max_attempts: usize,
    delay_ms: u64,
}

impl Default for RetryDefaults {
    fn default() -> Self {
        Self {
            allow: false,
            max_attempts: 1,
            delay_ms: 0,
        }
    }
}

impl From<&RunnerConfig> for RetryDefaults {
    fn from(runner_config: &RunnerConfig) -> Self {
        Self {
            allow: runner_config.retry_allow(),
            max_attempts: runner_config.retry_max_attempts(),
            delay_ms: runner_config.retry_delay_ms(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct RetrySettings {
    allow: bool,
    max_attempts: usize,
    delay_ms: u64,
}

impl RetrySettings {
    pub(super) fn for_task(task: &dyn Task, defaults: RetryDefaults) -> Self {
        let retry_config = task.retry_config().copied().unwrap_or_default();
        Self {
            allow: retry_config.resolved_allow(defaults.allow),
            max_attempts: retry_config.resolved_max_attempts(defaults.max_attempts),
            delay_ms: retry_config.resolved_delay_ms(defaults.delay_ms),
        }
    }

    pub(super) fn max_attempts(self) -> usize {
        self.max_attempts
    }

    pub(super) fn delay_ms(self) -> u64 {
        self.delay_ms
    }

    pub(super) fn should_retry(self, result: &HostTaskResult, attempts: usize) -> bool {
        self.allow
            && attempts < self.max_attempts
            && matches!(result.failure(), Some(failure) if failure.retryable())
    }

    pub(super) fn retry_exhausted(self, result: &HostTaskResult, attempts: usize) -> bool {
        self.allow
            && attempts >= self.max_attempts
            && self.max_attempts > 1
            && matches!(result.failure(), Some(failure) if failure.retryable())
    }
}
