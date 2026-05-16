use super::env_defaults::{
    deserialize_bool_loose, get_default_log_file, get_log_level_default,
    get_log_to_console_default,
};
use serde::{Deserialize, Serialize};

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
            file_size: 1024 * 1024 * 10,
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
