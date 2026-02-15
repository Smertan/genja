use crate::inventory::{Defaults, Groups, Hosts};
use config::{Config as ConfigBuilder, ConfigError, File, FileFormat};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use ssh2_config::{ParseRule, SshConfig};
use std::env;
use std::fs::File as StdFile;
use std::io::{BufReader, ErrorKind};
use std::path::{Path, PathBuf};

// Environment variable names
const ENV_RAISE_ON_ERROR: &str = "GENJA_CORE_RAISE_ON_ERROR";
const ENV_INVENTORY_PLUGIN: &str = "GENJA_INVENTORY_PLUGIN";
const ENV_RUNNER_PLUGIN: &str = "GENJA_RUNNER_PLUGIN";
const ENV_LOG_LEVEL: &str = "GENJA_LOGGING_LEVEL";
const ENV_LOG_FILE: &str = "GENJA_LOGGING_LOG_FILE";
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

fn get_inventory_plugin_config() -> String {
    env::var(ENV_INVENTORY_PLUGIN).unwrap_or_else(|_err| String::from("FileInventoryPlugin"))
}

fn get_runner_plugin_default() -> String {
    env::var(ENV_RUNNER_PLUGIN).unwrap_or_else(|_err| String::from("threaded"))
}

fn get_runner_options_default() -> serde_json::Value {
    serde_json::json!({
        "num_of_workers": 10
    })
}

fn get_log_level_default() -> String {
    env::var(ENV_LOG_LEVEL).unwrap_or_else(|_err| String::from("info"))
}

fn get_log_to_console_default() -> bool {
    match env::var(ENV_LOG_TO_CONSOLE) {
        Ok(val) => parse_bool_loose(val.as_str()).unwrap_or(false),
        Err(_) => false,
    }
}
fn get_default_log_file() -> String {
    match env::var(ENV_LOG_FILE) {
        Ok(val) => val,
        Err(_) => {
            if let Ok(output) = std::process::Command::new("cargo")
                .args(["metadata", "--format-version", "1", "--no-deps"])
                .output()
            {
                if output.status.success() {
                    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                        if let Some(root) = value.get("workspace_root").and_then(|v| v.as_str()) {
                            return PathBuf::from(root)
                                .join("genja.log")
                                .to_string_lossy()
                                .to_string();
                        }
                    }
                }
            }

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
/// # Examples
///
/// ```
/// use genja_core::settings::{OptionsConfig, OptionsConfigBuilder};
///
/// // Create with default values (all None)
///
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
        OptionsConfigBuilder::new()
    }
}

pub struct OptionsConfigBuilder {
    hosts_file: Option<String>,
    groups_file: Option<String>,
    defaults_file: Option<String>,
}

impl OptionsConfigBuilder {
    pub fn new() -> Self {
        Self {
            hosts_file: None,
            groups_file: None,
            defaults_file: None,
        }
    }

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
        Self::new()
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
///   or "FileInventoryPlugin" if not set.
/// * `options` - Configuration options specifying the file paths for hosts, groups,
///   and defaults inventory files.
/// * `transform_function` - Optional name of a transformation function to apply to
///   the loaded inventory data. This allows custom processing of inventory before use.
/// * `transform_function_options` - Optional JSON configuration passed to the
///   transformation function, allowing parameterized transformations.
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
    transform_function_options: Option<serde_json::Value>,
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
        InventoryConfigBuilder::new()
    }
}

pub struct InventoryConfigBuilder {
    plugin: Option<String>,
    options: Option<OptionsConfig>,
    transform_function: Option<String>,
    transform_function_options: Option<serde_json::Value>,
}

impl InventoryConfigBuilder {
    pub fn new() -> Self {
        Self {
            plugin: None,
            options: None,
            transform_function: None,
            transform_function_options: None,
        }
    }

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

    pub fn transform_function_options(mut self, options: serde_json::Value) -> Self {
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
        Self::new()
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
    // TODO: Check last tabnine session
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
        CoreConfigBuilder::new()
    }
}

pub struct CoreConfigBuilder {
    raise_on_error: Option<bool>,
}

impl CoreConfigBuilder {
    pub fn new() -> Self {
        Self { raise_on_error: None }
    }

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
        Self::new()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct SSHConfig {
    config_file: Option<String>,
}

impl SSHConfig {
    fn new() -> Self {
        SSHConfig { config_file: None }
    }
    /// Validates the SSH config file syntax if a path is provided.
    /// Returns Ok(()) if the file exists and can be parsed or
    /// Err(e) if the file does not exist or cannot be parsed.
    ///
    /// If the SSH config file is not specified, this method returns Ok(()).
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
            // Ok(())
        } else {
            Ok(()) // No config file specified, nothing to validate
        }
    }

    /// Parses and returns the SSH config if a path is provided
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
        SSHConfig::new()
    }
}

