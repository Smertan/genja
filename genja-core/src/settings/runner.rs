use super::env_defaults::{
    get_runner_max_connection_attempts_default, get_runner_max_task_depth_default,
    get_runner_options_default, get_runner_plugin_default,
};
use serde::{Deserialize, Serialize};

/// Configuration for the task runner plugin system.
///
/// This struct defines how tasks should be executed in Genja, specifying which
/// runner plugin to use and its configuration options. The runner plugin controls
/// the execution strategy (e.g., serial or threaded) and behavior for running
/// tasks across hosts.
///
/// # Fields
///
/// * `plugin` - The name of the runner plugin to use for task execution.
///   Defaults to the value from the `GENJA_RUNNER_PLUGIN` environment variable,
///   or "threaded" if not set. Common values include "threaded" for concurrent
///   execution or "serial" for one-at-a-time execution.
/// * `options` - A JSON object containing plugin-specific configuration options.
///   The structure and available options depend on the selected plugin. Defaults
///   to an empty object.
/// * `worker_count` - Optional worker count for runner implementations that support
///   a fixed concurrency setting. For the built-in `"threaded"` runner, this is the
///   canonical way to control the maximum number of concurrent host executions.
/// * `max_task_depth` - Maximum recursion depth for task/sub-task execution.
///   Defaults to `10`.
/// * `max_connection_attempts` - Maximum number of connection attempts before retries
///   should stop and the connection should be treated as failed. Defaults to `3`.
///
/// # Deserialization
///
/// - Missing fields use their default values (see `Default` impl)
/// - The `plugin` field defaults to `GENJA_RUNNER_PLUGIN` env var or "threaded"
/// - The `options` field defaults to `{}`
/// - The `worker_count` field defaults to `None`
/// - Invalid field values cause deserialization to fail
///
/// # Examples
///
/// ```
/// use genja_core::settings::RunnerConfig;
///
/// // Create with default values
/// let config = RunnerConfig::default();
///
/// // Create with custom configuration
/// let config = RunnerConfig::builder()
///     .plugin("threaded")
///     .worker_count(5)
///     .build();
///
/// println!("Using runner plugin: {}", config.plugin());
/// ```
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

/// Builder for constructing `RunnerConfig` instances with custom settings.
///
/// This builder provides a fluent interface for creating `RunnerConfig` objects,
/// allowing selective configuration of task runner settings. Fields that are not
/// explicitly set will use their default values when `build()` is called.
///
/// # Fields
///
/// * `plugin` - Optional name of the runner plugin to use for task execution. When set to
///   `Some(name)`, the specified plugin will be used. If `None`, the default value from
///   the `GENJA_RUNNER_PLUGIN` environment variable or "threaded" will be used.
/// * `options` - Optional JSON object containing plugin-specific configuration options.
///   When set to `Some(value)`, the specified options will be used. If `None`, the default
///   value of `{}` will be used. The structure and available options depend on the
///   selected plugin.
/// * `worker_count` - Optional worker count for runner implementations that support a
///   fixed concurrency setting. If `None`, the runner decides an appropriate default.
/// * `max_task_depth` - Optional maximum recursion depth for task/sub-task execution. When set to
///   `Some(value)`, the specified depth will be used. If `None`, defaults to `10`.
/// * `max_connection_attempts` - Optional maximum number of connection attempts before retries
///   should stop. When set to `Some(value)`, the specified limit will be used. If `None`,
///   defaults to `3`.
///
/// # Examples
///
/// ```
/// use genja_core::settings::RunnerConfig;
///
/// // Build with custom plugin and options
/// let config = RunnerConfig::builder()
///     .plugin("threaded")
///     .worker_count(5)
///     .build();
///
/// // Build with defaults
/// let config = RunnerConfig::builder().build();
/// ```
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

impl Default for RunnerConfigBuilder {
    fn default() -> Self {
        Self {
            plugin: None,
            options: None,
            worker_count: None,
            max_task_depth: None,
            max_connection_attempts: None,
        }
    }
}
