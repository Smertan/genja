use super::env_defaults::get_inventory_plugin_config;
use crate::inventory::TransformFunctionOptions;
use serde::{Deserialize, Serialize};

/// Optional file paths for hosts, groups, and defaults inventory sources.
#[derive(Deserialize, Serialize, Clone, Debug, Default)]
pub struct OptionsConfig {
    pub(super) hosts_file: Option<String>,
    pub(super) groups_file: Option<String>,
    pub(super) defaults_file: Option<String>,
}

impl OptionsConfig {
    pub fn builder() -> OptionsConfigBuilder {
        OptionsConfigBuilder::default()
    }

    pub fn hosts_file(&self) -> Option<&str> {
        self.hosts_file.as_deref()
    }

    pub fn groups_file(&self) -> Option<&str> {
        self.groups_file.as_deref()
    }

    pub fn defaults_file(&self) -> Option<&str> {
        self.defaults_file.as_deref()
    }
}

/// Builder for `OptionsConfig`.
#[derive(Default)]
pub struct OptionsConfigBuilder {
    hosts_file: Option<String>,
    groups_file: Option<String>,
    defaults_file: Option<String>,
}

impl OptionsConfigBuilder {
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

/// Inventory loader configuration.
///
/// The plugin name defaults from `GENJA_INVENTORY_PLUGIN`. When present,
/// `transform_function` is attached to the built `Inventory` after the raw
/// inventory files are loaded. The transform itself is then applied lazily
/// when inventory data is accessed or resolved.
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct InventoryConfig {
    #[serde(default = "get_inventory_plugin_config")]
    pub(super) plugin: String,
    pub(super) options: OptionsConfig,
    pub(super) transform_function: Option<String>,
    pub(super) transform_function_options: Option<TransformFunctionOptions>,
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
        InventoryConfigBuilder::default()
    }

    pub fn plugin(&self) -> &str {
        &self.plugin
    }

    pub fn options(&self) -> &OptionsConfig {
        &self.options
    }

    pub fn transform_function(&self) -> Option<&str> {
        self.transform_function.as_deref()
    }

    pub fn transform_function_options(&self) -> Option<&TransformFunctionOptions> {
        self.transform_function_options.as_ref()
    }
}

/// Builder for `InventoryConfig`.
#[derive(Default)]
pub struct InventoryConfigBuilder {
    plugin: Option<String>,
    options: Option<OptionsConfig>,
    transform_function: Option<String>,
    transform_function_options: Option<TransformFunctionOptions>,
}

impl InventoryConfigBuilder {
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

    pub fn transform_function_options(mut self, options: TransformFunctionOptions) -> Self {
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
