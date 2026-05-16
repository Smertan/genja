//! Configuration and settings for Genja Core.
//!
//! This module defines the configuration structs that drive Genja behavior,
//! plus helpers for loading from config files and environment variables.
//!
//! **Key points**
//! - All configs implement `Default` and can be created with `::default()`.
//! - Builders allow partial configuration; missing fields are filled with defaults.
//! - `Settings::from_file` loads JSON or YAML and validates SSH config when present.
//!
//! # Configuration Precedence
//!
//! 1. Configuration files (JSON/YAML) are loaded first
//! 2. Environment variables provide defaults for missing fields
//! 3. Hard-coded defaults are used as final fallback
//!
//! # Environment Variables
//!
//! The following environment variables are supported:
//!
//! - `GENJA_CORE_RAISE_ON_ERROR` - Controls error handling behavior (default: false)
//! - `GENJA_INVENTORY_PLUGIN` - Inventory plugin name (default: "FileInventoryPlugin")
//! - `GENJA_RUNNER_PLUGIN` - Runner plugin name (default: "threaded")
//! - `GENJA_LOGGING_LEVEL` - Log level (default: "info")
//! - `GENJA_LOGGING_LOG_FILE` - Log file path (default: "genja.log")
//! - `GENJA_LOGGING_TO_CONSOLE` - Enable console logging (default: false)
//!
//! # Settings Reference
//!
//! See `docs/settings.md` for a complete schema summary and example config files.
//!
//! # Examples
//!
//! ## Defaults
//! ```
//! use genja_core::Settings;
//!
//! let settings = Settings::default();
//! ```
//!
//! ## Builders
//! ```
//! use genja_core::Settings;
//! use genja_core::settings::{LoggingConfig, RunnerConfig};
//!
//! let settings = Settings::builder()
//!     .logging(LoggingConfig::builder().level("debug").build())
//!     .runner(RunnerConfig::builder().plugin("threaded").build())
//!     .build();
//! ```
//!
//! ## Load From File
//! ```no_run
//! use genja_core::Settings;
//!
//! let settings = Settings::from_file("config.yaml")?;
//! # Ok::<(), genja_core::ConfigLoadError>(())
//! ```
//!
//! ## SSH Validation
//! SSH config is validated automatically when calling `Settings::from_file`.
//! For manual validation, use `SSHConfig::validate`.
use crate::inventory::TransformFunctionOptions;
use crate::ConfigLoadError;
use config::{Config as ConfigBuilder, File, FileFormat};
use serde::{Deserialize, Serialize};
mod env_defaults;
mod inventory_loading;
mod ssh_loading;

#[cfg(test)]
mod tests;

use self::env_defaults::{
    deserialize_bool_loose, get_default_log_file, get_inventory_plugin_config,
    get_log_level_default, get_log_to_console_default,
    get_runner_max_connection_attempts_default, get_runner_max_task_depth_default,
    get_runner_options_default, get_runner_plugin_default, raise_on_error,
};

#[cfg(test)]
use self::env_defaults::{
    parse_bool_loose, ENV_INVENTORY_PLUGIN, ENV_LOG_FILE, ENV_LOG_LEVEL, ENV_LOG_TO_CONSOLE,
    ENV_RAISE_ON_ERROR, ENV_RUNNER_PLUGIN,
};

/// Configuration options for inventory file paths.
///
/// This struct holds optional file paths for the three main inventory components:
/// hosts, groups, and defaults. Each field can be `None` if the corresponding
/// inventory file is not specified or not needed.
///
/// # Fields
///
/// * `hosts_file` - Optional path to the hosts inventory file. This file typically
///   contains the list of hosts that can be managed by Genja.
/// * `groups_file` - Optional path to the groups inventory file. This file typically
///   defines groups of hosts for easier management and organization.
/// * `defaults_file` - Optional path to the defaults inventory file. This file typically
///   contains default configuration values that apply across hosts or groups.
///
/// # Deserialization
///
/// - Missing fields default to `None`
/// - Invalid field values cause deserialization to fail
///
/// # Examples
///
/// ```
/// use genja_core::settings::OptionsConfig;
///
/// // Create with default values (all None)
/// let options = OptionsConfig::default();
///
/// // Create with specific file paths
/// let options = OptionsConfig::builder()
///     .hosts_file("/path/to/hosts.yaml")
///     .groups_file("/path/to/groups.yaml")
///     .defaults_file("/path/to/defaults.yaml")
///     .build();
/// ```
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct OptionsConfig {
    hosts_file: Option<String>,
    groups_file: Option<String>,
    defaults_file: Option<String>,
}

