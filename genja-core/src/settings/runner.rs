use super::env_defaults::{
    get_runner_max_connection_attempts_default, get_runner_max_task_depth_default,
    get_runner_options_default, get_runner_plugin_default, get_runner_retry_allow_default,
    get_runner_retry_delay_ms_default, get_runner_retry_max_attempts_default,
};
use crate::task::TaskRetryConfig;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

/// Task runner configuration.
///
/// The plugin name defaults from `GENJA_RUNNER_PLUGIN`. `worker_count`,
/// `max_task_depth`, and `max_connection_attempts` control built-in runner
/// behavior. `task_retry` carries runner-level task retry defaults and `options`
/// carries plugin-specific JSON for custom runners.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct RunnerConfig {
    plugin: String,
    options: serde_json::Value,
    worker_count: Option<usize>,
    max_task_depth: usize,
    max_connection_attempts: usize,
    #[serde(
        default = "default_retry_config",
        deserialize_with = "deserialize_retry_config"
    )]
    task_retry: TaskRetryConfig,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            plugin: get_runner_plugin_default(),
            options: get_runner_options_default(),
            worker_count: None,
            max_task_depth: get_runner_max_task_depth_default(),
            max_connection_attempts: get_runner_max_connection_attempts_default(),
            task_retry: default_retry_config(),
        }
    }
}

impl RunnerConfig {
    pub fn builder() -> RunnerConfigBuilder {
        RunnerConfigBuilder::default()
    }

    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    pub fn options(&self) -> &serde_json::Value {
        &self.options
    }

    pub fn worker_count(&self) -> Option<usize> {
        self.worker_count
    }

    pub fn max_task_depth(&self) -> usize {
        self.max_task_depth
    }

    pub fn max_connection_attempts(&self) -> usize {
        self.max_connection_attempts
    }

    /// Return normalized default task retry settings for this runner.
    pub fn task_retry(&self) -> &TaskRetryConfig {
        &self.task_retry
    }

    pub(crate) fn retry_allow(&self) -> bool {
        self.task_retry
            .allow()
            .unwrap_or_else(get_runner_retry_allow_default)
    }

    pub(crate) fn retry_max_attempts(&self) -> usize {
        self.task_retry
            .max_attempts()
            .unwrap_or_else(get_runner_retry_max_attempts_default)
    }

    pub(crate) fn retry_delay_ms(&self) -> u64 {
        self.task_retry
            .delay_ms()
            .unwrap_or_else(get_runner_retry_delay_ms_default)
    }
}

/// Builder for `RunnerConfig`.
#[derive(Default)]
pub struct RunnerConfigBuilder {
    plugin: Option<String>,
    options: Option<serde_json::Value>,
    worker_count: Option<usize>,
    max_task_depth: Option<usize>,
    max_connection_attempts: Option<usize>,
    task_retry: Option<TaskRetryConfig>,
}

impl RunnerConfigBuilder {
    pub fn plugin(mut self, plugin: impl Into<String>) -> Self {
        self.plugin = Some(plugin.into());
        self
    }

    pub fn options(mut self, options: serde_json::Value) -> Self {
        self.options = Some(options);
        self
    }

    pub fn worker_count(mut self, worker_count: usize) -> Self {
        self.worker_count = Some(worker_count);
        self
    }

    pub fn max_task_depth(mut self, max_task_depth: usize) -> Self {
        self.max_task_depth = Some(max_task_depth);
        self
    }

    pub fn max_connection_attempts(mut self, max_connection_attempts: usize) -> Self {
        self.max_connection_attempts = Some(max_connection_attempts);
        self
    }

    /// Set default task retry settings for this runner.
    pub fn task_retry(mut self, task_retry: TaskRetryConfig) -> Self {
        self.task_retry = Some(normalize_retry_config(task_retry));
        self
    }

    pub fn build(self) -> RunnerConfig {
        RunnerConfig {
            plugin: self.plugin.unwrap_or_else(get_runner_plugin_default),
            options: self.options.unwrap_or_else(get_runner_options_default),
            worker_count: self.worker_count,
            max_task_depth: self
                .max_task_depth
                .unwrap_or_else(get_runner_max_task_depth_default),
            max_connection_attempts: self
                .max_connection_attempts
                .unwrap_or_else(get_runner_max_connection_attempts_default),
            task_retry: self.task_retry.unwrap_or_else(default_retry_config),
        }
    }
}

fn default_retry_config() -> TaskRetryConfig {
    TaskRetryConfig::builder()
        .allow(get_runner_retry_allow_default())
        .max_attempts(get_runner_retry_max_attempts_default())
        .delay_ms(get_runner_retry_delay_ms_default())
        .build()
}

fn normalize_retry_config(retry: TaskRetryConfig) -> TaskRetryConfig {
    let mut builder = TaskRetryConfig::builder()
        .allow(retry.allow().unwrap_or_else(get_runner_retry_allow_default))
        .max_attempts(
            retry
                .max_attempts()
                .unwrap_or_else(get_runner_retry_max_attempts_default),
        )
        .delay_ms(
            retry
                .delay_ms()
                .unwrap_or_else(get_runner_retry_delay_ms_default),
        );
    if let Some(policy) = retry.policy() {
        builder = builder.policy(policy);
    }
    if let Some(factor) = retry.factor() {
        builder = builder.factor(factor);
    }
    if let Some(max_delay_ms) = retry.max_delay_ms() {
        builder = builder.max_delay_ms(max_delay_ms);
    }
    builder.build()
}

fn deserialize_retry_config<'de, D>(deserializer: D) -> Result<TaskRetryConfig, D::Error>
where
    D: Deserializer<'de>,
{
    TaskRetryConfig::deserialize(deserializer).map(normalize_retry_config)
}
