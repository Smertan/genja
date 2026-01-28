use config::{Config as ConfigBuilder, ConfigError, File, FileFormat};
use serde::{Deserialize, Serialize};
use ssh2_config::{ParseRule, SshConfig};
use std::fs::File as StdFile;
use std::io::BufReader;
use std::path::Path;

fn raise_on_error() -> bool {
    true
}

fn get_runner_config() -> String {
    String::from("threaded")
}

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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct InventoryConfig {
    #[serde(default = "get_runner_config")]
    plugin: String,
    options: OptionsConfig,
}

impl Default for InventoryConfig {
    fn default() -> Self {
        InventoryConfig {
            plugin: get_runner_config(),
            options: OptionsConfig::default(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CoreConfig {
    #[serde(default = "raise_on_error")]
    raise_on_error: bool,
}

impl Default for CoreConfig {
    fn default() -> Self {
        CoreConfig {
            raise_on_error: raise_on_error(),
        }
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

            path.try_exists()
                .expect(format!("SSH config file not found: {:?}", path).as_str());

            let inner = match StdFile::open(path) {
                Ok(file) => file,
                Err(e) => return Err(format!("Failed to open SSH config file {:?}: {}", path, e)),
            };
            let mut reader = BufReader::new(inner);
            // .expect("Could not open configuration file");

            match SshConfig::default().parse(&mut reader, ParseRule::STRICT) {
                Ok(_) => return Ok(()),
                Err(e) => return Err(format!("Failed to parse SSH config file {:?}: {}", path, e)),
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
                Err(e) => return Err(format!("Failed to open SSH config file {:?}: {}", path, e)),
            };
            let mut reader = BufReader::new(file);

            match SshConfig::default().parse(&mut reader, ParseRule::STRICT) {
                Ok(config) => Ok(Some(config)),
                Err(e) => Err(format!("Failed to parse SSH config file {:?}: {}", path, e)),
            }
        } else {
            Ok(None)
        }
    }
}

impl Default for SSHConfig {
    fn default() -> Self {
        SSHConfig::new()
    }
}

// #[derive(Deserialize, Serialize, Clone, Debug)]
// #[serde(default)]
// pub struct SSHConfig {
//     config_file: Option<String>,
// }

// impl SSHConfig {
//     fn new() -> Self {
//         SSHConfig {
//             config_file: None,
//         }
//     }
// }

// impl Default for SSHConfig {
//     fn default() -> Self {
//         SSHConfig::new()
//     }
// }

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    // #[serde(default = "CoreConfig::default")]
    core: CoreConfig,
    inventory: InventoryConfig,
    ssh: SSHConfig,
    // logging: LoggingConfig,
    // runner: RunnerConfig
}

// #[derive(Deserialize, Serialize, Clone, Debug)]
// #[serde(default)]
// pub struct Config {

//     core_config: CoreConfig,
//     inventory_config: InventoryConfig,
//     // pub inventory: InventoryConfig,
//     // pub logging: LoggingConfig,
//     // pub ssh: SSHConfig,
//     // pub runner: RunnerConfig
// }

impl Settings {
    fn new() -> Self {
        Self {
            core: CoreConfig::default(),
            inventory: InventoryConfig::default(),
            ssh: SSHConfig::default(),
            // logging: LoggingConfig::default(),
            //     runner: RunnerConfig::default(),
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
            // .add_source(File::new(file_path, FileFormat::Yaml).required(false))
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