impl Default for OptionsConfig {
    fn default() -> Self {
        OptionsConfig {
            hosts_file: None,
            groups_file: None,
            defaults_file: None,
        }
    }
}

impl OptionsConfig {
    pub fn builder() -> OptionsConfigBuilder {
        OptionsConfigBuilder::default()
    }

    pub fn hosts_file(&self) -> Option<&str> {
        self.hosts_file.as_deref()
    }

    pub fn groups_file(&self) -> Option<&str> {
        self.groups_file.as_deref()
    }

    pub fn defaults_file(&self) -> Option<&str> {
        self.defaults_file.as_deref()
    }
}

/// Builder for constructing `OptionsConfig` instances with custom file paths.
///
/// This builder provides a fluent interface for creating `OptionsConfig` objects,
/// allowing selective configuration of inventory file paths. Fields that are not
/// explicitly set will remain `None` when `build()` is called.
///
/// # Fields
///
/// * `hosts_file` - Optional path to the hosts inventory file. When set to `Some(path)`,
///   the specified file will be used for loading host inventory data. When set to `None`,
///   no hosts file will be configured.
/// * `groups_file` - Optional path to the groups inventory file. When set to `Some(path)`,
///   the specified file will be used for loading group definitions. When set to `None`,
///   no groups file will be configured.
/// * `defaults_file` - Optional path to the defaults inventory file. When set to
///   `Some(path)`, the specified file will be used for loading default configuration
///   values. When set to `None`, no defaults file will be configured.
///
/// # Examples
///
/// ```
/// use genja_core::settings::OptionsConfig;
///
/// // Build with all file paths specified
/// let options = OptionsConfig::builder()
///     .hosts_file("/path/to/hosts.yaml")
///     .groups_file("/path/to/groups.yaml")
///     .defaults_file("/path/to/defaults.yaml")
///     .build();
///
/// // Build with only hosts file
/// let options = OptionsConfig::builder()
///     .hosts_file("/path/to/hosts.yaml")
///     .build();
///
/// // Build with defaults (all None)
/// let options = OptionsConfig::builder().build();
/// ```
pub struct OptionsConfigBuilder {
    hosts_file: Option<String>,
    groups_file: Option<String>,
    defaults_file: Option<String>,
}

impl OptionsConfigBuilder {
    pub fn hosts_file(mut self, path: impl Into<String>) -> Self {
        self.hosts_file = Some(path.into());
        self
    }

    pub fn groups_file(mut self, path: impl Into<String>) -> Self {
        self.groups_file = Some(path.into());
        self
    }

    pub fn defaults_file(mut self, path: impl Into<String>) -> Self {
        self.defaults_file = Some(path.into());
        self
    }

    pub fn build(self) -> OptionsConfig {
        OptionsConfig {
            hosts_file: self.hosts_file,
            groups_file: self.groups_file,
            defaults_file: self.defaults_file,
        }
    }
}

impl Default for OptionsConfigBuilder {
    fn default() -> Self {
        Self {
            hosts_file: None,
            groups_file: None,
            defaults_file: None,
        }
    }
}

/// Configuration for inventory management in Genja.
///
/// This struct defines how inventory data (hosts, groups, and defaults) should be loaded
/// and processed. It specifies the inventory plugin to use, file paths for inventory
/// components, and optional transformation functions to modify the loaded inventory data.
///
/// # Fields
///
/// * `plugin` - The name of the inventory plugin to use for loading inventory data.
///   Defaults to the value from the `GENJA_INVENTORY_PLUGIN` environment variable,
///   or **FileInventoryPlugin** if not set.
/// * `options` - Configuration options specifying the file paths for hosts, groups,
///   and defaults inventory files.
/// * `transform_function` - Optional name of a transformation function to apply to
///   the loaded inventory data. This allows custom processing of inventory before use.
/// * `transform_function_options` - Optional JSON configuration passed to the
///   transformation function, allowing parameterized transformations.
///
/// # Deserialization
///
/// - Missing fields use their default values (see `Default` impl)
/// - The `plugin` field defaults to `GENJA_INVENTORY_PLUGIN` env var or "FileInventoryPlugin"
/// - Invalid field values cause deserialization to fail
///
/// # Examples
///
/// ```
/// use genja_core::settings::InventoryConfig;
///
/// // Create with default values
/// let config = InventoryConfig::default();
///
/// // Load inventory files
/// let (hosts, groups, defaults) = config.load_inventory_files().unwrap();
/// ```
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct InventoryConfig {
    #[serde(default = "get_inventory_plugin_config")]
    plugin: String,
    options: OptionsConfig,
    transform_function: Option<String>,
    transform_function_options: Option<TransformFunctionOptions>,
}

