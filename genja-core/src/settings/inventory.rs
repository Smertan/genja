use super::env_defaults::get_inventory_plugin_config;
use crate::inventory::TransformFunctionOptions;
use serde::{Deserialize, Serialize};

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
/// # Deserialization
///
/// - Missing fields default to `None`
/// - Invalid field values cause deserialization to fail
///
/// # Examples
///
/// ```
/// use genja_core::settings::OptionsConfig;
///
/// // Create with default values (all None)
/// let options = OptionsConfig::default();
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
    pub(super) hosts_file: Option<String>,
    pub(super) groups_file: Option<String>,
    pub(super) defaults_file: Option<String>,
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

/// Builder for constructing `OptionsConfig` instances with custom file paths.
///
/// This builder provides a fluent interface for creating `OptionsConfig` objects,
/// allowing selective configuration of inventory file paths. Fields that are not
/// explicitly set will remain `None` when `build()` is called.
///
/// # Fields
///
/// * `hosts_file` - Optional path to the hosts inventory file. When set to `Some(path)`,
///   the specified file will be used for loading host inventory data. When set to `None`,
///   no hosts file will be configured.
/// * `groups_file` - Optional path to the groups inventory file. When set to `Some(path)`,
///   the specified file will be used for loading group definitions. When set to `None`,
///   no groups file will be configured.
/// * `defaults_file` - Optional path to the defaults inventory file. When set to
///   `Some(path)`, the specified file will be used for loading default configuration
///   values. When set to `None`, no defaults file will be configured.
///
/// # Examples
///
/// ```
/// use genja_core::settings::OptionsConfig;
///
/// // Build with all file paths specified
/// let options = OptionsConfig::builder()
///     .hosts_file("/path/to/hosts.yaml")
///     .groups_file("/path/to/groups.yaml")
///     .defaults_file("/path/to/defaults.yaml")
///     .build();
///
/// // Build with only hosts file
/// let options = OptionsConfig::builder()
///     .hosts_file("/path/to/hosts.yaml")
///     .build();
///
/// // Build with defaults (all None)
/// let options = OptionsConfig::builder().build();
/// ```
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

impl Default for OptionsConfigBuilder {
    fn default() -> Self {
        Self {
            hosts_file: None,
            groups_file: None,
            defaults_file: None,
        }
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
///   or **FileInventoryPlugin** if not set.
/// * `options` - Configuration options specifying the file paths for hosts, groups,
///   and defaults inventory files.
/// * `transform_function` - Optional name of a transformation function to apply to
///   the loaded inventory data. This allows custom processing of inventory before use.
/// * `transform_function_options` - Optional JSON configuration passed to the
///   transformation function, allowing parameterized transformations.
///
/// # Deserialization
///
/// - Missing fields use their default values (see `Default` impl)
/// - The `plugin` field defaults to `GENJA_INVENTORY_PLUGIN` env var or "FileInventoryPlugin"
/// - Invalid field values cause deserialization to fail
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

/// Builder for constructing `InventoryConfig` instances with custom settings.
///
/// This builder provides a fluent interface for creating `InventoryConfig` objects,
/// allowing selective configuration of inventory management settings. Fields that are
/// not explicitly set will use their default values when `build()` is called.
///
/// # Fields
///
/// * `plugin` - Optional name of the inventory plugin to use. When set to `Some(name)`,
///   the specified plugin will be used for loading inventory data. When set to `None`,
///   the default value from the `GENJA_INVENTORY_PLUGIN` environment variable or
///   "FileInventoryPlugin" will be used.
/// * `options` - Optional configuration for inventory file paths. When set to
///   `Some(options)`, the specified paths for hosts, groups, and defaults files will
///   be used. When set to `None`, default `OptionsConfig` values (all `None`) will be used.
/// * `transform_function` - Optional name of a transformation function to apply to
///   the loaded inventory data. When set to `Some(name)`, the specified function will
///   be invoked to transform the inventory. When set to `None`, no transformation
///   will be applied.
/// * `transform_function_options` - Optional JSON configuration passed to the
///   transformation function. When set to `Some(value)`, the specified JSON object
///   will be provided as parameters to the transformation function. When set to `None`,
///   no options will be passed to the transformation function.
///
/// # Examples
///
/// ```
/// use genja_core::inventory::TransformFunctionOptions;
/// use genja_core::settings::{InventoryConfig, OptionsConfig};
///
/// // Build with custom plugin and options
/// let config = InventoryConfig::builder()
///     .plugin("CustomInventoryPlugin")
///     .options(OptionsConfig::builder()
///         .hosts_file("/path/to/hosts.yaml")
///         .build())
///     .transform_function("my_transform")
///     .transform_function_options(
///         TransformFunctionOptions::new(serde_json::json!({"key": "value"})),
///     )
///     .build();
///
/// // Build with defaults
/// let config = InventoryConfig::builder().build();
/// ```
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

impl Default for InventoryConfigBuilder {
    fn default() -> Self {
        Self {
            plugin: None,
            options: None,
            transform_function: None,
            transform_function_options: None,
        }
    }
}
