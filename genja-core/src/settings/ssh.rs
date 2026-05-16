use serde::{Deserialize, Serialize};

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
    pub(super) config_file: Option<String>,
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