impl Default for InventoryConfig {
    fn default() -> Self {
        InventoryConfig {
            plugin: get_inventory_plugin_config(),
            options: OptionsConfig::default(),
            transform_function: None,
            transform_function_options: None,
        }
    }
}

impl InventoryConfig {
    pub fn builder() -> InventoryConfigBuilder {
        InventoryConfigBuilder::default()
    }

    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    pub fn options(&self) -> &OptionsConfig {
        &self.options
    }

    pub fn transform_function(&self) -> Option<&str> {
        self.transform_function.as_deref()
    }

    pub fn transform_function_options(&self) -> Option<&TransformFunctionOptions> {
        self.transform_function_options.as_ref()
    }
}

/// Builder for constructing `InventoryConfig` instances with custom settings.
///
/// This builder provides a fluent interface for creating `InventoryConfig` objects,
/// allowing selective configuration of inventory management settings. Fields that are
/// not explicitly set will use their default values when `build()` is called.
///
/// # Fields
///
/// * `plugin` - Optional name of the inventory plugin to use. When set to `Some(name)`,
///   the specified plugin will be used for loading inventory data. When set to `None`,
///   the default value from the `GENJA_INVENTORY_PLUGIN` environment variable or
///   "FileInventoryPlugin" will be used.
/// * `options` - Optional configuration for inventory file paths. When set to
///   `Some(options)`, the specified paths for hosts, groups, and defaults files will
///   be used. When set to `None`, default `OptionsConfig` values (all `None`) will be used.
/// * `transform_function` - Optional name of a transformation function to apply to
///   the loaded inventory data. When set to `Some(name)`, the specified function will
///   be invoked to transform the inventory. When set to `None`, no transformation
///   will be applied.
/// * `transform_function_options` - Optional JSON configuration passed to the
///   transformation function. When set to `Some(value)`, the specified JSON object
///   will be provided as parameters to the transformation function. When set to `None`,
///   no options will be passed to the transformation function.
///
/// # Examples
///
/// ```
/// use genja_core::inventory::TransformFunctionOptions;
/// use genja_core::settings::{InventoryConfig, OptionsConfig};
///
/// // Build with custom plugin and options
/// let config = InventoryConfig::builder()
///     .plugin("CustomInventoryPlugin")
///     .options(OptionsConfig::builder()
///         .hosts_file("/path/to/hosts.yaml")
///         .build())
///     .transform_function("my_transform")
///     .transform_function_options(
///         TransformFunctionOptions::new(serde_json::json!({"key": "value"})),
///     )
///     .build();
///
/// // Build with defaults
/// let config = InventoryConfig::builder().build();
/// ```
pub struct InventoryConfigBuilder {
    plugin: Option<String>,
    options: Option<OptionsConfig>,
    transform_function: Option<String>,
    transform_function_options: Option<TransformFunctionOptions>,
}

impl InventoryConfigBuilder {
    pub fn plugin(mut self, plugin: impl Into<String>) -> Self {
        self.plugin = Some(plugin.into());
        self
    }

    pub fn options(mut self, options: OptionsConfig) -> Self {
        self.options = Some(options);
        self
    }

    pub fn transform_function(mut self, transform: impl Into<String>) -> Self {
        self.transform_function = Some(transform.into());
        self
    }

    pub fn transform_function_options(mut self, options: TransformFunctionOptions) -> Self {
        self.transform_function_options = Some(options);
        self
    }

    pub fn build(self) -> InventoryConfig {
        InventoryConfig {
            plugin: self.plugin.unwrap_or_else(get_inventory_plugin_config),
            options: self.options.unwrap_or_default(),
            transform_function: self.transform_function,
            transform_function_options: self.transform_function_options,
        }
    }
}

