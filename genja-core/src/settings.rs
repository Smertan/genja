use config::{Config as ConfigBuilder, ConfigError, File, FileFormat};
use serde::{Deserialize, Serialize};


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
pub struct Settings {
    // #[serde(default = "CoreConfig::default")]
    core: CoreConfig,
    inventory: InventoryConfig,
}

fn raise_on_error() -> bool {
    true
}

fn get_runner_config() -> String {
    String::from("threaded")
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
        //     logging: LoggingConfig::default(),
        //     ssh: SSHConfig::default(),
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
        config.try_deserialize()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}