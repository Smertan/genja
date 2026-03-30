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
//! - `GENJA_LOGGING_LOG_FILE` - Log file path (default: "./genja.log")
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
//! # Ok::<(), config::ConfigError>(())
//! ```
//!
//! ## SSH Validation
//! SSH config is validated automatically when calling `Settings::from_file`.
//! For manual validation, use `SSHConfig::validate`.
use crate::inventory::{Defaults, Groups, Hosts, TransformFunctionOptions};
use config::{Config as ConfigBuilder, ConfigError, File, FileFormat};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use ssh2_config::{ParseRule, SshConfig};
use std::env;
use std::fs::File as StdFile;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};

// Environment variable names
/// Environment variable name for controlling error handling behavior.
///
/// When set, this variable determines whether Genja should raise (panic/abort) on errors
/// or handle them gracefully. Accepts boolean-like values such as "true", "false", "yes",
/// "no", "1", "0", "on", "off" (case-insensitive).
///
/// Default: `false` (errors are handled gracefully)
const ENV_RAISE_ON_ERROR: &str = "GENJA_CORE_RAISE_ON_ERROR";

/// Environment variable name for specifying the inventory plugin.
///
/// This variable determines which inventory plugin implementation should be used
/// for loading and managing host inventory data.
///
/// Default: `"FileInventoryPlugin"`
const ENV_INVENTORY_PLUGIN: &str = "GENJA_INVENTORY_PLUGIN";

/// Environment variable name for specifying the runner plugin.
///
/// This variable determines which runner plugin implementation should be used
/// for executing tasks across hosts (e.g., "threaded", "sequential").
///
/// Default: `"threaded"`
const ENV_RUNNER_PLUGIN: &str = "GENJA_RUNNER_PLUGIN";

/// Environment variable name for setting the logging level.
///
/// This variable controls the verbosity of log output. Valid values include
/// "trace", "debug", "info", "warn", and "error".
///
/// Default: `"info"`
const ENV_LOG_LEVEL: &str = "GENJA_LOGGING_LEVEL";

/// Environment variable name for specifying the log file path.
///
/// This variable determines where log output should be written. If not set,
/// logs are written to `genja.log` in the current working directory.
///
/// Default: `genja.log` in the current working directory
const ENV_LOG_FILE: &str = "GENJA_LOGGING_LOG_FILE";

/// Environment variable name for enabling console logging.
///
/// When set, this variable determines whether logs should be written to the console
/// in addition to the log file. Accepts boolean-like values such as "true", "false",
/// "yes", "no", "1", "0", "on", "off" (case-insensitive).
///
/// Default: `false` (console logging disabled)
const ENV_LOG_TO_CONSOLE: &str = "GENJA_LOGGING_TO_CONSOLE";

/// Parses a string into a boolean value using loose matching rules.
///
/// This function accepts various common string representations of boolean values,
/// performing case-insensitive matching after trimming whitespace. It recognizes
/// multiple formats for both true and false values.
///
/// # Parameters
///
/// * `s` - A string slice containing the value to parse. Leading and trailing
///   whitespace will be trimmed before parsing.
///
/// # Returns
///
/// * `Some(true)` - If the string matches any of: "true", "t", "1", "yes", "y", "on"
///   (case-insensitive)
/// * `Some(false)` - If the string matches any of: "false", "f", "0", "no", "n", "off"
///   (case-insensitive)
/// * `None` - If the string does not match any recognized boolean representation
fn parse_bool_loose(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "true" | "t" | "1" | "yes" | "y" | "on" => Some(true),
        "false" | "f" | "0" | "no" | "n" | "off" => Some(false),
        _ => None,
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BoolLike {
    Bool(bool),
    String(String),
}

fn deserialize_bool_loose<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<BoolLike>::deserialize(deserializer)?;
    match value {
        Some(BoolLike::Bool(val)) => Ok(val),
        Some(BoolLike::String(val)) => parse_bool_loose(val.as_str())
            .ok_or_else(|| serde::de::Error::custom(format!("invalid boolean value: {val:?}"))),
        None => Ok(false),
    }
}

fn raise_on_error() -> bool {
    match std::env::var(ENV_RAISE_ON_ERROR) {
        Ok(s) => match parse_bool_loose(s.as_str()) {
            Some(true) => true,
            Some(false) => false,
            _ => {
                eprintln!(
                    "Invalid {} value: {:?}, using default false",
                    ENV_RAISE_ON_ERROR, s
                );
                false
            }
        },
        Err(_) => {
            eprintln!("{} not found, using default false", ENV_RAISE_ON_ERROR);
            false
        }
    }
}