impl Default for InventoryConfigBuilder {
    fn default() -> Self {
        Self {
            plugin: None,
            options: None,
            transform_function: None,
            transform_function_options: None,
        }
    }
}

/// Configuration for core Genja behavior.
///
/// This struct controls fundamental aspects of how Genja operates, particularly
/// error handling behavior. The configuration can be loaded from files or
/// environment variables, with flexible boolean parsing support.
///
/// # Fields
///
/// * `raise_on_error` - Controls whether Genja should raise (panic/abort) on errors
///   or handle them gracefully. When `true`, errors will cause the application to
///   terminate immediately. When `false`, errors are handled and reported without
///   terminating execution. Defaults to the value from the `GENJA_CORE_RAISE_ON_ERROR`
///   environment variable, or `false` if not set. Supports loose boolean parsing,
///   accepting values like "true", "yes", "1", "on" for true, and "false", "no",
///   "0", "off" for false (case-insensitive).
///
/// # Deserialization
///
/// - Missing fields use their default values (see `Default` impl)
/// - The `raise_on_error` field defaults to `GENJA_CORE_RAISE_ON_ERROR` env var or `false`
/// - Invalid field values cause deserialization to fail
///
/// # Examples
///
/// ```
/// use genja_core::settings::CoreConfig;
///
/// // Create with default values
/// let config = CoreConfig::default();
///
/// // Check error handling behavior
/// if config.raise_on_error() {
///     println!("Errors will cause immediate termination");
/// } else {
///     println!("Errors will be handled gracefully");
/// }
/// ```
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CoreConfig {
    #[serde(
        default = "raise_on_error",
        deserialize_with = "deserialize_bool_loose"
    )]
    raise_on_error: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        CoreConfig {
            raise_on_error: raise_on_error(),
        }
    }
}

impl CoreConfig {
    pub fn builder() -> CoreConfigBuilder {
        CoreConfigBuilder::default()
    }

    pub fn raise_on_error(&self) -> bool {
        self.raise_on_error
    }
}

/// Builder for constructing `CoreConfig` instances with custom settings.
///
/// This builder provides a fluent interface for creating `CoreConfig` objects,
/// allowing selective configuration of core behavior settings. Fields that are
/// not explicitly set will use their default values when `build()` is called.
///
/// # Fields
///
/// * `raise_on_error` - Optional flag controlling error handling behavior. When set to
///   `Some(true)`, errors will cause immediate termination. When set to `Some(false)`,
///   errors will be handled gracefully. If `None`, the default value from the
///   `GENJA_CORE_RAISE_ON_ERROR` environment variable or `false` will be used.
///
/// # Examples
///
/// ```
/// use genja_core::settings::CoreConfig;
///
/// // Build with custom error handling
/// let config = CoreConfig::builder()
///     .raise_on_error(true)
///     .build();
///
/// // Build with defaults
/// let config = CoreConfig::builder().build();
/// ```
pub struct CoreConfigBuilder {
    raise_on_error: Option<bool>,
}

impl CoreConfigBuilder {
    pub fn raise_on_error(mut self, raise_on_error: bool) -> Self {
        self.raise_on_error = Some(raise_on_error);
        self
    }

    pub fn build(self) -> CoreConfig {
        CoreConfig {
            raise_on_error: self.raise_on_error.unwrap_or_else(raise_on_error),
        }
    }
}

impl Default for CoreConfigBuilder {
    fn default() -> Self {
        Self {
            raise_on_error: None,
        }
    }
}

/// Configuration for SSH client settings.
///
/// This struct holds optional SSH configuration settings that can be used to customize
/// SSH client behavior. It supports loading SSH configuration from a file, which can
/// contain standard SSH client configuration directives.
///
/// # Fields
///
/// * `config_file` - Optional path to an SSH configuration file. When provided, this file
///   should contain valid SSH client configuration directives (e.g., Host entries, connection
///   settings, authentication options). The file format should follow the standard SSH config
///   file syntax as defined by OpenSSH. If `None`, no SSH configuration file will be used.
///
/// # Deserialization
///
/// - Missing fields default to `None`
/// - Invalid field values cause deserialization to fail
///
/// # Examples
///
/// ```
/// use genja_core::settings::SSHConfig;
///
/// // Create with default values (no config file)
/// let config = SSHConfig::default();
///
/// // Create with a specific SSH config file
/// let config = SSHConfig::builder()
///     .config_file("/home/user/.ssh/config")
///     .build();
///
/// // Validate the SSH config file syntax
/// if let Err(e) = config.validate() {
///     eprintln!("Invalid SSH config: {}", e);
/// }
/// ```
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct SSHConfig {
    config_file: Option<String>,
}
impl Default for SSHConfig {
    fn default() -> Self {
        SSHConfig { config_file: None }
    }
}

