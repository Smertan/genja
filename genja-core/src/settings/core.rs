use super::env_defaults::{deserialize_bool_loose, raise_on_error};
use serde::{Deserialize, Serialize};

/// Core runtime behavior settings.
///
/// `raise_on_error` defaults from `GENJA_CORE_RAISE_ON_ERROR` and accepts the
/// same loose boolean forms used elsewhere in the settings module.
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

/// Builder for `CoreConfig`.
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