impl SSHConfig {
    pub fn builder() -> SSHConfigBuilder {
        SSHConfigBuilder::new()
    }
}

pub struct SSHConfigBuilder {
    config_file: Option<String>,
}

impl SSHConfigBuilder {
    pub fn new() -> Self {
        Self { config_file: None }
    }

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
        Self::new()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct RunnerConfig {
    pub plugin: String,
    // #[serde(default = "get_runner_options_default")]_runner_options_default")]
    pub options: serde_json::Value,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            plugin: get_runner_plugin_default(),
            options: get_runner_options_default(),
        }
    }
}

impl RunnerConfig {
    pub fn builder() -> RunnerConfigBuilder {
        RunnerConfigBuilder::new()
    }
}

pub struct RunnerConfigBuilder {
    plugin: Option<String>,
    options: Option<serde_json::Value>,
}

impl RunnerConfigBuilder {
    pub fn new() -> Self {
        Self {
            plugin: None,
            options: None,
        }
    }

    pub fn plugin(mut self, plugin: impl Into<String>) -> Self {
        self.plugin = Some(plugin.into());
        self
    }

    pub fn options(mut self, options: serde_json::Value) -> Self {
        self.options = Some(options);
        self
    }

    pub fn build(self) -> RunnerConfig {
        RunnerConfig {
            plugin: self.plugin.unwrap_or_else(get_runner_plugin_default),
            options: self.options.unwrap_or_else(get_runner_options_default),
        }
    }
}

impl Default for RunnerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Stores the logging configuration for Genja.
///
/// If the user does not specify a logging configuration in their config file,
/// the default values will be used.
///
/// **Note:** Genja does not initialize logging itself. The user must configure
/// the logging subscriber in their application code. See the documentation in
/// `lib.rs` for examples of how to set up logging using these configuration values.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct LoggingConfig {
    #[serde(deserialize_with = "deserialize_bool_loose")]
    pub enabled: bool,
    pub level: String,
    pub log_file: String,
    #[serde(deserialize_with = "deserialize_bool_loose")]
    pub to_console: bool,
    pub file_size: u64,
    pub max_file_count: usize,
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
        LoggingConfigBuilder::new()
    }
}

pub struct LoggingConfigBuilder {
    enabled: Option<bool>,
    level: Option<String>,
    log_file: Option<String>,
    to_console: Option<bool>,
    file_size: Option<u64>,
    max_file_count: Option<usize>,
}

impl LoggingConfigBuilder {
    pub fn new() -> Self {
        Self {
            enabled: None,
            level: None,
            log_file: None,
            to_console: None,
            file_size: None,
            max_file_count: None,
        }
    }

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
        Self::new()
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    // #[serde(default = "CoreConfig::default")]
    pub core: CoreConfig,
    pub inventory: InventoryConfig,
    pub ssh: SSHConfig,
    pub runner: RunnerConfig,
    pub logging: LoggingConfig,
}

impl Settings {
    fn new() -> Self {
        Self {
            core: CoreConfig::default(),
            inventory: InventoryConfig::default(),
            ssh: SSHConfig::default(),
            runner: RunnerConfig::default(),
            logging: LoggingConfig::default(),
        }
    }

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
            .add_source(File::new(file_path, format).required(false))
            .build()
            .unwrap();
        let parsed_config: Settings = config.try_deserialize()?;

        // Validate SSH config syntax if provided
        if let Err(e) = parsed_config.ssh.validate() {
            return Err(ConfigError::Message(e));
        }
        Ok(parsed_config)
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    pub fn builder() -> SettingsBuilder {
        SettingsBuilder::new()
    }
}

pub struct SettingsBuilder {
    core: Option<CoreConfig>,
    inventory: Option<InventoryConfig>,
    ssh: Option<SSHConfig>,
    runner: Option<RunnerConfig>,
    logging: Option<LoggingConfig>,
}

impl SettingsBuilder {
    pub fn new() -> Self {
        Self {
            core: None,
            inventory: None,
            ssh: None,
            runner: None,
            logging: None,
        }
    }

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
        Self::new()
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
    }

    #[test]
    fn runner_config_deserializes_empty_object_to_defaults() {
        let runner: RunnerConfig = serde_json::from_str("{}").unwrap();
        assert_eq!(runner.plugin, "threaded");
        assert_eq!(runner.options, json!({"num_of_workers": 10}));
    }

    #[test]
    fn runner_config_deserializes_with_values() {
        let json = r#"{
            "plugin": "custom",
            "options": {"num_of_workers": 3, "queue": "fast"}
        }"#;
        let runner: RunnerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(runner.plugin, "custom");
        assert_eq!(
            runner.options,
            json!({"num_of_workers": 3, "queue": "fast"})
        );
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
}