impl SSHConfig {
    pub fn builder() -> SSHConfigBuilder {
        SSHConfigBuilder::default()
    }

    pub fn config_file(&self) -> Option<&str> {
        self.config_file.as_deref()
    }
}

/// Builder for constructing `SSHConfig` instances with custom settings.
///
/// This builder provides a fluent interface for creating `SSHConfig` objects,
/// allowing selective configuration of SSH client settings. Fields that are
/// not explicitly set will use their default values when `build()` is called.
///
/// # Fields
///
/// * `config_file` - Optional path to an SSH configuration file. When set to
///   `Some(path)`, the SSH configuration will be loaded from the specified file.
///   When set to `None`, no SSH configuration file will be used. The file should
///   contain valid SSH client configuration directives following the standard
///   SSH config file syntax as defined by OpenSSH.
///
/// # Examples
///
/// ```
/// use genja_core::settings::SSHConfig;
///
/// // Build with custom SSH config file
/// let config = SSHConfig::builder()
///     .config_file("/home/user/.ssh/config")
///     .build();
///
/// // Build with defaults (no config file)
/// let config = SSHConfig::builder().build();
/// ```
pub struct SSHConfigBuilder {
    config_file: Option<String>,
}

impl SSHConfigBuilder {
    pub fn config_file(mut self, path: impl Into<String>) -> Self {
        self.config_file = Some(path.into());
        self
    }

    pub fn build(self) -> SSHConfig {
        SSHConfig {
            config_file: self.config_file,
        }
    }
}

impl Default for SSHConfigBuilder {
    fn default() -> Self {
        Self { config_file: None }
    }
}

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
    // #[serde(default = "get_runner_options_default")]_runner_options_default")]
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

/// Stores the logging configuration for Genja.
///
/// If the user does not specify a logging configuration in their config file,
/// the default values will be used.
///
/// This struct defines how logging should be configured, including log levels,
/// output destinations, and log file rotation settings. The configuration supports
/// flexible boolean parsing for enabled and console output flags.
///
/// **Note:** Genja does not initialize logging itself. The user must configure
/// the logging subscriber in their application code. See the documentation in
/// `lib.rs` for examples of how to set up logging using these configuration values.
///
/// # Fields
///
/// * `enabled` - Controls whether logging is enabled. When `false`, logging should
///   be disabled entirely. Supports loose boolean parsing (e.g., "true", "yes", "1").
///   Defaults to `true`.
/// * `level` - The logging level to use (e.g., "trace", "debug", "info", "warn", "error").
///   Defaults to the value from the `GENJA_LOGGING_LEVEL` environment variable,
///   or "info" if not set.
/// * `log_file` - The file path where logs should be written. Defaults to the value
///   from the `GENJA_LOGGING_LOG_FILE` environment variable, or `genja.log` in the
///   current working directory if not set.
/// * `to_console` - Controls whether logs should be written to the console in addition
///   to the log file. Supports loose boolean parsing. Defaults to the value from the
///   `GENJA_LOGGING_TO_CONSOLE` environment variable, or `false` if not set.
/// * `file_size` - The maximum size in bytes for a single log file before rotation
///   occurs. Defaults to 10 MB (10485760 bytes).
/// * `max_file_count` - The maximum number of rotated log files to keep. Older files
///   are deleted when this limit is exceeded. Defaults to 10.
///
/// # Deserialization
///
/// - Missing fields use their default values (see `Default` impl)
/// - The `level` field defaults to `GENJA_LOGGING_LEVEL` env var or "info"
/// - The `log_file` field defaults to `GENJA_LOGGING_LOG_FILE` env var or `genja.log`
///   in the current working directory
/// - The `to_console` field defaults to `GENJA_LOGGING_TO_CONSOLE` env var or `false`
/// - Invalid field values cause deserialization to fail
///
/// # Examples
///
/// ```
/// use genja_core::settings::LoggingConfig;
///
/// // Create with default values
/// let config = LoggingConfig::default();
///
/// // Create with custom configuration
/// let config = LoggingConfig::builder()
///     .enabled(true)
///     .level("debug")
///     .log_file("/var/log/genja.log")
///     .to_console(true)
///     .file_size(1024 * 1024 * 5) // 5 MB
///     .max_file_count(5)
///     .build();
/// ```
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct LoggingConfig {
    #[serde(deserialize_with = "deserialize_bool_loose")]
    enabled: bool,
    level: String,
    log_file: String,
    #[serde(deserialize_with = "deserialize_bool_loose")]
    to_console: bool,
    file_size: u64,
    max_file_count: usize,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: get_log_level_default(),
            log_file: get_default_log_file(),
            to_console: get_log_to_console_default(),
            file_size: 1024 * 1024 * 10, // 10 MB
            max_file_count: 10,
        }
    }
}

