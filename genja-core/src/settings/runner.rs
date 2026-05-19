use super::env_defaults::{
    get_runner_max_connection_attempts_default, get_runner_max_task_depth_default,
    get_runner_options_default, get_runner_plugin_default,
};
use serde::{Deserialize, Serialize};

/// Task runner configuration.
///
/// The plugin name defaults from `GENJA_RUNNER_PLUGIN`. `worker_count`,
/// `max_task_depth`, and `max_connection_attempts` control built-in runner
/// behavior; `options` carries plugin-specific JSON for custom runners.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct RunnerConfig {
    plugin: String,
    options: serde_json::Value,
    worker_count: Option<usize>,
    max_task_depth: usize,
    max_connection_attempts: usize,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            plugin: get_runner_plugin_default(),
            options: get_runner_options_default(),
            worker_count: None,
            max_task_depth: get_runner_max_task_depth_default(),
            max_connection_attempts: get_runner_max_connection_attempts_default(),
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
}

/// Builder for `RunnerConfig`.
#[derive(Default)]
pub struct RunnerConfigBuilder {
    plugin: Option<String>,
    options: Option<serde_json::Value>,
    worker_count: Option<usize>,
    max_task_depth: Option<usize>,
    max_connection_attempts: Option<usize>,
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
        }
    }
}
