use config::{Config as ConfigBuilder, ConfigError, File, FileFormat};
use serde::{Deserialize, Serialize};
use ssh2_config::{ParseRule, SshConfig};
use std::fs::File as StdFile;
use std::io::{BufReader, ErrorKind};
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

#[cfg(test)]
mod tests {
    use super::{OptionsConfig, SSHConfig};
    use regex::Regex;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

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
}