impl LoggingConfig {
    pub fn builder() -> LoggingConfigBuilder {
        LoggingConfigBuilder::default()
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn level(&self) -> &str {
        &self.level
    }

    pub fn log_file(&self) -> &str {
        &self.log_file
    }

    pub fn to_console(&self) -> bool {
        self.to_console
    }

    pub fn file_size(&self) -> u64 {
        self.file_size
    }

    pub fn max_file_count(&self) -> usize {
        self.max_file_count
    }
}

/// Builder for constructing `LoggingConfig` instances with custom settings.
///
/// This builder provides a fluent interface for creating `LoggingConfig` objects,
/// allowing selective configuration of logging behavior. Fields that are not
/// explicitly set will use their default values when `build()` is called.
///
/// # Fields
///
/// * `enabled` - Optional flag controlling whether logging is enabled. When set to
///   `Some(true)`, logging will be enabled. When set to `Some(false)`, logging will
///   be disabled. If `None`, the default value of `true` will be used.
/// * `level` - Optional logging level (e.g., "trace", "debug", "info", "warn", "error").
///   When set to `Some(level)`, the specified level will be used. If `None`, the default
///   value from the `GENJA_LOGGING_LEVEL` environment variable or "info" will be used.
/// * `log_file` - Optional file path where logs should be written. When set to
///   `Some(path)`, logs will be written to the specified file. If `None`, the default
///   value from the `GENJA_LOGGING_LOG_FILE` environment variable or `genja.log`
///   in the current working directory will be used.
/// * `to_console` - Optional flag controlling whether logs should be written to the
///   console in addition to the log file. When set to `Some(true)`, console logging
///   will be enabled. When set to `Some(false)`, console logging will be disabled.
///   If `None`, the default value from the `GENJA_LOGGING_TO_CONSOLE` environment
///   variable or `false` will be used.
/// * `file_size` - Optional maximum size in bytes for a single log file before rotation
///   occurs. When set to `Some(size)`, the specified size limit will be used. If `None`,
///   the default value of 10 MB (10485760 bytes) will be used.
/// * `max_file_count` - Optional maximum number of rotated log files to keep. When set
///   to `Some(count)`, the specified limit will be used. If `None`, the default value
///   of 10 will be used.
///
/// # Examples
///
/// ```
/// use genja_core::settings::LoggingConfig;
///
/// // Build with custom settings
/// let config = LoggingConfig::builder()
///     .enabled(true)
///     .level("debug")
///     .log_file("/var/log/genja.log")
///     .to_console(true)
///     .file_size(1024 * 1024 * 5) // 5 MB
///     .max_file_count(5)
///     .build();
///
/// // Build with defaults
/// let config = LoggingConfig::builder().build();
/// ```
pub struct LoggingConfigBuilder {
    enabled: Option<bool>,
    level: Option<String>,
    log_file: Option<String>,
    to_console: Option<bool>,
    file_size: Option<u64>,
    max_file_count: Option<usize>,
}

impl LoggingConfigBuilder {
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn level(mut self, level: impl Into<String>) -> Self {
        self.level = Some(level.into());
        self
    }

    pub fn log_file(mut self, log_file: impl Into<String>) -> Self {
        self.log_file = Some(log_file.into());
        self
    }

    pub fn to_console(mut self, to_console: bool) -> Self {
        self.to_console = Some(to_console);
        self
    }

