//! Internal resolution of task retry settings.

use super::Task;
use crate::settings::RunnerConfig;

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

    pub(super) fn allow(self) -> bool {
        self.allow
    }

    pub(super) fn max_attempts(self) -> usize {
        self.max_attempts
    }

    pub(super) fn delay_ms(self) -> u64 {
        self.delay_ms
    }
}
