//! Task retry policy contract and internal resolution of retry settings.

use super::{HostTaskResult, IdempotencyMode, TaskRetryConfig, TaskRetryPolicyKind};
use crate::settings::RunnerConfig;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
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
    pub(super) fn new(
        result: &'a HostTaskResult,
        completed_attempts: usize,
        settings: &'a TaskRetrySettings,
        idempotency_mode: IdempotencyMode,
    ) -> Self {
        Self {
            result,
            completed_attempts,
            settings,
            idempotency_mode,
        }
    }

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

/// A policy that never schedules another task attempt.
///
/// Selecting this policy does not change the task's retry authorization or
/// attempt limit; it simply returns [`TaskRetryDecision::Stop`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NoRetry;

impl TaskRetryPolicy for NoRetry {
    fn decide(&self, _context: &TaskRetryContext<'_>) -> TaskRetryDecision {
        TaskRetryDecision::Stop
    }
}

/// Retry after the task's resolved `delay_ms` for each eligible failure.
///
/// This is the default algorithm when no policy is selected. A zero delay
/// schedules the next attempt without waiting, matching existing behavior.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FixedRetry;

impl TaskRetryPolicy for FixedRetry {
    fn decide(&self, context: &TaskRetryContext<'_>) -> TaskRetryDecision {
        if context
            .settings
            .can_retry(context.result, context.completed_attempts)
        {
            TaskRetryDecision::RetryAfter(Duration::from_millis(context.settings.delay_ms))
        } else {
            TaskRetryDecision::Stop
        }
    }
}

/// Invalid parameters for [`ExponentialRetry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExponentialRetryError {
    /// The multiplier must be at least two.
    InvalidFactor,
    /// The maximum delay must be between one millisecond and 24 hours.
    InvalidMaxDelay,
}

impl fmt::Display for ExponentialRetryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFactor => formatter.write_str("retry factor must be at least 2"),
            Self::InvalidMaxDelay => {
                formatter.write_str("maximum retry delay must be between 1 and 86400000 ms")
            }
        }
    }
}

impl Error for ExponentialRetryError {}

/// Retry with a bounded exponential delay based on the task's `delay_ms`.
///
/// After completed attempt `n >= 1`, the delay before attempt `n + 1` is
/// `min(max_delay_ms, delay_ms * factor^(n - 1))`. The first retry therefore
/// uses the base delay. Arithmetic saturates before the cap, so it cannot
/// wrap. A zero base delay remains zero. The cap applies to each wait, not to
/// the total execution time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExponentialRetry {
    factor: u32,
    max_delay_ms: u64,
}

impl ExponentialRetry {
    /// Largest supported delay cap: 24 hours in milliseconds.
    pub const MAX_DELAY_MS: u64 = 86_400_000;

    /// Create an exponential policy with a factor of at least two and a
    /// positive delay cap of at most 24 hours.
    pub fn new(factor: u32, max_delay_ms: u64) -> Result<Self, ExponentialRetryError> {
        if factor < 2 {
            return Err(ExponentialRetryError::InvalidFactor);
        }
        if max_delay_ms == 0 || max_delay_ms > Self::MAX_DELAY_MS {
            return Err(ExponentialRetryError::InvalidMaxDelay);
        }
        Ok(Self {
            factor,
            max_delay_ms,
        })
    }

    /// Return the multiplier applied between retry waits.
    pub fn factor(&self) -> u32 {
        self.factor
    }

    /// Return the maximum delay for one retry wait, in milliseconds.
    pub fn max_delay_ms(&self) -> u64 {
        self.max_delay_ms
    }

    fn delay_ms(&self, base_delay_ms: u64, completed_attempts: usize) -> u64 {
        let mut delay = base_delay_ms.min(self.max_delay_ms);
        for _ in 1..completed_attempts {
            if delay == 0 || delay == self.max_delay_ms {
                break;
            }
            delay = delay
                .saturating_mul(u64::from(self.factor))
                .min(self.max_delay_ms);
        }
        delay
    }
}

impl Default for ExponentialRetry {
    fn default() -> Self {
        Self {
            factor: 2,
            max_delay_ms: Self::MAX_DELAY_MS,
        }
    }
}