    pub fn file_size(mut self, file_size: u64) -> Self {
        self.file_size = Some(file_size);
        self
    }

    pub fn max_file_count(mut self, max_file_count: usize) -> Self {
        self.max_file_count = Some(max_file_count);
        self
    }

    pub fn build(self) -> LoggingConfig {
        LoggingConfig {
            enabled: self.enabled.unwrap_or(true),
            level: self.level.unwrap_or_else(get_log_level_default),
            log_file: self.log_file.unwrap_or_else(get_default_log_file),
            to_console: self.to_console.unwrap_or_else(get_log_to_console_default),
            file_size: self.file_size.unwrap_or(1024 * 1024 * 10),
            max_file_count: self.max_file_count.unwrap_or(10),
        }
    }
}

impl Default for LoggingConfigBuilder {
    fn default() -> Self {
        Self {
            enabled: None,
            level: None,
            log_file: None,
            to_console: None,
            file_size: None,
            max_file_count: None,
        }
    }
}

/// Main configuration container for Genja.
///
/// Aggregates all configuration sections (core, inventory, runner, logging, SSH)
/// and provides methods for loading from files and accessing subsections.
///
/// # Deserialization
///
/// - Missing fields use their default values (see `Default` impl)
/// - Invalid field values cause deserialization to fail
///
/// # Examples
///
/// ```
/// use genja_core::Settings;
///
/// // Create with default values
/// let settings = Settings::default();
///
/// // Create with custom values using builders
/// let settings = Settings::builder()
///     .logging(
///         genja_core::settings::LoggingConfig::builder()
///             .level("debug")
///             .build(),
///     )
///     .build();
///
/// // Access subsections
/// println!("Log level: {}", settings.logging().level());
/// ```
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    // #[serde(default = "CoreConfig::default")]
    core: CoreConfig,
    inventory: InventoryConfig,
    ssh: SSHConfig,
    runner: RunnerConfig,
    logging: LoggingConfig,
}

impl Settings {
    /// Loads Genja settings from a configuration file.
    ///
    /// This method reads and deserializes a configuration file into a `Settings` instance.
    /// The file format is automatically determined based on the file extension. After
    /// loading, the method validates any SSH configuration that was specified to ensure
    /// it contains valid SSH config syntax.
    ///
    /// # Parameters
    ///
    /// * `file_path` - The path to the configuration file to load. The file extension
    ///   determines the deserialization format: `.json` for JSON, `.yaml` or `.yml` for YAML.
    ///   The file must exist and be readable.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Ok(Settings)` - If the file was successfully loaded, parsed, and validated.
    ///   The returned `Settings` instance contains all configuration sections with values
    ///   from the file merged with defaults for any missing fields.
    /// * `Err(ConfigLoadError)` - If an error occurred during loading, parsing, or validation.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigLoadError`] if:
    /// * The file has an unsupported extension (not `.json`, `.yaml`, or `.yml`)
    /// * The file cannot be read (e.g., doesn't exist, permission denied)
    /// * The file contents cannot be parsed as valid JSON or YAML
    /// * The file structure doesn't match the expected `Settings` schema
    /// * The SSH configuration file (if specified) fails validation
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::Settings;
    ///
    /// // Load from a JSON file
    /// let settings = Settings::from_file("config.json").unwrap();
    ///
    /// // Load from a YAML file
    /// let settings = Settings::from_file("config.yaml").unwrap();
    ///
    /// // Handle errors
    /// match Settings::from_file("config.yml") {
    ///     Ok(settings) => println!("Loaded settings successfully"),
    ///     Err(e) => eprintln!("Failed to load settings: {}", e),
    /// }
    /// ```
    pub fn from_file(file_path: &str) -> Result<Self, ConfigLoadError> {
        let format = if file_path.ends_with(".json") {
            FileFormat::Json
        } else if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
            FileFormat::Yaml
        } else {
            return Err(ConfigLoadError::UnsupportedFormat {
                path: file_path.to_string(),
            });
        };
        let config = ConfigBuilder::builder()
            .add_source(File::new(file_path, format).required(true))
            .build()
            .map_err(|err| ConfigLoadError::Read {
                path: file_path.to_string(),
                message: err.to_string(),
            })?;
        let parsed_config: Settings =
            config
                .try_deserialize()
                .map_err(|err| ConfigLoadError::Deserialize {
                    path: file_path.to_string(),
                    message: err.to_string(),
                })?;

