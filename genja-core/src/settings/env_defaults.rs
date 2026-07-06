use serde::Deserialize;
use std::env;
use std::path::PathBuf;

/// Environment variable name for controlling error handling behavior.
///
/// When set, this variable determines whether Genja should raise (panic/abort) on errors
/// or handle them gracefully. Accepts boolean-like values such as "true", "false", "yes",
/// "no", "1", "0", "on", "off" (case-insensitive).
///
/// Default: `false` (errors are handled gracefully)
pub(super) const ENV_RAISE_ON_ERROR: &str = "GENJA_CORE_RAISE_ON_ERROR";

/// Environment variable name for specifying the inventory plugin.
///
/// This variable determines which inventory plugin implementation should be used
/// for loading and managing host inventory data.
///
/// Default: `"FileInventoryPlugin"`
pub(super) const ENV_INVENTORY_PLUGIN: &str = "GENJA_INVENTORY_PLUGIN";

/// Environment variable name for specifying the runner plugin.
///
/// This variable determines which runner plugin implementation should be used
/// for executing tasks across hosts (e.g., "threaded", "serial").
///
/// Default: `"threaded"`
pub(super) const ENV_RUNNER_PLUGIN: &str = "GENJA_RUNNER_PLUGIN";

/// Environment variable name for setting the logging level.
///
/// This variable controls the verbosity of log output. Valid values include
/// "trace", "debug", "info", "warn", and "error".
///
/// Default: `"info"`
pub(super) const ENV_LOG_LEVEL: &str = "GENJA_LOGGING_LEVEL";

/// Environment variable name for specifying the log file path.
///
/// This variable determines where log output should be written. If not set,
/// logs are written to `genja.log` in the current working directory.
///
/// Default: `genja.log` in the current working directory
pub(super) const ENV_LOG_FILE: &str = "GENJA_LOGGING_LOG_FILE";

/// Environment variable name for enabling console logging.
///
/// When set, this variable determines whether logs should be written to the console
/// in addition to the log file. Accepts boolean-like values such as "true", "false",
/// "yes", "no", "1", "0", "on", "off" (case-insensitive).
///
/// Default: `false` (console logging disabled)
pub(super) const ENV_LOG_TO_CONSOLE: &str = "GENJA_LOGGING_TO_CONSOLE";

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
pub(super) fn parse_bool_loose(s: &str) -> Option<bool> {
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

pub(super) fn deserialize_bool_loose<'de, D>(deserializer: D) -> Result<bool, D::Error>
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

pub(super) fn raise_on_error() -> bool {
    match std::env::var(ENV_RAISE_ON_ERROR) {
        Ok(s) => match parse_bool_loose(s.as_str()) {
            Some(value) => value,
            None => {
                log::warn!("Invalid {ENV_RAISE_ON_ERROR} value {s:?}; using default false");
                false
            }
        },
        Err(_) => false,
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
pub(super) fn get_inventory_plugin_config() -> String {
    env::var(ENV_INVENTORY_PLUGIN).unwrap_or_else(|_err| String::from("FileInventoryPlugin"))
}

/// Returns the default runner plugin from `GENJA_RUNNER_PLUGIN`, or "threaded".
///
/// See tests in this module for behavioral verification.
pub(super) fn get_runner_plugin_default() -> String {
    env::var(ENV_RUNNER_PLUGIN).unwrap_or_else(|_err| String::from("threaded"))
}

/// Returns the default runner options JSON.
///
/// See tests in this module for behavioral verification.
pub(super) fn get_runner_options_default() -> serde_json::Value {
    serde_json::json!({})
}

/// Returns the default max task depth for runner execution.
///
/// See tests in this module for behavioral verification.
pub(super) fn get_runner_max_task_depth_default() -> usize {
    10
}

/// Returns the default maximum number of connection attempts for runner execution.
///
/// See tests in this module for behavioral verification.
pub(super) fn get_runner_max_connection_attempts_default() -> usize {
    3
}

/// Returns the default task retry authorization for runner execution.
pub(super) fn get_runner_retry_allow_default() -> bool {
    false
}

/// Returns the default maximum number of task attempts for runner execution.
pub(super) fn get_runner_retry_max_attempts_default() -> usize {
    1
}

/// Returns the default fixed task retry delay in milliseconds for runner execution.
pub(super) fn get_runner_retry_delay_ms_default() -> u64 {
    0
}

/// Returns the default log level from `GENJA_LOGGING_LEVEL`, or "info".
///
/// See tests in this module for behavioral verification.
pub(super) fn get_log_level_default() -> String {
    env::var(ENV_LOG_LEVEL).unwrap_or_else(|_err| String::from("info"))
}

/// Returns the default console logging flag from `GENJA_LOGGING_TO_CONSOLE`.
///
/// See tests in this module for behavioral verification.
pub(super) fn get_log_to_console_default() -> bool {
    match env::var(ENV_LOG_TO_CONSOLE) {
        Ok(val) => match parse_bool_loose(val.as_str()) {
            Some(value) => value,
            None => {
                log::warn!("Invalid {ENV_LOG_TO_CONSOLE} value {val:?}; using default false");
                false
            }
        },
        Err(_) => false,
    }
}

/// Returns the default log file path, preferring `GENJA_LOGGING_LOG_FILE` when set.
///
/// When the environment variable is not set, defaults to `genja.log` in the
/// current working directory.
///
/// See tests in this module for behavioral verification.
pub(super) fn get_default_log_file() -> String {
    match env::var(ENV_LOG_FILE) {
        Ok(val) => val,
        Err(_) => {
            let start_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            start_dir.join("genja.log").to_string_lossy().to_string()
        }
    }
}