impl TaskRetryPolicy for ExponentialRetry {
    fn decide(&self, context: &TaskRetryContext<'_>) -> TaskRetryDecision {
        if context
            .settings
            .can_retry(context.result, context.completed_attempts)
        {
            TaskRetryDecision::RetryAfter(Duration::from_millis(
                self.delay_ms(context.settings.delay_ms, context.completed_attempts),
            ))
        } else {
            TaskRetryDecision::Stop
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct RetryDefaults {
    allow: bool,
    max_attempts: usize,
    delay_ms: u64,
    policy: ResolvedRetryPolicy,
}

impl Default for RetryDefaults {
    fn default() -> Self {
        Self {
            allow: false,
            max_attempts: 1,
            delay_ms: 0,
            policy: ResolvedRetryPolicy::Fixed,
        }
    }
}

impl From<&RunnerConfig> for RetryDefaults {
    fn from(runner_config: &RunnerConfig) -> Self {
        Self {
            allow: runner_config.retry_allow(),
            max_attempts: runner_config.retry_max_attempts(),
            delay_ms: runner_config.retry_delay_ms(),
            policy: ResolvedRetryPolicy::from_config(
                runner_config.task_retry(),
                ResolvedRetryPolicy::Fixed,
            ),
        }
    }
}

impl RetryDefaults {
    pub(super) fn with_policy(mut self, policy: ResolvedRetryPolicy) -> Self {
        self.policy = policy;
        self
    }
}

/// Algorithm resolved for a task node; the parameters travel with it.
#[derive(Clone)]
pub(super) enum ResolvedRetryPolicy {
    NoRetry,
    Fixed,
    Exponential(ExponentialRetry),
    Custom(Arc<dyn TaskRetryPolicy>),
}

impl fmt::Debug for ResolvedRetryPolicy {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoRetry => formatter.write_str("NoRetry"),
            Self::Fixed => formatter.write_str("Fixed"),
            Self::Exponential(policy) => {
                formatter.debug_tuple("Exponential").field(policy).finish()
            }
            Self::Custom(_) => formatter.write_str("Custom"),
        }
    }
}

impl ResolvedRetryPolicy {
    fn from_config(config: &TaskRetryConfig, fallback: Self) -> Self {
        match config.policy() {
            Some(TaskRetryPolicyKind::NoRetry) => Self::NoRetry,
            Some(TaskRetryPolicyKind::Fixed) => Self::Fixed,
            Some(TaskRetryPolicyKind::Exponential) => Self::Exponential(
                ExponentialRetry::new(
                    config.factor().unwrap_or(2),
                    config
                        .max_delay_ms()
                        .unwrap_or(ExponentialRetry::MAX_DELAY_MS),
                )
                .expect("retry policy parameters are validated during configuration"),
            ),
            None => fallback,
        }
    }

    pub(super) fn for_config(config: Option<&TaskRetryConfig>, defaults: &RetryDefaults) -> Self {
        config.map_or_else(
            || defaults.policy.clone(),
            |config| Self::from_config(config, defaults.policy.clone()),
        )
    }

    pub(super) fn as_policy(&self) -> &dyn TaskRetryPolicy {
        match self {
            Self::NoRetry => &NoRetry,
            Self::Fixed => &FixedRetry,
            Self::Exponential(policy) => policy,
            Self::Custom(policy) => policy.as_ref(),
        }
    }
}

pub(super) type RetrySettings = TaskRetrySettings;

impl TaskRetrySettings {
    pub(super) fn for_config(config: Option<&TaskRetryConfig>, defaults: &RetryDefaults) -> Self {
        let retry_config = config.copied().unwrap_or_default();
        Self {
            allow: retry_config.resolved_allow(defaults.allow),
            max_attempts: retry_config.resolved_max_attempts(defaults.max_attempts),
            delay_ms: retry_config.resolved_delay_ms(defaults.delay_ms),
        }
    }

    fn can_retry(&self, result: &HostTaskResult, attempts: usize) -> bool {
        self.allow
            && attempts < self.max_attempts
            && matches!(result.failure(), Some(failure) if failure.retryable())
    }

    pub(super) fn decide(
        &self,
        policy: &dyn TaskRetryPolicy,
        result: &HostTaskResult,
        attempts: usize,
        idempotency_mode: IdempotencyMode,
    ) -> TaskRetryDecision {
        if !self.can_retry(result, attempts) {
            return TaskRetryDecision::Stop;
        }
        let context = TaskRetryContext::new(result, attempts, self, idempotency_mode);
        match policy.decide(&context) {
            TaskRetryDecision::RetryAfter(delay) if self.can_retry(result, attempts) => {
                TaskRetryDecision::RetryAfter(delay)
            }
            _ => TaskRetryDecision::Stop,
        }
    }