/// Retrieves the inventory plugin configuration from environment variables.
///
/// This function checks the `GENJA_INVENTORY_PLUGIN` environment variable to determine
/// which inventory plugin implementation should be used. If the environment variable is
/// not set or cannot be read, it returns a default value.
///
/// # Returns
///
/// Returns a `String` containing the name of the inventory plugin to use. If the
/// `GENJA_INVENTORY_PLUGIN` environment variable is set, returns its value. Otherwise,
/// returns `"FileInventoryPlugin"` as the default.
///
/// See tests in this module for behavioral verification.
fn get_inventory_plugin_config() -> String {
    env::var(ENV_INVENTORY_PLUGIN).unwrap_or_else(|_err| String::from("FileInventoryPlugin"))
}

/// Returns the default runner plugin from `GENJA_RUNNER_PLUGIN`, or "threaded".
///
/// See tests in this module for behavioral verification.
fn get_runner_plugin_default() -> String {
    env::var(ENV_RUNNER_PLUGIN).unwrap_or_else(|_err| String::from("threaded"))
}

/// Returns the default runner options JSON.
///
/// See tests in this module for behavioral verification.
fn get_runner_options_default() -> serde_json::Value {
    serde_json::json!({
        "num_of_workers": 10
    })
}

/// Returns the default max task depth for runner execution.
///
/// See tests in this module for behavioral verification.
fn get_runner_max_task_depth_default() -> usize {
    10
}

/// Returns the default log level from `GENJA_LOGGING_LEVEL`, or "info".
///
/// See tests in this module for behavioral verification.
fn get_log_level_default() -> String {
    env::var(ENV_LOG_LEVEL).unwrap_or_else(|_err| String::from("info"))
}

/// Returns the default console logging flag from `GENJA_LOGGING_TO_CONSOLE`.
///
/// See tests in this module for behavioral verification.
fn get_log_to_console_default() -> bool {
    match env::var(ENV_LOG_TO_CONSOLE) {
        Ok(val) => parse_bool_loose(val.as_str()).unwrap_or(false),
        Err(_) => false,
    }
}

/// Returns the default log file path, preferring `GENJA_LOGGING_LOG_FILE` when set.
///
/// When the environment variable is not set, defaults to `genja.log` in the
/// current working directory.
///
/// See tests in this module for behavioral verification.
fn get_default_log_file() -> String {
    match env::var(ENV_LOG_FILE) {
        Ok(val) => val,
        Err(_) => {
            let start_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            start_dir.join("genja.log").to_string_lossy().to_string()
        }
    }
}

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

impl InventoryConfig {
    /// Loads inventory data from configured file paths.
    ///
    /// This method reads and deserializes inventory files (hosts, groups, and defaults)
    /// based on the paths specified in the `options` field. If a file path is not
    /// provided for a particular inventory component, a default or empty value is used.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing a tuple of:
    /// * `Hosts` - The loaded hosts inventory. If no hosts file is specified, returns
    ///   an empty `Hosts` instance.
    /// * `Option<Groups>` - The loaded groups inventory, or `None` if no groups file
    ///   is specified.
    /// * `Option<Defaults>` - The loaded defaults inventory, or `None` if no defaults
    ///   file is specified.
    ///
    /// # Errors
    ///
    /// Returns an error string if:
    /// * Any specified file cannot be read
    /// * Any file contains invalid JSON or YAML syntax
    /// * A file has an unsupported format (not .json, .yaml, or .yml)
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::settings::InventoryConfig;
    ///
    /// let config = InventoryConfig::default();
    /// match config.load_inventory_files() {
    ///     Ok((hosts, groups, defaults)) => {
    ///         println!("Loaded {} hosts", hosts.len());
    ///     }
    ///     Err(e) => eprintln!("Failed to load inventory: {}", e),
    /// }
    /// ```
    // FIXME: Fix the error handling for the inventory loading process.

    pub fn load_inventory_files(
        &self,
    ) -> Result<(Hosts, Option<Groups>, Option<Defaults>), String> {
        let hosts = match self.options.hosts_file.as_deref() {
            Some(path) => Self::load_from_file::<Hosts>(path)?,
            None => Hosts::new(),
        };

        let groups = match self.options.groups_file.as_deref() {
            Some(path) => Some(Self::load_from_file::<Groups>(path)?),
            None => None,
        };

        let defaults = match self.options.defaults_file.as_deref() {
            Some(path) => Some(Self::load_from_file::<Defaults>(path)?),
            None => None,
        };

        Ok((hosts, groups, defaults))
    }

