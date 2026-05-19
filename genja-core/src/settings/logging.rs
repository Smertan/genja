use super::env_defaults::{
    deserialize_bool_loose, get_default_log_file, get_log_level_default, get_log_to_console_default,
};
use serde::{Deserialize, Serialize};

/// Logging settings consumed by applications embedding Genja.
///
/// Genja does not install a logging subscriber itself. Environment-backed
/// defaults are used for `level`, `log_file`, and `to_console`, and the boolean
/// fields accept the module's loose boolean forms.
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

/// Builder for `LoggingConfig`.
#[derive(Default)]
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