    pub(super) fn retry_exhausted(self, result: &HostTaskResult, attempts: usize) -> bool {
        self.allow
            && attempts >= self.max_attempts
            && self.max_attempts > 1
            && matches!(result.failure(), Some(failure) if failure.retryable())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::task::{TaskFailure, TaskSuccess};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn failed(retryable: bool) -> HostTaskResult {
        HostTaskResult::failed(
            TaskFailure::new(std::io::Error::other("failed")).with_retryable(retryable),
        )
    }

    fn settings() -> TaskRetrySettings {
        TaskRetrySettings {
            allow: true,
            max_attempts: 6,
            delay_ms: 1_000,
        }
    }

    #[test]
    fn built_in_policies_choose_expected_delays() {
        let result = failed(true);
        let settings = settings();
        let exponential = ExponentialRetry::new(3, ExponentialRetry::MAX_DELAY_MS).unwrap();

        assert_eq!(
            settings.decide(&NoRetry, &result, 1, IdempotencyMode::Disabled),
            TaskRetryDecision::Stop
        );
        for (attempt, seconds) in [(1, 1), (2, 3), (3, 9), (4, 27), (5, 81)] {
            assert_eq!(
                settings.decide(&FixedRetry, &result, attempt, IdempotencyMode::Disabled),
                TaskRetryDecision::RetryAfter(Duration::from_secs(1))
            );
            assert_eq!(
                settings.decide(&exponential, &result, attempt, IdempotencyMode::Disabled),
                TaskRetryDecision::RetryAfter(Duration::from_secs(seconds))
            );
        }
        assert_eq!(
            settings.decide(&exponential, &result, 6, IdempotencyMode::Disabled),
            TaskRetryDecision::Stop
        );
    }

    #[test]
    fn exponential_delay_is_capped_and_overflow_safe() {
        let policy = ExponentialRetry::new(u32::MAX, 5_000).unwrap();
        assert_eq!(policy.delay_ms(1, 1), 1);
        assert_eq!(policy.delay_ms(1, 2), 5_000);
        assert_eq!(policy.delay_ms(u64::MAX, usize::MAX), 5_000);
        assert_eq!(policy.delay_ms(0, usize::MAX), 0);
        assert_eq!(ExponentialRetry::default().factor(), 2);
        assert_eq!(
            ExponentialRetry::default().max_delay_ms(),
            ExponentialRetry::MAX_DELAY_MS
        );
    }

    #[test]
    fn exponential_parameters_are_validated() {
        assert_eq!(
            ExponentialRetry::new(1, 1),
            Err(ExponentialRetryError::InvalidFactor)
        );
        assert_eq!(
            ExponentialRetry::new(2, 0),
            Err(ExponentialRetryError::InvalidMaxDelay)
        );
        assert_eq!(
            ExponentialRetry::new(2, ExponentialRetry::MAX_DELAY_MS + 1),
            Err(ExponentialRetryError::InvalidMaxDelay)
        );
    }

    #[test]
    fn task_policy_selection_overrides_or_inherits_the_resolved_algorithm() {
        let parent_config = TaskRetryConfig::builder()
            .policy(TaskRetryPolicyKind::Exponential)
            .factor(3)
            .max_delay_ms(60_000)
            .build();
        let selected =
            ResolvedRetryPolicy::from_config(&parent_config, ResolvedRetryPolicy::NoRetry);
        assert!(matches!(
            &selected,
            ResolvedRetryPolicy::Exponential(policy)
                if policy.factor() == 3 && policy.max_delay_ms() == 60_000
        ));

        let child_without_selection = TaskRetryConfig::builder().max_attempts(2).build();
        let inherited =
            ResolvedRetryPolicy::from_config(&child_without_selection, selected.clone());
        assert!(matches!(inherited, ResolvedRetryPolicy::Exponential(_)));

        let child_with_selection = TaskRetryConfig::builder()
            .policy(TaskRetryPolicyKind::Fixed)
            .build();
        let overridden = ResolvedRetryPolicy::from_config(&child_with_selection, selected);
        assert!(matches!(overridden, ResolvedRetryPolicy::Fixed));
    }

    struct AlwaysRetry(AtomicUsize);

    impl TaskRetryPolicy for AlwaysRetry {
        fn decide(&self, context: &TaskRetryContext<'_>) -> TaskRetryDecision {
            assert_eq!(context.idempotency_mode(), IdempotencyMode::Check);
            assert_eq!(context.settings().delay_ms(), 1_000);
            self.0.fetch_add(1, Ordering::SeqCst);
            TaskRetryDecision::RetryAfter(Duration::ZERO)
        }
    }

    #[test]
    fn runtime_gate_rejects_ineligible_results_before_custom_policy() {
        let policy = AlwaysRetry(AtomicUsize::new(0));
        let settings = settings();
        for result in [
            failed(false),
            HostTaskResult::skipped(),
            HostTaskResult::passed(TaskSuccess::new()),
        ] {
            assert_eq!(
                settings.decide(&policy, &result, 1, IdempotencyMode::Check),
                TaskRetryDecision::Stop
            );
        }
        assert_eq!(
            settings.decide(&policy, &failed(true), 6, IdempotencyMode::Check),
            TaskRetryDecision::Stop
        );
        let disabled = TaskRetrySettings {
            allow: false,
            ..settings
        };
        assert_eq!(
            disabled.decide(&policy, &failed(true), 1, IdempotencyMode::Check),
            TaskRetryDecision::Stop
        );
        assert_eq!(policy.0.load(Ordering::SeqCst), 0);
        assert_eq!(
            settings.decide(&policy, &failed(true), 1, IdempotencyMode::Check),
            TaskRetryDecision::RetryAfter(Duration::ZERO)
        );
        assert_eq!(policy.0.load(Ordering::SeqCst), 1);
    }
}