        // Validate SSH config syntax if provided
        parsed_config
            .ssh
            .validate()
            .map_err(ConfigLoadError::SshConfig)?;
        Ok(parsed_config)
    }

    pub fn core(&self) -> &CoreConfig {
        &self.core
    }

    pub fn inventory(&self) -> &InventoryConfig {
        &self.inventory
    }

    pub fn ssh(&self) -> &SSHConfig {
        &self.ssh
    }

    pub fn runner(&self) -> &RunnerConfig {
        &self.runner
    }

    pub fn logging(&self) -> &LoggingConfig {
        &self.logging
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            core: CoreConfig::default(),
            inventory: InventoryConfig::default(),
            ssh: SSHConfig::default(),
            runner: RunnerConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Settings {
    pub fn builder() -> SettingsBuilder {
        SettingsBuilder::default()
    }
}

/// Builder for constructing `Settings` instances with custom configuration sections.
///
/// This builder provides a fluent interface for creating `Settings` objects,
/// allowing selective configuration of different subsystems (core, inventory, SSH,
/// runner, and logging). Fields that are not explicitly set will use their default
/// values when `build()` is called.
///
/// # Fields
///
/// * `core` - Optional core configuration controlling fundamental Genja behavior
///   such as error handling. When set to `Some(config)`, the specified core
///   configuration will be used. If `None`, the default `CoreConfig` will be used.
/// * `inventory` - Optional inventory configuration specifying how inventory data
///   (hosts, groups, defaults) should be loaded and processed. When set to
///   `Some(config)`, the specified inventory configuration will be used. If `None`,
///   the default `InventoryConfig` will be used.
/// * `ssh` - Optional SSH configuration for SSH client settings. When set to
///   `Some(config)`, the specified SSH configuration will be used. If `None`,
///   the default `SSHConfig` will be used.
/// * `runner` - Optional runner configuration specifying which task execution
///   plugin to use and its options. When set to `Some(config)`, the specified
///   runner configuration will be used. If `None`, the default `RunnerConfig`
///   will be used.
/// * `logging` - Optional logging configuration controlling log levels, output
///   destinations, and rotation settings. When set to `Some(config)`, the
///   specified logging configuration will be used. If `None`, the default
///   `LoggingConfig` will be used.
///
/// # Examples
///
/// ```
/// use genja_core::Settings;
/// use genja_core::settings::{LoggingConfig, RunnerConfig};
///
/// // Build with custom logging and runner configurations
/// let settings = Settings::builder()
///     .logging(LoggingConfig::builder()
///         .level("debug")
///         .to_console(true)
///         .build())
///     .runner(RunnerConfig::builder()
///         .plugin("threaded")
///         .worker_count(5)
///         .build())
///     .build();
///
/// // Build with defaults
/// let settings = Settings::builder().build();
/// ```
pub struct SettingsBuilder {
    core: Option<CoreConfig>,
    inventory: Option<InventoryConfig>,
    ssh: Option<SSHConfig>,
    runner: Option<RunnerConfig>,
    logging: Option<LoggingConfig>,
}

impl SettingsBuilder {
    pub fn core(mut self, core: CoreConfig) -> Self {
        self.core = Some(core);
        self
    }

    pub fn inventory(mut self, inventory: InventoryConfig) -> Self {
        self.inventory = Some(inventory);
        self
    }

    pub fn ssh(mut self, ssh: SSHConfig) -> Self {
        self.ssh = Some(ssh);
        self
    }

    pub fn runner(mut self, runner: RunnerConfig) -> Self {
        self.runner = Some(runner);
        self
    }

    pub fn logging(mut self, logging: LoggingConfig) -> Self {
        self.logging = Some(logging);
        self
    }

    pub fn build(self) -> Settings {
        Settings {
            core: self.core.unwrap_or_default(),
            inventory: self.inventory.unwrap_or_default(),
            ssh: self.ssh.unwrap_or_default(),
            runner: self.runner.unwrap_or_default(),
            logging: self.logging.unwrap_or_default(),
        }
    }
}

impl Default for SettingsBuilder {
    fn default() -> Self {
        Self {
            core: None,
            inventory: None,
            ssh: None,
            runner: None,
            logging: None,
        }
    }
}
