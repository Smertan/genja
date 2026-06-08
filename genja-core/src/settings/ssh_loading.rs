use super::SSHConfig;
use crate::SshConfigError;
use ssh2_config::{ParseRule, SshConfig};
use std::fs::File as StdFile;
use std::io::{BufReader, ErrorKind};
use std::path::Path;

impl SSHConfig {
    /// Validates the SSH configuration file syntax if a path is provided.
    ///
    /// This method performs comprehensive validation of an SSH configuration file by:
    /// 1. Verifying that the file exists and is accessible
    /// 2. Opening the file for reading
    /// 3. Parsing the file contents using strict SSH config syntax rules
    ///
    /// If no SSH configuration file is specified (the `config_file` field is `None`),
    /// this method returns `Ok(())` without performing any validation.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if:
    /// * No config file is specified (nothing to validate)
    /// * The config file exists, can be opened, and contains valid SSH configuration syntax
    ///
    /// Returns `Err(SshConfigError)` if:
    /// * The specified file does not exist or cannot be accessed
    /// * The file cannot be opened due to permission issues or other I/O errors
    /// * The file contents cannot be parsed as valid SSH configuration syntax
    ///
    /// # Errors
    ///
    /// This method returns an error in the following cases:
    /// * File existence check fails (see `ensure_exists` for details)
    /// * `SshConfigError::OpenFailed` - The file exists but cannot be opened
    /// * `SshConfigError::ParseFailed` - The file contains invalid SSH config syntax
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::settings::SSHConfig;
    ///
    /// let config = SSHConfig::builder()
    ///     .config_file("/home/user/.ssh/config")
    ///     .build();
    ///
    /// match config.validate() {
    ///     Ok(()) => println!("SSH config is valid"),
    ///     Err(e) => eprintln!("Invalid SSH config: {}", e),
    /// }
    /// ```
    pub fn validate(&self) -> Result<(), SshConfigError> {
        if let Some(ref path) = self.config_file {
            let path = Path::new(path);

            self.ensure_exists(path)?;

            let file = match StdFile::open(path) {
                Ok(file) => file,
                Err(e) => {
                    return Err(SshConfigError::OpenFailed {
                        path: path.display().to_string(),
                        message: e.to_string(),
                    });
                }
            };
            let mut reader = BufReader::new(file);

            match SshConfig::default().parse(&mut reader, ParseRule::STRICT) {
                Ok(_) => Ok(()),
                Err(e) => Err(SshConfigError::ParseFailed {
                    path: path.display().to_string(),
                    message: e.to_string(),
                }),
            }
        } else {
            Ok(())
        }
    }

    /// Parses the SSH configuration file and returns the parsed configuration.
    ///
    /// This method reads and parses an SSH configuration file if one is specified in the
    /// `config_file` field. The parsing follows strict SSH config file syntax rules as
    /// defined by OpenSSH. If no configuration file is specified, the method returns
    /// `Ok(None)` without performing any parsing.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing:
    /// * `Ok(Some(SshConfig))` - If a config file is specified and successfully parsed,
    ///   containing the parsed SSH configuration with all host entries and settings.
    /// * `Ok(None)` - If no config file is specified (the `config_file` field is `None`).
    /// * `Err(SshConfigError)` - If an error occurs during parsing.
    ///
    /// # Errors
    ///
    /// Returns [`SshConfigError`] if:
    /// * The specified SSH config file does not exist at the given path
    /// * The file cannot be opened due to permission issues or other I/O errors
    /// * The file contents cannot be parsed as valid SSH configuration syntax
    /// * The file contains syntax errors or invalid SSH configuration directives
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja_core::settings::SSHConfig;
    ///
    /// let config = SSHConfig::builder()
    ///     .config_file("/home/user/.ssh/config")
    ///     .build();
    ///
    /// match config.parse() {
    ///     Ok(Some(ssh_config)) => {
    ///         println!("Successfully parsed SSH config");
    ///     }
    ///     Ok(None) => {
    ///         println!("No SSH config file specified");
    ///     }
    ///     Err(e) => {
    ///         eprintln!("Failed to parse SSH config: {}", e);
    ///     }
    /// }
    /// ```
    pub fn parse(&self) -> Result<Option<SshConfig>, SshConfigError> {
        if let Some(ref path) = self.config_file {
            let path = Path::new(path);

            self.ensure_exists(path)?;

            let file = match StdFile::open(path) {
                Ok(file) => file,
                Err(e) => Err(SshConfigError::OpenFailed {
                    path: path.display().to_string(),
                    message: e.to_string(),
                })?,
            };
            let mut reader = BufReader::new(file);

            match SshConfig::default().parse(&mut reader, ParseRule::STRICT) {
                Ok(config) => Ok(Some(config)),
                Err(e) => Err(SshConfigError::ParseFailed {
                    path: path.display().to_string(),
                    message: e.to_string(),
                }),
            }
        } else {
            Ok(None)
        }
    }

    /// Verifies that an SSH configuration file exists and is accessible.
    ///
    /// This method checks whether the specified file path exists and can be accessed.
    /// It provides detailed error messages for different failure scenarios, including
    /// permission issues and I/O errors.
    ///
    /// # Parameters
    ///
    /// * `path` - A reference to the file path to check. This should point to an SSH
    ///   configuration file that needs to be validated for existence and accessibility.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the file exists and is accessible.
    ///
    /// Returns `Err(SshConfigError)` if:
    /// * The file does not exist
    /// * Permission is denied when attempting to access the file
    /// * An I/O error occurs during the existence check
    /// * Any other filesystem error prevents verification
    ///
    /// # Errors
    ///
    /// This method returns an error in the following cases:
    /// * `SshConfigError::NotFound` - The file does not exist at the specified path
    /// * `SshConfigError::PermissionDenied` - The file exists but cannot be accessed due to
    ///   insufficient permissions
    /// * `SshConfigError::CheckFailed` - Any other filesystem error occurred during the check
    pub(super) fn ensure_exists(&self, path: &Path) -> Result<(), SshConfigError> {
        match path.try_exists() {
            Ok(true) => Ok(()),
            Ok(false) => Err(SshConfigError::NotFound {
                path: path.display().to_string(),
            }),
            Err(e) => match e.kind() {
                ErrorKind::PermissionDenied => Err(SshConfigError::PermissionDenied {
                    path: path.display().to_string(),
                    message: e.to_string(),
                }),
                ErrorKind::NotFound => Err(SshConfigError::NotFound {
                    path: path.display().to_string(),
                }),
                _ => Err(SshConfigError::CheckFailed {
                    path: path.display().to_string(),
                    message: e.to_string(),
                }),
            },
        }
    }
}
