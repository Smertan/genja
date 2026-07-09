use super::{CoreConfig, InventoryConfig, LoggingConfig, RunnerConfig, SSHConfig};
use crate::ConfigLoadError;
use config::{Config as ConfigBuilder, File, FileFormat};
use serde::{Deserialize, Serialize};

/// Top-level Genja configuration.
///
/// Missing sections deserialize to their defaults.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    core: CoreConfig,
    inventory: InventoryConfig,
    ssh: SSHConfig,
    runner: RunnerConfig,
    logging: LoggingConfig,
}

impl Settings {
    /// Loads settings from a `.json`, `.yaml`, or `.yml` file.
    ///
    /// Missing fields fall back to defaults. If an SSH config path is configured,
    /// it is validated after deserialization.
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

impl Settings {
    pub fn builder() -> SettingsBuilder {
        SettingsBuilder::default()
    }
}

/// Builder for `Settings`.
#[derive(Default)]
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
