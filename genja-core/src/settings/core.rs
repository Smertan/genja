use super::env_defaults::{deserialize_bool_loose, raise_on_error};
use serde::{Deserialize, Serialize};

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
