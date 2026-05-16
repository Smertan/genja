use super::{CoreConfig, InventoryConfig, LoggingConfig, RunnerConfig, SSHConfig};
use crate::ConfigLoadError;
use config::{Config as ConfigBuilder, File, FileFormat};
use serde::{Deserialize, Serialize};

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
