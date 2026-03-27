//! Plugin integration layer for Genja.
//!
//! This module provides the bridge between Genja Core settings and the plugin
//! system, implementing the default file-based inventory loading strategy.
//!
//! # Architecture
//!
//! - **Plugin Management**: Handled by [`genja_plugin_manager::PluginManager`]
//! - **Inventory Loading**: Delegated to inventory plugins implementing [`PluginInventory`]
//! - **Default Implementation**: [`DefaultInventoryPlugin`] loads from files
//! - **Extensibility**: Transform function plugins can modify hosts during load
//!
//! # Plugin Types
//!
//! ## Inventory Plugins
//!
//! Inventory plugins implement [`PluginInventory`] and are responsible for
//! loading host, group, and default data from various sources. The default
//! implementation reads from JSON/YAML files.
//!
//! ## Transform Function Plugins
//!
//! Transform function plugins implement [`PluginTransformFunction`] and can
//! modify host properties during inventory construction. They are applied
//! after the base inventory is loaded.
//!
//! # Configuration
//!
//! Inventory loading is configured through [`Settings::inventory()`], which
//! specifies:
//! - File paths for hosts, groups, and defaults
//! - Optional transform function plugin name
//! - Transform function options (if applicable)
//!
//! # Examples
//!
//! ## Basic File Loading
//!
//! ```no_run
//! use genja::plugins::DefaultInventoryPlugin;
//! use genja_core::Settings;
//! use genja_plugin_manager::PluginManager;
//! use genja_plugin_manager::plugin_types::PluginInventory;
//!
//! let settings = Settings::default();
//! let plugins = PluginManager::new();
//!
//! let inventory = DefaultInventoryPlugin
//!     .load(&settings, &plugins)
//!     .expect("inventory load failed");
//!
//! println!("Loaded {} hosts", inventory.hosts().len());
//! ```
//!
//! ## With Custom Transform Function
//!
//! ```no_run
//! use genja::plugins::DefaultInventoryPlugin;
//! use genja_core::Settings;
//! use genja_plugin_manager::PluginManager;
//! use genja_plugin_manager::plugin_types::{PluginInventory, Plugins};
//!
//! let settings = Settings::from_file("config.yaml")?;
//! let mut plugins = PluginManager::new();
//!
//! // Register custom transform function plugin
//! // plugins.register_plugin(Plugins::TransformFunction(...));
//!
//! let inventory = DefaultInventoryPlugin
//!     .load(&settings, &plugins)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # See Also
//!
//! - [`Genja::load_inventory`](crate::Genja::load_inventory) - Loading inventory into runtime
//! - [`Settings::inventory`](genja_core::Settings::inventory) - Inventory configuration
//! - [`PluginManager`](genja_plugin_manager::PluginManager) - Plugin management

use genja_core::inventory::Inventory;
use genja_core::{InventoryLoadError, Settings};
use genja_plugin_manager::PluginManager;
use genja_plugin_manager::plugin_types::{Plugin, PluginInventory, Plugins};

/// Default file-based inventory plugin.
///
/// Loads inventory from JSON or YAML files specified in settings. This plugin
/// is automatically registered when no custom inventory plugin is configured.
///
/// # File Loading
///
/// The plugin reads three types of files based on `Settings::inventory()`:
/// - **Hosts file** - Required, contains host definitions
/// - **Groups file** - Optional, defines host groupings
/// - **Defaults file** - Optional, provides default values
///
/// # Transform Functions
///
/// If a transform function plugin is specified in settings, it will be applied
/// to each host during inventory construction. Transform functions can modify
/// host properties dynamically.
///
/// # Examples
///
/// ## Basic Usage
///
/// ```no_run
/// use genja::plugins::DefaultInventoryPlugin;
/// use genja_core::Settings;
/// use genja_plugin_manager::PluginManager;
/// use genja_plugin_manager::plugin_types::PluginInventory;
///
/// let settings = Settings::default();
/// let plugins = PluginManager::new();
/// let inventory = DefaultInventoryPlugin
///     .load(&settings, &plugins)
///     .expect("inventory load failed");
/// ```
///
/// ## With Transform Function
///
/// ```no_run
/// use genja::plugins::DefaultInventoryPlugin;
/// use genja_core::Settings;
/// use genja_plugin_manager::PluginManager;
/// use genja_plugin_manager::plugin_types::{PluginInventory, Plugins, PluginTransformFunction};
///
/// let mut settings = Settings::default();
/// // Configure transform function in settings...
///
/// let mut plugins = PluginManager::new();
/// // Register transform function plugin...
///
/// let inventory = DefaultInventoryPlugin
///     .load(&settings, &plugins)
///     .expect("inventory load failed");
/// ```
pub struct DefaultInventoryPlugin;

impl Plugin for DefaultInventoryPlugin {
    fn name(&self) -> String {
        "FileInventoryPlugin".to_string()
    }
}

impl PluginInventory for DefaultInventoryPlugin {
    /// Loads inventory from files specified in settings.
    ///
    /// This method orchestrates the complete inventory loading process:
    /// 1. Reads host, group, and default files from paths in settings
    /// 2. Constructs an inventory builder with the loaded data
    /// 3. Applies transform function plugin if configured
    /// 4. Returns the fully constructed inventory
    ///
    /// # Parameters
    ///
    /// * `settings` - Configuration containing file paths and plugin names
    /// * `plugins` - Plugin manager for accessing transform function plugins
    ///
    /// # Returns
    ///
    /// Returns `Ok(Inventory)` with the loaded and configured inventory.
    ///
    /// # Errors
    ///
    /// Returns `Err(InventoryLoadError)` if:
    /// - Any specified file cannot be read or parsed
    /// - A configured transform function plugin is not found
    /// - A named plugin exists but is not a transform function plugin
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use genja::plugins::DefaultInventoryPlugin;
    /// use genja_core::Settings;
    /// use genja_plugin_manager::PluginManager;
    /// use genja_plugin_manager::plugin_types::PluginInventory;
    ///
    /// let settings = Settings::from_file("config.yaml")?;
    /// let plugins = PluginManager::new();
    ///
    /// let inventory = DefaultInventoryPlugin.load(&settings, &plugins)?;
    /// println!("Loaded {} hosts", inventory.hosts().len());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    fn load(
        &self,
        settings: &Settings,
        plugins: &PluginManager,
    ) -> Result<Inventory, InventoryLoadError> {
        let inventory_cfg = settings.inventory();

        let (hosts, groups, defaults) = inventory_cfg
            .load_inventory_files()
            .map_err(InventoryLoadError::from)?;

        let mut builder = Inventory::builder().hosts(hosts);

        if let Some(groups) = groups {
            builder = builder.groups(groups);
        }
        if let Some(defaults) = defaults {
            builder = builder.defaults(defaults);
        }

        if let Some(name) = inventory_cfg.transform_function() {
            let plugin = plugins.get_plugin(name).ok_or_else(|| {
                InventoryLoadError::from(format!("Transform plugin '{}' not found", name))
            })?;

            match plugin {
                Plugins::TransformFunction(transform) => {
                    builder = builder.transform_function(transform.transform_function());
                }
                _ => {
                    return Err(InventoryLoadError::from(format!(
                        "Plugin '{}' is not a transform function plugin",
                        name
                    )));
                }
            }

            if let Some(options) = inventory_cfg.transform_function_options() {
                builder = builder.transform_function_options(options.clone());
            }
        }

        Ok(builder.build())
    }
}