    /// Loads and deserializes data from a file.
    ///
    /// This helper method reads a file from the filesystem and deserializes its contents
    /// based on the file extension. Supports JSON (.json) and YAML (.yaml, .yml) formats.
    ///
    /// # Type Parameters
    ///
    /// * `T` - The type to deserialize the file contents into. Must implement `DeserializeOwned`.
    ///
    /// # Parameters
    ///
    /// * `path` - The file path to read and deserialize. The file extension determines
    ///   the deserialization format.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the deserialized data of type `T`.
    ///
    /// # Errors
    ///
    /// Returns an error string if:
    /// * The file cannot be read (e.g., doesn't exist, permission denied)
    /// * The file contents cannot be parsed as valid JSON or YAML
    /// * The file has an unsupported extension (not .json, .yaml, or .yml)
    fn load_from_file<T>(path: &str) -> Result<T, String>
    where
        T: DeserializeOwned,
    {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read file {}: {}", path, e))?;

        if path.ends_with(".json") {
            serde_json::from_str(&contents)
                .map_err(|e| format!("Failed to parse JSON file {}: {}", path, e))
        } else if path.ends_with(".yaml") || path.ends_with(".yml") {
            serde_yaml::from_str(&contents)
                .map_err(|e| format!("Failed to parse YAML file {}: {}", path, e))
        } else {
            Err(format!(
                "Unsupported file format for {}. Use .json, .yaml, or .yml",
                path
            ))
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
impl SSHConfig {
    /// Validates the SSH configuration file syntax if a path is provided.
    ///
    /// This method performs comprehensive validation of an SSH configuration file by:
    /// 1. Verifying that the file exists and is accessible
    /// 2. Opening the file for reading
    /// 3. Parsing the file contents using strict SSH config syntax rules
    ///
    /// If no SSH configuration file is specified (the `config_file` field is `None`),
    /// this method returns `Ok(())` without performing any validation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if:
    /// * No config file is specified (nothing to validate)
    /// * The config file exists, can be opened, and contains valid SSH configuration syntax
    ///
    /// Returns `Err(String)` with a descriptive error message if:
    /// * The specified file does not exist or cannot be accessed
    /// * The file cannot be opened due to permission issues or other I/O errors
    /// * The file contents cannot be parsed as valid SSH configuration syntax
    ///
    /// # Errors
    ///
    /// This method returns an error in the following cases:
    /// * File existence check fails (see `ensure_exists` for details)
    /// * `"Failed to open SSH config file {path}: {error}"` - The file exists but cannot be opened
    /// * `"Failed to parse SSH config file {path}: {error}"` - The file contains invalid SSH config syntax
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::settings::SSHConfig;
    ///
    /// let config = SSHConfig::builder()
    ///     .config_file("/home/user/.ssh/config")
    ///     .build();
    ///
    /// match config.validate() {
    ///     Ok(()) => println!("SSH config is valid"),
    ///     Err(e) => eprintln!("Invalid SSH config: {}", e),
    /// }
    /// ```
    pub fn validate(&self) -> Result<(), String> {
        if let Some(ref path) = self.config_file {
            let path = Path::new(path);

            // TODO: Improve the error handling in case there is an error due to permissions or other issues.
            match self.ensure_exists(path) {
                Ok(()) => (),
                Err(e) => return Err(format!("{e}")),
            }
            // path.try_exists()
            //     .expect(format!("SSH config file not found: {:?}", path).as_str());

            let file = match StdFile::open(path) {
                Ok(file) => file,
                Err(e) => {
                    return Err(format!(
                        "Failed to open SSH config file {}: {}",
                        path.display(),
                        e
                    ))
                }
            };
            let mut reader = BufReader::new(file);
            // .expect("Could not open configuration file");

            match SshConfig::default().parse(&mut reader, ParseRule::STRICT) {
                Ok(_) => return Ok(()),
                Err(e) => {
                    return Err(format!(
                        "Failed to parse SSH config file {}: {}",
                        path.display(),
                        e
                    ))
                }
            };
        } else {
            Ok(()) // No config file specified, nothing to validate
        }
    }

    /// Parses the SSH configuration file and returns the parsed configuration.
    ///
    /// This method reads and parses an SSH configuration file if one is specified in the
    /// `config_file` field. The parsing follows strict SSH config file syntax rules as
    /// defined by OpenSSH. If no configuration file is specified, the method returns
    /// `Ok(None)` without performing any parsing.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Ok(Some(SshConfig))` - If a config file is specified and successfully parsed,
    ///   containing the parsed SSH configuration with all host entries and settings.
    /// * `Ok(None)` - If no config file is specified (the `config_file` field is `None`).
    /// * `Err(String)` - If an error occurs during parsing, with a descriptive error message.
    ///
    /// # Errors
    ///
    /// Returns an error string if:
    /// * The specified SSH config file does not exist at the given path
    /// * The file cannot be opened due to permission issues or other I/O errors
    /// * The file contents cannot be parsed as valid SSH configuration syntax
    /// * The file contains syntax errors or invalid SSH configuration directives
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::settings::SSHConfig;
    ///
    /// let config = SSHConfig::builder()
    ///     .config_file("/home/user/.ssh/config")
    ///     .build();
    ///
    /// match config.parse() {
    ///     Ok(Some(ssh_config)) => {
    ///         println!("Successfully parsed SSH config");
    ///     }
    ///     Ok(None) => {
    ///         println!("No SSH config file specified");
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to parse SSH config: {}", e);
    ///     }
    /// }
    /// ```
    pub fn parse(&self) -> Result<Option<SshConfig>, String> {
        if let Some(ref path) = self.config_file {
            let path = Path::new(path);

            if !path.exists() {
                return Err(format!("SSH config file not found: {:?}", path));
            }

            let file = match StdFile::open(path) {
                Ok(file) => file,
                Err(e) => {
                    return Err(format!(
                        "Failed to open SSH config file {:?}: {}",
                        path.display(),
                        e
                    ))
                }
            };
            let mut reader = BufReader::new(file);

            match SshConfig::default().parse(&mut reader, ParseRule::STRICT) {
                Ok(config) => Ok(Some(config)),
                Err(e) => Err(format!(
                    "Failed to parse SSH config file {}: {}",
                    path.display(),
                    e
                )),
            }
        } else {
            Ok(None)
        }
    }

    /// Verifies that an SSH configuration file exists and is accessible.
    ///
    /// This method checks whether the specified file path exists and can be accessed.
    /// It provides detailed error messages for different failure scenarios, including
    /// permission issues and I/O errors.
    ///
    /// # Parameters
    ///
    /// * `path` - A reference to the file path to check. This should point to an SSH
    ///   configuration file that needs to be validated for existence and accessibility.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the file exists and is accessible. Returns `Err(String)` with
    /// a descriptive error message if:
    /// * The file does not exist
    /// * Permission is denied when attempting to access the file
    /// * An I/O error occurs during the existence check
    /// * Any other filesystem error prevents verification
    ///
    /// # Errors
    ///
    /// This method returns an error in the following cases:
    /// * `"SSH config file not found: {path}"` - The file does not exist at the specified path
    /// * `"SSH config file exists but permission denied: {path}: {error}"` - The file exists
    ///   but cannot be accessed due to insufficient permissions
    /// * `"SSH config file not found (I/O error): {path}: {error}"` - An I/O error occurred
    ///   indicating the file was not found
    /// * `"Failed to check SSH config file {path}: {error}"` - Any other filesystem error
    ///   occurred during the check
    fn ensure_exists(&self, path: &Path) -> Result<(), String> {
        match path.try_exists() {
            Ok(true) => Ok(()),
            Ok(false) => Err(format!("SSH config file not found: {}", path.display())),
            Err(e) => match e.kind() {
                ErrorKind::PermissionDenied => Err(format!(
                    "SSH config file exists but permission denied: {}: {}",
                    path.display(),
                    e
                )),
                ErrorKind::NotFound => Err(format!(
                    "SSH config file not found (I/O error): {}: {}",
                    path.display(),
                    e
                )),
                _ => Err(format!(
                    "Failed to check SSH config file {}: {}",
                    path.display(),
                    e
                )),
            },
        }
    }
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
/// the execution strategy (e.g., sequential, threaded, async) and behavior for
/// running tasks across hosts.
///
/// # Fields
///
/// * `plugin` - The name of the runner plugin to use for task execution.
///   Defaults to the value from the `GENJA_RUNNER_PLUGIN` environment variable,
///   or "threaded" if not set. Common values include "threaded" for concurrent
///   execution or "sequential" for one-at-a-time execution.
/// * `options` - A JSON object containing plugin-specific configuration options.
///   The structure and available options depend on the selected plugin. For the
///   default "threaded" plugin, this typically includes `num_of_workers` to control
///   the thread pool size. Defaults to `{"num_of_workers": 10}`.
/// * `max_task_depth` - Maximum recursion depth for task/sub-task execution.
///   Defaults to `10`.
///
/// # Deserialization
///
/// - Missing fields use their default values (see `Default` impl)
/// - The `plugin` field defaults to `GENJA_RUNNER_PLUGIN` env var or "threaded"
/// - The `options` field defaults to `{"num_of_workers": 10}`
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
///     .options(serde_json::json!({"num_of_workers": 5}))
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
    max_task_depth: usize,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            plugin: get_runner_plugin_default(),
            options: get_runner_options_default(),
            max_task_depth: get_runner_max_task_depth_default(),
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

    pub fn max_task_depth(&self) -> usize {
        self.max_task_depth
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
///   value of `{"num_of_workers": 10}` will be used. The structure and available options
///   depend on the selected plugin.
/// * `max_task_depth` - Optional maximum recursion depth for task/sub-task execution. When set to
///   `Some(value)`, the specified depth will be used. If `None`, defaults to `10`.
///
/// # Examples
///
/// ```
/// use genja_core::settings::RunnerConfig;
///
/// // Build with custom plugin and options
/// let config = RunnerConfig::builder()
///     .plugin("threaded")
///     .options(serde_json::json!({"num_of_workers": 5}))
///     .build();
///
/// // Build with defaults
/// let config = RunnerConfig::builder().build();
/// ```
pub struct RunnerConfigBuilder {
    plugin: Option<String>,
    options: Option<serde_json::Value>,
    max_task_depth: Option<usize>,
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

    pub fn max_task_depth(mut self, max_task_depth: usize) -> Self {
        self.max_task_depth = Some(max_task_depth);
        self
    }

    pub fn build(self) -> RunnerConfig {
        RunnerConfig {
            plugin: self.plugin.unwrap_or_else(get_runner_plugin_default),
            options: self.options.unwrap_or_else(get_runner_options_default),
            max_task_depth: self
                .max_task_depth
                .unwrap_or_else(get_runner_max_task_depth_default),
        }
    }
}

impl Default for RunnerConfigBuilder {
    fn default() -> Self {
        Self {
            plugin: None,
            options: None,
            max_task_depth: None,
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
///   value from the `GENJA_LOGGING_LOG_FILE` environment variable or `genja.log` in
///   the current working directory
///   `genja.log` file will be used in the current working directory.
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
    /// * `Err(ConfigError)` - If an error occurred during loading, parsing, or validation.
    ///
    /// # Errors
    ///
    /// Returns a `ConfigError` if:
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
    pub fn from_file(file_path: &str) -> Result<Self, ConfigError> {
        let format = if file_path.ends_with(".json") {
            FileFormat::Json
        } else if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
            FileFormat::Yaml
        } else {
            return Err(ConfigError::Message(
                "Unsupported file format. Use .json, .yaml, or .yml".to_string(),
            ));
        };
        let config = ConfigBuilder::builder()
            .add_source(File::new(file_path, format).required(true))
            .build()?;
        let parsed_config: Settings = config.try_deserialize()?;

        // Validate SSH config syntax if provided
        if let Err(e) = parsed_config.ssh.validate() {
            return Err(ConfigError::Message(e));
        }
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
///         .options(serde_json::json!({"num_of_workers": 5}))
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

#[cfg(test)]
mod tests {
    use super::{OptionsConfig, RunnerConfig, SSHConfig};
    use regex::Regex;
    use serde_json::json;
    use std::env;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Returns a static reference to a mutex used for synchronizing environment variable access in tests.
    ///
    /// This function ensures that tests modifying environment variables do not run concurrently,
    /// preventing race conditions and test interference. The mutex is initialized once and reused
    /// across all test invocations.
    ///
    /// # Returns
    ///
    /// A static reference to a `Mutex<()>` that can be locked to serialize environment variable operations.
    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    /// Temporarily sets or removes an environment variable for the duration of a test function.
    ///
    /// This function provides a safe way to test code that depends on environment variables by:
    /// 1. Acquiring an exclusive lock to prevent concurrent environment modifications
    /// 2. Saving the current value of the environment variable
    /// 3. Setting or removing the environment variable as specified
    /// 4. Executing the provided test function
    /// 5. Restoring the original environment variable state
    ///
    /// # Parameters
    ///
    /// * `key` - The name of the environment variable to modify
    /// * `val` - The value to set for the environment variable. If `Some(value)`, the variable
    ///   is set to that value. If `None`, the variable is removed from the environment.
    /// * `f` - A closure containing the test code to execute with the modified environment variable
    fn with_env_var(key: &str, val: Option<&str>, f: impl FnOnce()) {
        let _guard = env_lock().lock().unwrap();
        let prev = env::var(key).ok();
        match val {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key), // tests when the variable is not set
        }
        f();
        match prev {
            Some(v) => env::set_var(key, v),
            None => env::remove_var(key),
        }
    }

    struct Context {
        _tempdir: tempfile::TempDir,
        filename: PathBuf,
    }

    fn write_temp_ssh_config(contents: &str) -> Context {
        let tempdir = tempfile::tempdir().unwrap();
        let unique = format!(
            "sshconfig_test_{}_{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let filename = tempdir.path().join(unique);
        let mut file = std::fs::File::create(&filename).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        Context {
            _tempdir: tempdir,
            filename,
        }
    }

    #[test]
    fn validate_ok_with_valid_config() {
        let context = write_temp_ssh_config("Host example\n  HostName example.com\n");
        let ssh_config = SSHConfig {
            config_file: Some(context.filename.to_string_lossy().to_string()),
        };

        let result = ssh_config.validate();
        assert!(result.is_ok());
        assert!(matches!(result, Ok(_)));
    }

    #[test]
    fn validate_ok_with_no_config_file() {
        let ssh_config = SSHConfig { config_file: None };
        assert!(ssh_config.validate().is_ok());
    }

    #[test]
    fn validate_err_with_invalid_config() {
        let context = write_temp_ssh_config("Contents that are not valid ssh config contents\n");
        let ssh_config = SSHConfig {
            config_file: Some(context.filename.to_string_lossy().to_string()),
        };
        let result = ssh_config.validate();
        assert!(matches!(result, Err(_)));
        let pattern =
            Regex::new(r"Failed to parse SSH config file \S+: unknown field: Contents").unwrap();
        assert!(pattern.is_match(&result.unwrap_err().to_string()));
    }

    #[test]
    fn parse_returns_config_when_present() {
        let context = write_temp_ssh_config("Host example\n  HostName example.com\n");
        let ssh_config = SSHConfig {
            config_file: Some(context.filename.to_string_lossy().to_string()),
        };

        let result = ssh_config.parse();
        assert!(matches!(result, Ok(Some(_))));
    }

    #[test]
    fn parse_returns_none_when_missing() {
        let ssh_config = SSHConfig { config_file: None };
        assert!(matches!(ssh_config.parse(), Ok(None)));
    }

    #[test]
    fn ensure_exists_returns_ok_when_present() {
        let context = write_temp_ssh_config("Host example\n  HostName example.com\n");
        let ssh_config = SSHConfig {
            config_file: Some(context.filename.to_string_lossy().to_string()),
        };

        let result = ssh_config.ensure_exists(&context.filename);
        assert!(result.is_ok());
    }

    #[test]
    fn ensure_exists_returns_err_when_missing() {
        let ssh_config = SSHConfig { config_file: None };
        let result = ssh_config.ensure_exists(&Path::new("nonexistent_file.txt"));
        assert!(matches!(result, Err(_)));
        assert_eq!(
            result.unwrap_err().to_string(),
            "SSH config file not found: nonexistent_file.txt"
        );
    }

    #[test]
    fn options_config_default_is_all_none() {
        let options = OptionsConfig::default();
        assert!(options.hosts_file.is_none());
        assert!(options.groups_file.is_none());
        assert!(options.defaults_file.is_none());
    }

    #[test]
    fn options_config_deserializes_empty_object_to_none() {
        let options: OptionsConfig = serde_json::from_str("{}").unwrap();
        assert!(options.hosts_file.is_none());
        assert!(options.groups_file.is_none());
        assert!(options.defaults_file.is_none());
    }

    #[test]
    fn options_config_deserializes_with_values() {
        let json = r#"{
            "hosts_file": "/tmp/hosts.yaml",
            "groups_file": "/tmp/groups.yaml",
            "defaults_file": "/tmp/defaults.yaml"
        }"#;
        let options: OptionsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(options.hosts_file.as_deref(), Some("/tmp/hosts.yaml"));
        assert_eq!(options.groups_file.as_deref(), Some("/tmp/groups.yaml"));
        assert_eq!(options.defaults_file.as_deref(), Some("/tmp/defaults.yaml"));
    }

    #[test]
    fn runner_config_default_values() {
        let runner = RunnerConfig::default();
        assert_eq!(runner.plugin, "threaded");
        assert_eq!(runner.options, json!({"num_of_workers": 10}));
        assert_eq!(runner.max_task_depth, 10);
    }

    #[test]
    fn runner_config_deserializes_empty_object_to_defaults() {
        let runner: RunnerConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(runner.plugin, "threaded");
        assert_eq!(runner.options, json!({"num_of_workers": 10}));
        assert_eq!(runner.max_task_depth, 10);
    }

    #[test]
    fn runner_config_deserializes_with_values() {
        let json = r#"{
            "plugin": "custom",
            "options": {"num_of_workers": 3, "queue": "fast"},
            "max_task_depth": 5
        }"#;
        let runner: RunnerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(runner.plugin, "custom");
        assert_eq!(
            runner.options,
            json!({"num_of_workers": 3, "queue": "fast"})
        );
        assert_eq!(runner.max_task_depth, 5);
    }

    #[test]
    fn parse_bool_loose_accepts_common_values() {
        assert_eq!(super::parse_bool_loose("true"), Some(true));
        assert_eq!(super::parse_bool_loose("TrUe"), Some(true));
        assert_eq!(super::parse_bool_loose("1"), Some(true));
        assert_eq!(super::parse_bool_loose("yes"), Some(true));
        assert_eq!(super::parse_bool_loose("on"), Some(true));
        assert_eq!(super::parse_bool_loose("false"), Some(false));
        assert_eq!(super::parse_bool_loose("0"), Some(false));
        assert_eq!(super::parse_bool_loose("no"), Some(false));
        assert_eq!(super::parse_bool_loose("off"), Some(false));
        assert_eq!(super::parse_bool_loose("maybe"), None);
    }

    #[test]
    fn deserialize_bool_loose_from_string_and_bool() {
        #[derive(serde::Deserialize)]
        struct T {
            #[serde(deserialize_with = "super::deserialize_bool_loose")]
            v: bool,
        }

        let t: T = serde_json::from_str(r#"{ "v": "yes" }"#).unwrap();
        assert!(t.v);
        let t: T = serde_json::from_str(r#"{ "v": false }"#).unwrap();
        assert!(!t.v);
    }

    #[test]
    fn deserialize_bool_loose_rejects_invalid_string() {
        #[derive(serde::Deserialize, Debug)]
        struct T {
            #[serde(deserialize_with = "super::deserialize_bool_loose")]
            _v: bool,
        }

        let err = serde_json::from_str::<T>(r#"{ "_v": "maybe" }"#).unwrap_err();
        assert!(err.to_string().contains("invalid boolean value"));
    }

    #[test]
    fn raise_on_error_uses_env_and_fallbacks() {
        with_env_var(super::ENV_RAISE_ON_ERROR, Some("true"), || {
            assert!(super::raise_on_error());
        });
        with_env_var(super::ENV_RAISE_ON_ERROR, Some("not_a_bool"), || {
            assert!(!super::raise_on_error());
        });
        with_env_var(super::ENV_RAISE_ON_ERROR, None, || {
            assert!(!super::raise_on_error());
        });
    }

    #[test]
    fn get_log_to_console_default_parses_env() {
        with_env_var(super::ENV_LOG_TO_CONSOLE, Some("yes"), || {
            assert!(super::get_log_to_console_default());
        });
        with_env_var(super::ENV_LOG_TO_CONSOLE, Some("no"), || {
            assert!(!super::get_log_to_console_default());
        });
    }

    #[test]
    fn env_string_defaults_respect_env_and_fallbacks() {
        with_env_var(super::ENV_INVENTORY_PLUGIN, Some("CustomInv"), || {
            assert_eq!(super::get_inventory_plugin_config(), "CustomInv");
        });
        with_env_var(super::ENV_INVENTORY_PLUGIN, None, || {
            assert_eq!(super::get_inventory_plugin_config(), "FileInventoryPlugin");
        });

        with_env_var(super::ENV_RUNNER_PLUGIN, Some("CustomRunner"), || {
            assert_eq!(super::get_runner_plugin_default(), "CustomRunner");
        });
        with_env_var(super::ENV_RUNNER_PLUGIN, None, || {
            assert_eq!(super::get_runner_plugin_default(), "threaded");
        });

        with_env_var(super::ENV_LOG_LEVEL, Some("debug"), || {
            assert_eq!(super::get_log_level_default(), "debug");
        });
        with_env_var(super::ENV_LOG_LEVEL, None, || {
            assert_eq!(super::get_log_level_default(), "info");
        });
    }

    #[test]
    fn get_default_log_file_prefers_env() {
        with_env_var(super::ENV_LOG_FILE, Some("/tmp/genja-test.log"), || {
            assert_eq!(super::get_default_log_file(), "/tmp/genja-test.log");
        });
    }

    #[test]
    fn get_default_log_file_uses_cwd_when_env_missing() {
        let _guard = env_lock().lock().unwrap();
        let prev = env::var(super::ENV_LOG_FILE).ok();
        env::remove_var(super::ENV_LOG_FILE);

        let tempdir = tempfile::tempdir().unwrap();
        let prev_dir = env::current_dir().unwrap();
        env::set_current_dir(tempdir.path()).unwrap();

        let expected = tempdir.path().join("genja.log");
        assert_eq!(
            super::get_default_log_file(),
            expected.to_string_lossy().to_string()
        );

        env::set_current_dir(prev_dir).unwrap();
        match prev {
            Some(v) => env::set_var(super::ENV_LOG_FILE, v),
            None => env::remove_var(super::ENV_LOG_FILE),
        }
    }

    #[test]
    fn settings_from_file_errors_when_missing() {
        let tempdir = tempfile::tempdir().unwrap();
        let missing = tempdir.path().join("missing.yaml");
        let err = super::Settings::from_file(missing.to_string_lossy().as_ref()).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("not found"));
    }

    #[test]
    fn settings_from_file_errors_on_unsupported_extension() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("config.txt");
        std::fs::write(&file_path, "{}").unwrap();
        let err = super::Settings::from_file(file_path.to_string_lossy().as_ref()).unwrap_err();
        assert!(err.to_string().contains("Unsupported file format"));
    }

    #[test]
    fn settings_from_file_errors_on_invalid_json() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("config.json");
        std::fs::write(&file_path, "{ not valid json").unwrap();
        let err = super::Settings::from_file(file_path.to_string_lossy().as_ref()).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn settings_from_file_uses_defaults_for_empty_file() {
        let _guard = env_lock().lock().unwrap();
        let keys = [
            super::ENV_RAISE_ON_ERROR,
            super::ENV_INVENTORY_PLUGIN,
            super::ENV_RUNNER_PLUGIN,
            super::ENV_LOG_LEVEL,
            super::ENV_LOG_FILE,
            super::ENV_LOG_TO_CONSOLE,
        ];

        let prev: Vec<(String, Option<String>)> = keys
            .iter()
            .map(|key| (key.to_string(), env::var(key).ok()))
            .collect();

        for key in keys {
            env::remove_var(key);
        }

        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("config.yaml");
        std::fs::write(&file_path, "{}").unwrap();
        let settings = super::Settings::from_file(file_path.to_string_lossy().as_ref()).unwrap();

        assert!(!settings.core().raise_on_error());
        assert_eq!(settings.inventory().plugin(), "FileInventoryPlugin");
        assert_eq!(settings.runner().plugin(), "threaded");
        assert_eq!(settings.logging().level(), "info");
        assert!(settings.logging().enabled());

        for (key, val) in prev {
            match val {
                Some(v) => env::set_var(&key, v),
                None => env::remove_var(&key),
            }
        }
    }

    #[test]
    fn inventory_loads_empty_files() {
        let tempdir = tempfile::tempdir().unwrap();
        let hosts_path = tempdir.path().join("hosts.json");
        let groups_path = tempdir.path().join("groups.json");
        let defaults_path = tempdir.path().join("defaults.json");
        std::fs::write(&hosts_path, "{}").unwrap();
        std::fs::write(&groups_path, "{}").unwrap();
        std::fs::write(&defaults_path, "{}").unwrap();

        let options = super::OptionsConfig::builder()
            .hosts_file(hosts_path.to_string_lossy().as_ref())
            .groups_file(groups_path.to_string_lossy().as_ref())
            .defaults_file(defaults_path.to_string_lossy().as_ref())
            .build();
        let config = super::InventoryConfig::builder().options(options).build();

        let (hosts, groups, defaults) = config.load_inventory_files().unwrap();
        assert!(hosts.is_empty());
        assert!(groups.unwrap().is_empty());
        // let defaults = defaults.unwrap();
        assert!(defaults.unwrap().is_empty());
        // assert!(defaults
        //     .as_object()
        //     .map(|map| map.is_empty())
        //     .unwrap_or(false));
    }

    #[test]
    fn inventory_load_errors_on_missing_file() {
        let tempdir = tempfile::tempdir().unwrap();
        let missing = tempdir.path().join("missing.json");
        let options = super::OptionsConfig::builder()
            .hosts_file(missing.to_string_lossy().as_ref())
            .build();
        let config = super::InventoryConfig::builder().options(options).build();
        let err = config.load_inventory_files().unwrap_err();
        assert!(err.contains("Failed to read file"));
    }

    #[test]
    fn inventory_load_errors_on_unsupported_extension() {
        let tempdir = tempfile::tempdir().unwrap();
        let file_path = tempdir.path().join("hosts.txt");
        std::fs::write(&file_path, "{}").unwrap();
        let options = super::OptionsConfig::builder()
            .hosts_file(file_path.to_string_lossy().as_ref())
            .build();
        let config = super::InventoryConfig::builder().options(options).build();
        let err = config.load_inventory_files().unwrap_err();
        assert!(err.contains("Unsupported file format"));
    }
}
