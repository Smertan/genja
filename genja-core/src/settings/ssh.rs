use serde::{Deserialize, Serialize};

/// SSH client settings.
///
/// When `config_file` is set, `Settings::from_file` validates that the referenced
/// OpenSSH-style config can be opened and parsed.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
#[serde(default)]
pub struct SSHConfig {
    pub(super) config_file: Option<String>,
}

impl SSHConfig {
    pub fn builder() -> SSHConfigBuilder {
        SSHConfigBuilder::default()
    }

    pub fn config_file(&self) -> Option<&str> {
        self.config_file.as_deref()
    }
}

/// Builder for `SSHConfig`.
#[derive(Default)]
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
