//! Python bindings for the Genja runtime and builder.
//!
//! This module provides Python-accessible wrappers around the core Genja runtime,
//! enabling Python applications to create, configure, and execute Genja tasks with
//! full access to the plugin system, inventory management, and task execution engine.
//!
//! # Core Components
//!
//! - [`PyGenja`] - Main runtime class for executing tasks and managing inventory
//! - [`PyGenjaBuilder`] - Builder pattern for constructing runtime instances
//!
//! # Creating Runtime Instances
//!
//! The runtime can be created in several ways, each suited to different use cases:
//!
//! ## From Hosts Dictionary
//!
//! The simplest approach for basic host management:
//!
//! ```python
//! from genja import Genja
//!
//! hosts = {
//!     "router1": {"hostname": "10.0.0.1", "platform": "ios"},
//!     "router2": {"hostname": "10.0.0.2", "platform": "nxos"}
//! }
//!
//! runtime = Genja.from_hosts(hosts)
//! ```
//!
//! ## From Full Inventory
//!
//! For complete inventory with groups and defaults:
//!
//! ```python
//! inventory = {
//!     "hosts": {
//!         "router1": {"hostname": "10.0.0.1", "groups": ["core"]},
//!         "router2": {"hostname": "10.0.0.2", "groups": ["edge"]}
//!     },
//!     "groups": {
//!         "core": {"platform": "ios"},
//!         "edge": {"platform": "nxos"}
//!     },
//!     "defaults": {
//!         "username": "admin",
//!         "port": 22
//!     }
//! }
//!
//! runtime = Genja.from_inventory(inventory)
//! ```
//!
//! ## From Settings File
//!
//! Load configuration from YAML or JSON:
//!
//! ```python
//! runtime = Genja.from_settings_file("config.yaml")
//! ```
//!
//! ## Using the Builder Pattern
//!
//! For advanced configuration with plugins:
//!
//! ```python
//! from genja import Genja, PluginManager
//!
//! plugin_manager = PluginManager()
//! plugin_manager.load_rust_plugins_from_directory("./plugins")
//!
//! runtime = (Genja.builder(hosts)
//!     .with_plugin_manager(plugin_manager)
//!     .with_runner("threaded")
//!     .build())
//! ```
//!
//! # Inventory Management
//!
//! The runtime provides multiple ways to access and filter inventory:
//!
//! ## Accessing Inventory
//!
//! ```python
//! # Get raw hosts only
//! hosts = runtime.inventory()
//!
//! # Get full inventory structure with transforms applied
//! full = runtime.inventory_full()
//!
//! # Get raw inventory (before transformation)
//! raw = runtime.inventory_raw()
//!
//! # Get only raw hosts
//! hosts_raw = runtime.hosts_raw()
//! ```
//!
//! ## Filtering Hosts
//!
//! ```python
//! # Filter by key presence
//! ios_devices = runtime.filter_by_key("platform")
//!
//! # Filter by key-value pattern (supports regex)
//! core_routers = runtime.filter_by_key_value("groups", "core")
//! ios_routers = runtime.filter_by_key_value("platform", "^ios")
//!
//! # Filters are chainable and immutable
//! filtered = (runtime
//!     .filter_by_key("platform")
//!     .filter_by_key_value("platform", "ios"))
//! ```
//!
//! ## Iterating Over Hosts
//!
//! ```python
//! # Iterate over selected hosts (after filtering)
//! for host_id, host in runtime.iter_selected_hosts():
//!     print(f"{host_id}: {host['hostname']}")
//!
//! # Iterate over all inventory hosts
//! for host_id, host in runtime.iter_inventory_hosts():
//!     print(f"{host_id}: {host}")
//! ```
//!
//! # Task Execution
//!
//! Execute decorated Python task classes across hosts:
//!
//! ```python
//! from genja.task import Host, TaskSuccessResult, task
//!
//! @task(name="backup_config")
//! class BackupTask:
//!     def run(self, task, host: Host, context):
//!         return TaskSuccessResult(summary=f"backed up {host.hostname}")
//!
//! # Run task with default depth limit
//! results = runtime.run_task(BackupTask)
//!
//! # Run with custom depth limit for sub-tasks
//! results = runtime.run_task(BackupTask, max_depth=5)
//! ```
//!
//! # Plugin Integration
//!
//! ## Loading Plugins
//!
//! Plugins can be loaded from directories or registered individually:
//!
//! ```python
//! from genja import PluginManager
//!
//! # Load from directory
//! plugin_manager = PluginManager()
//! plugin_manager.load_rust_plugins_from_directory("./plugins")
//!
//! # Use with runtime
//! runtime = Genja.from_hosts(hosts, plugin_manager=plugin_manager)
//! ```
//!
//! ## Python Plugins
//!
//! Register Python-based plugins directly:
//!
//! ```python
//! class MyPythonPlugin:
//!     def process(self, data):
//!         return data
//!
//! runtime = (Genja.builder(hosts)
//!     .with_plugin(MyPythonPlugin())
//!     .build())
//! ```
//!
//! # Runner Selection
//!
//! Choose execution strategy for tasks:
//!
//! ```python
//! # Use threaded runner (default)
//! runtime = runtime.with_runner("threaded")
//!
//! # Use serial runner for sequential execution
//! runtime = runtime.with_runner("serial")
//! ```
//!
//! # Settings Integration
//!
//! Provide settings loaded from a file or defaults:
//!
//! ```python
//! from genja import Settings
//!
//! settings = Settings.from_file("config.yaml")
//! runtime = Genja.from_hosts(hosts, settings=settings)
//! ```
//!
//! # Runtime State Inspection
//!
//! Query runtime state and configuration:
//!
//! ```python
//! # Check if plugins are loaded
//! if runtime.plugins_loaded():
//!     print("Plugins available")
//!
//! # Check if inventory is loaded
//! if runtime.inventory_loaded():
//!     print("Inventory ready")
//!
//! # Get host count
//! count = runtime.host_count()
//!
//! # Get host IDs
//! ids = runtime.host_ids()
//!
//! # Access settings
//! settings = runtime.settings()
//! ```
//!
//! # Pydantic Model Support
//!
//! The module automatically handles Pydantic models for inventory:
//!
//! ```python
//! from pydantic import BaseModel
//!
//! class Host(BaseModel):
//!     hostname: str
//!     platform: str
//!     port: int = 22
//!
//! hosts = {
//!     "router1": Host(hostname="10.0.0.1", platform="ios")
//! }
//!
//! # Pydantic models are automatically converted
//! runtime = Genja.from_hosts(hosts)
//! ```
//!
//! # Error Handling
//!
//! All operations return `PyResult` and raise `PyValueError` on failure:
//!
//! ```python
//! try:
//!     runtime = Genja.from_settings_file("missing.yaml")
//! except ValueError as e:
//!     print(f"Failed to load settings: {e}")
//!
//! try:
//!     filtered = runtime.filter_by_key_value("platform", "[invalid")
//! except ValueError as e:
//!     print(f"Invalid regex pattern: {e}")
//! ```
//!
//! # Builder Pattern Details
//!
//! The builder is consumed on `build()` and cannot be reused:
//!
//! ```python
//! builder = Genja.builder(hosts)
//! runtime1 = builder.build()  # OK
//! runtime2 = builder.build()  # Error: builder already consumed
//! ```
//!
//! To create multiple runtimes, create separate builders:
//!
//! ```python
//! builder1 = Genja.builder(hosts).with_runner("threaded")
//! builder2 = Genja.builder(hosts).with_runner("serial")
//!
//! runtime1 = builder1.build()
//! runtime2 = builder2.build()
//! ```
//!
//! # Thread Safety
//!
//! The runtime is thread-safe and can be shared across Python threads:
//!
//! ```python
//! import threading
//! from genja.task import TaskSuccessResult, task
//!
//! @task(name="show_version")
//! class ShowVersionTask:
//!     def run(self, task, host, context):
//!         return TaskSuccessResult(summary=f"checked {host.hostname}")
//!
//! def worker(runtime):
//!     results = runtime.run_task(ShowVersionTask)
//!     print(results)
//!
//! runtime = Genja.from_hosts(hosts)
//! threads = [threading.Thread(target=worker, args=(runtime,)) for _ in range(4)]
//! for t in threads:
//!     t.start()
//! for t in threads:
//!     t.join()
//! ```
//!
//! # Performance Considerations
//!
//! - Filtering operations create new runtime instances but share the underlying inventory
//! - Inventory transformations are applied when using transformed inventory accessors
//! - Task execution parallelism is controlled by the runner plugin
//! - Python-Rust conversions happen at API boundaries
//!
//! # Examples
//!
//! ## Complete Workflow
//!
//! ```python
//! from genja import Genja, PluginManager
//! from genja.task import TaskSuccessResult, task
//!
//! # Setup
//! plugin_manager = PluginManager()
//! plugin_manager.load_rust_plugins_from_directory("./plugins")
//!
//! hosts = {
//!     "router1": {"hostname": "10.0.0.1", "platform": "ios"},
//!     "router2": {"hostname": "10.0.0.2", "platform": "nxos"}
//! }
//!
//! @task(name="show_version")
//! class ShowVersionTask:
//!     def run(self, task, host, context):
//!         return TaskSuccessResult(summary=f"checked {host.hostname}")
//!
//! # Create runtime
//! runtime = (Genja.builder(hosts)
//!     .with_plugin_manager(plugin_manager)
//!     .with_runner("threaded")
//!     .build())
//!
//! # Filter and execute
//! ios_runtime = runtime.filter_by_key_value("platform", "ios")
//! results = ios_runtime.run_task(ShowVersionTask)
//!
//! # Process results
//! for host_id, result in results.to_dict()["hosts"].items():
//!     print(f"{host_id}: {result['status']}")
//! ```
//!
//! ## Settings File Workflow
//!
//! ```python
//! # config.yaml:
//! # inventory:
//! #   plugin: FileInventoryPlugin
//! #   options:
//! #     hosts_file: hosts.yaml
//! # runner:
//! #   plugin: threaded
//! #   worker_count: 10
//!
//! runtime = Genja.from_settings_file("config.yaml")
//! results = runtime.run_task(ShowVersionTask)
//! ```

use genja::Genja as RuntimeGenja;
use genja_core::inventory::{Defaults, Groups, Hosts, Inventory};
use genja_core::{GenjaError, Settings};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use serde::de::DeserializeOwned;
use std::sync::Mutex;

use crate::plugin_manager::{register_python_plugin_on_manager, PyPluginManager};
use crate::settings::PySettings;
use crate::task::{self, PyTaskResults};

/// Python wrapper for the Genja runtime.
///
/// This class provides the main interface for executing tasks and managing inventory
/// in Python applications. It wraps the core Rust `RuntimeGenja` implementation and
/// exposes its functionality through Python-friendly methods.
///
/// # Creation Methods
///
/// - [`PyGenja::from_hosts`] - Create from a simple hosts dictionary
/// - [`PyGenja::from_inventory`] - Create from a full inventory structure
/// - [`PyGenja::from_settings_file`] - Load from a YAML/JSON configuration file
/// - [`PyGenja::builder`] - Use the builder pattern for advanced configuration
///
/// # Features
///
/// - **Inventory Management**: Access and filter hosts with flexible query methods
/// - **Task Execution**: Run tasks across hosts with automatic parallelization
/// - **Plugin Integration**: Support for inventory, runner, and transform plugins
/// - **Filtering**: Chain filters to select specific hosts for task execution
/// - **Thread Safety**: Safe to share across Python threads
///
/// # Examples
///
/// ```python
/// from genja import Genja
/// from genja.task import TaskSuccessResult, task
///
/// @task(name="show_version")
/// class ShowVersionTask:
///     def run(self, task, host, context):
///         return TaskSuccessResult(summary=f"checked {host.hostname}")
///
/// hosts = {
///     "router1": {"hostname": "10.0.0.1", "platform": "ios"},
///     "router2": {"hostname": "10.0.0.2", "platform": "nxos"}
/// }
/// runtime = Genja.from_hosts(hosts)
///
/// ios_runtime = runtime.filter_by_key_value("platform", "ios")
/// results = ios_runtime.run_task(ShowVersionTask)
/// ```
#[pyclass(name = "Genja")]
#[derive(Clone)]
pub struct PyGenja {
    inner: RuntimeGenja,
}

#[pymethods]
impl PyGenja {
    /// Creates a new builder for constructing a Genja runtime instance.
    ///
    /// This method initializes a [`PyGenjaBuilder`] with the provided inventory and optional
    /// configuration. The builder pattern allows for flexible runtime construction with
    /// method chaining for additional configuration before calling `build()`.
    ///
    /// # Parameters
    ///
    /// * `hosts` - A Python object containing inventory data. Can be either:
    ///   - A dictionary mapping host IDs to host payloads (simple hosts format)
    ///   - A full inventory structure with "hosts", "groups", and "defaults" keys
    ///   - A Pydantic model that can be converted to a dictionary via `model_dump()`
    /// * `settings` - Optional runtime settings for configuring behavior such as logging,
    ///   runner selection, and plugin options. If `None`, default settings are used.
    /// * `plugin_manager` - Optional plugin manager for loading and managing plugins.
    ///   If `None`, a new plugin manager with default plugins is created.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<PyGenjaBuilder>` containing the builder instance on success,
    /// or a `PyValueError` if the inventory cannot be parsed or converted.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The `hosts` parameter cannot be converted to a valid inventory structure
    /// - The plugin manager cannot be initialized or accessed
    #[staticmethod]
    #[pyo3(signature = (hosts, settings=None, plugin_manager=None))]
    fn builder(
        hosts: Bound<'_, PyAny>,
        settings: Option<PyRef<'_, PySettings>>,
        plugin_manager: Option<PyRef<'_, PyPluginManager>>,
    ) -> PyResult<PyGenjaBuilder> {
        let inventory = python_inventory_to_rust_inventory(hosts)?;
        let settings = settings.map(|settings| settings.inner.clone());
        let plugin_manager = if let Some(plugin_manager) = plugin_manager {
            plugin_manager.take_inner()?
        } else {
            PyPluginManager::new().take_inner()?
        };

        Ok(PyGenjaBuilder {
            inner: Mutex::new(Some(PyGenjaBuilderState {
                inventory,
                settings,
                plugin_manager,
                runner: None,
            })),
        })
    }

    /// Creates a Genja runtime instance directly from a hosts dictionary.
    ///
    /// This is a convenience method that combines [`PyGenja::builder`] and `build()` into
    /// a single call. It's the simplest way to create a runtime when you only need to
    /// provide host information without additional configuration.
    ///
    /// # Parameters
    ///
    /// * `hosts` - A Python dictionary mapping host IDs to host payloads. Each host payload
    ///   should contain at minimum a "hostname" field, and may include additional fields
    ///   like "platform", "port", "username", etc.
    /// * `settings` - Optional runtime settings for configuring behavior. If `None`,
    ///   default settings are used.
    /// * `plugin_manager` - Optional plugin manager for loading and managing plugins.
    ///   If `None`, a new plugin manager with default plugins is created.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing the initialized runtime instance on success,
    /// or a `PyValueError` if the hosts cannot be parsed or the runtime cannot be built.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The `hosts` parameter is not a valid dictionary structure
    /// - Host payloads contain invalid data types or missing required fields
    /// - The runtime builder fails to construct the runtime instance
    #[staticmethod]
    #[pyo3(signature = (hosts, settings=None, plugin_manager=None))]
    fn from_hosts(
        hosts: Bound<'_, PyAny>,
        settings: Option<PyRef<'_, PySettings>>,
        plugin_manager: Option<PyRef<'_, PyPluginManager>>,
    ) -> PyResult<Self> {
        let builder = Self::builder(hosts, settings, plugin_manager)?;
        builder.build()
    }

    /// Creates a Genja runtime instance from a full inventory structure.
    ///
    /// This method accepts a complete inventory specification including hosts, groups,
    /// and defaults. It's useful when you need to organize hosts into groups with
    /// shared configuration or apply default values across all hosts.
    ///
    /// # Parameters
    ///
    /// * `inventory` - A Python dictionary containing the full inventory structure with
    ///   optional keys:
    ///   - "hosts": Dictionary mapping host IDs to host payloads
    ///   - "groups": Dictionary mapping group IDs to group configurations
    ///   - "defaults": Dictionary containing default values applied to all hosts
    /// * `settings` - Optional runtime settings for configuring behavior. If `None`,
    ///   default settings are used.
    /// * `plugin_manager` - Optional plugin manager for loading and managing plugins.
    ///   If `None`, a new plugin manager with default plugins is created.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing the initialized runtime instance on success,
    /// or a `PyValueError` if the inventory structure is invalid or the runtime cannot
    /// be built.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The `inventory` parameter is not a valid dictionary structure
    /// - The inventory structure contains invalid or inconsistent data
    /// - The runtime builder fails to construct the runtime instance
    #[staticmethod]
    #[pyo3(signature = (inventory, settings=None, plugin_manager=None))]
    fn from_inventory(
        inventory: Bound<'_, PyAny>,
        settings: Option<PyRef<'_, PySettings>>,
        plugin_manager: Option<PyRef<'_, PyPluginManager>>,
    ) -> PyResult<Self> {
        let builder = Self::builder(inventory, settings, plugin_manager)?;
        builder.build()
    }

    /// Creates a Genja runtime instance from a YAML or JSON settings file.
    ///
    /// This method loads runtime configuration from a file, including inventory plugin
    /// configuration, runner selection, logging settings, and other options. The inventory
    /// is loaded using the configured inventory plugin (or `FileInventoryPlugin` by default).
    ///
    /// # Parameters
    ///
    /// * `path` - Path to the settings file (YAML or JSON format). The file should contain
    ///   a valid settings structure with sections for inventory, runner, logging, etc.
    /// * `plugin_manager` - Optional plugin manager for loading and managing plugins.
    ///   If provided, it will be used instead of creating a new one. This is useful when
    ///   you need to register custom Python plugins before loading the settings.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing the fully configured runtime instance on success,
    /// or a `PyValueError` if the settings file cannot be read, parsed, or if the runtime
    /// cannot be built from the settings.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The settings file does not exist or cannot be read
    /// - The file contains invalid YAML or JSON syntax
    /// - The settings structure is invalid or incomplete
    /// - The configured inventory plugin cannot be found or fails to load inventory
    /// - The configured runner plugin cannot be found or initialized
    #[staticmethod]
    #[pyo3(signature = (path, plugin_manager=None))]
    fn from_settings_file(
        path: &str,
        plugin_manager: Option<PyRef<'_, PyPluginManager>>,
    ) -> PyResult<Self> {
        if let Some(plugin_manager) = plugin_manager {
            let settings = Settings::from_file(path).map_err(|err| {
                PyValueError::new_err(format!("failed to load settings from {path}: {err}"))
            })?;
            let plugin_manager = plugin_manager.take_inner()?;
            return build_runtime_from_settings(settings, plugin_manager, None);
        }

        let inner = RuntimeGenja::from_settings_file(path).map_err(|err| {
            PyValueError::new_err(format!(
                "failed to build Genja runtime from settings file {path}: {err}"
            ))
        })?;
        Ok(Self { inner })
    }

    /// Selects a specific runner plugin for task execution.
    ///
    /// This method creates a new runtime instance configured to use the specified runner plugin.
    /// The runner determines how tasks are executed across hosts (e.g., serial, threaded, or custom).
    /// The runtime instance is immutable, so this method returns a new instance with the selected runner.
    ///
    /// # Parameters
    ///
    /// * `runner` - The name of the runner plugin to use. Common values include:
    ///   - "serial" - Execute tasks sequentially, one host at a time
    ///   - "threaded" - Execute tasks in parallel using threads (default)
    ///   - Custom runner names registered via the plugin manager
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing a new runtime instance configured with the specified
    /// runner on success, or a `PyValueError` if the runner plugin cannot be found or initialized.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The specified runner plugin is not registered in the plugin manager
    /// - The runner plugin fails to initialize or configure
    fn with_runner(&self, runner: &str) -> PyResult<Self> {
        let inner = self.inner.with_runner(runner).map_err(|err| {
            PyValueError::new_err(format!("failed to select runner {runner}: {err}"))
        })?;
        Ok(Self { inner })
    }

    /// Checks whether plugins have been loaded into the runtime.
    ///
    /// This method returns `true` if a plugin manager has been configured and plugins
    /// are available for use, or `false` if no plugin manager is present.
    ///
    /// # Returns
    ///
    /// Returns `true` if plugins are loaded and available, `false` otherwise.
    fn plugins_loaded(&self) -> bool {
        self.inner.plugins_loaded()
    }

    /// Checks whether inventory has been loaded into the runtime.
    ///
    /// This method returns `true` if the runtime has successfully loaded inventory data
    /// (hosts, groups, and defaults), or `false` if no inventory is present.
    ///
    /// # Returns
    ///
    /// Returns `true` if inventory is loaded and available, `false` otherwise.
    fn inventory_loaded(&self) -> bool {
        self.inner.inventory_loaded()
    }

    /// Retrieves the current runtime settings.
    ///
    /// This method returns a copy of the settings object that was used to configure
    /// the runtime, including logging configuration, runner options, inventory plugin
    /// settings, and other runtime behavior parameters.
    ///
    /// # Returns
    ///
    /// Returns a `PySettings` instance containing the current runtime configuration.
    fn settings(&self) -> PySettings {
        PySettings {
            inner: self.inner.settings().clone(),
        }
    }

    /// Returns the total number of hosts in the current selection.
    ///
    /// This method returns the count of hosts that match the current filter criteria.
    /// If no filters have been applied, it returns the total number of hosts in the inventory.
    ///
    /// # Returns
    ///
    /// Returns the number of hosts as a `usize`.
    fn host_count(&self) -> usize {
        self.inner.host_count()
    }

    /// Returns the IDs of all hosts in the current selection.
    ///
    /// This method returns a list of host identifiers that match the current filter criteria.
    /// If no filters have been applied, it returns all host IDs from the inventory.
    /// The order of host IDs is consistent with the order used by other iteration methods.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<String>` containing the host IDs in the current selection.
    fn host_ids(&self) -> Vec<String> {
        self.inner
            .host_ids()
            .iter()
            .map(|host_id| host_id.to_string())
            .collect()
    }

    /// Iterates over the currently selected hosts and their payloads.
    ///
    /// This method returns a list of tuples containing host IDs and their corresponding
    /// transformed host payloads (with groups and defaults applied). Only hosts that match
    /// the current filter criteria are included. The host payloads are converted to Python
    /// dictionaries for easy access in Python code.
    ///
    /// # Parameters
    ///
    /// * `py` - The Python GIL token required for creating Python objects and performing
    ///   conversions between Rust and Python types.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Vec<(String, Py<PyAny>)>>` containing a list of tuples where:
    /// - The first element is the host ID as a `String`
    /// - The second element is the host payload as a Python dictionary (`Py<PyAny>`)
    ///
    /// Returns a `PyValueError` if the hosts cannot be iterated or if payload conversion fails.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The inventory is not loaded or accessible
    /// - Host payloads cannot be converted to Python dictionaries
    /// - The selected hosts cannot be retrieved from the runtime
    fn iter_selected_hosts(&self, py: Python<'_>) -> PyResult<Vec<(String, Py<PyAny>)>> {
        let hosts = self.inner.iter_selected_hosts().map_err(|err| {
            PyValueError::new_err(format!("failed to iterate selected hosts: {err}"))
        })?;
        let selected_ids = self.host_ids();
        selected_ids
            .into_iter()
            .zip(hosts.into_iter())
            .map(|(host_id, host)| {
                Ok((
                    host_id,
                    entity_to_py_dict(py, &host, "failed to convert selected host payload")?,
                ))
            })
            .collect()
    }

    /// Filters the runtime to include only hosts that have a specific key in their payload.
    ///
    /// This method creates a new runtime instance that contains only the hosts whose payloads
    /// include the specified key. The filtering is applied to the current selection, so it can
    /// be chained with other filter operations. The original runtime instance remains unchanged.
    ///
    /// # Parameters
    ///
    /// * `key` - The name of the key to check for in each host's payload. Only hosts that
    ///   contain this key (regardless of its value) will be included in the filtered runtime.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing a new runtime instance with only the hosts that
    /// have the specified key in their payloads on success, or a `PyValueError` if the filter
    /// operation fails.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The inventory is not loaded or accessible
    /// - The filter operation encounters an internal error
    fn filter_by_key(&self, key: &str) -> PyResult<Self> {
        let inner = self.inner.filter_by_key(key).map_err(|err| {
            PyValueError::new_err(format!("failed to filter hosts by key {key}: {err}"))
        })?;
        Ok(Self { inner })
    }

    /// Filters the runtime to include only hosts whose specified key matches a value pattern.
    ///
    /// This method creates a new runtime instance that contains only the hosts whose payloads
    /// have the specified key with a value matching the provided pattern. The pattern is treated
    /// as a regular expression, allowing for flexible matching. The filtering is applied to the
    /// current selection, so it can be chained with other filter operations. The original runtime
    /// instance remains unchanged.
    ///
    /// # Parameters
    ///
    /// * `key` - The name of the key to check in each host's payload. Only hosts that contain
    ///   this key will be evaluated against the value pattern.
    /// * `value_pattern` - A regular expression pattern to match against the value of the
    ///   specified key. Hosts whose key value matches this pattern will be included in the
    ///   filtered runtime. The pattern follows standard regex syntax (e.g., "^ios$" for exact
    ///   match, ".*core.*" for substring match).
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing a new runtime instance with only the hosts whose
    /// specified key value matches the pattern on success, or a `PyValueError` if the filter
    /// operation fails or the regex pattern is invalid.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The inventory is not loaded or accessible
    /// - The `value_pattern` is not a valid regular expression
    /// - The filter operation encounters an internal error
    fn filter_by_key_value(&self, key: &str, value_pattern: &str) -> PyResult<Self> {
        let inner = self
            .inner
            .filter_by_key_value(key, value_pattern)
            .map_err(|err| {
                PyValueError::new_err(format!(
                    "failed to filter hosts by key {key} and value pattern {value_pattern}: {err}"
                ))
            })?;
        Ok(Self { inner })
    }

    /// Retrieves the raw inventory hosts as a Python dictionary.
    ///
    /// This method returns the raw hosts from the inventory (before group and default
    /// transformations are applied) as a Python dictionary mapping host IDs to host payloads.
    /// This is useful when you need to access the original host definitions without any
    /// inherited values from groups or defaults.
    ///
    /// # Parameters
    ///
    /// * `py` - The Python GIL token required for creating Python objects and performing
    ///   conversions between Rust and Python types.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary where keys are host IDs
    /// and values are host payload dictionaries on success, or a `PyValueError` if the inventory
    /// cannot be accessed or converted.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The inventory is not loaded or accessible
    /// - Host payloads cannot be converted to Python dictionaries
    fn inventory(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let inventory = self.inner.inventory().map_err(|err| {
            PyValueError::new_err(format!("failed to access loaded inventory: {err}"))
        })?;
        inventory_hosts_to_py_dict(py, inventory.hosts_raw())
    }

    /// Retrieves the complete inventory structure as a Python dictionary.
    ///
    /// This method returns the full inventory including transformed hosts (with groups and
    /// defaults applied), groups, and defaults as a Python dictionary. The returned structure
    /// contains three keys: "hosts", "groups", and "defaults". This is useful when you need
    /// to access the complete inventory configuration including all transformations.
    ///
    /// # Parameters
    ///
    /// * `py` - The Python GIL token required for creating Python objects and performing
    ///   conversions between Rust and Python types.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary with keys "hosts",
    /// "groups", and "defaults" on success, or a `PyValueError` if the inventory cannot be
    /// accessed or converted. The "hosts" value contains transformed host payloads with
    /// group and default values applied.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The inventory is not loaded or accessible
    /// - Inventory components cannot be converted to Python dictionaries
    fn inventory_full(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let inventory = self.inner.inventory().map_err(|err| {
            PyValueError::new_err(format!("failed to access loaded inventory: {err}"))
        })?;
        inventory_to_py_dict(py, inventory)
    }

    /// Retrieves the raw inventory structure as a Python dictionary.
    ///
    /// This method returns the complete raw inventory including hosts, groups, and defaults
    /// before any transformations are applied. The returned structure contains three keys:
    /// "hosts", "groups", and "defaults", all in their original form as loaded from the
    /// inventory source. This is useful when you need to inspect the original inventory
    /// configuration without any inherited or computed values.
    ///
    /// # Parameters
    ///
    /// * `py` - The Python GIL token required for creating Python objects and performing
    ///   conversions between Rust and Python types.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary with keys "hosts",
    /// "groups", and "defaults" representing the raw, untransformed inventory on success,
    /// or a `PyValueError` if the inventory cannot be accessed or converted.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The inventory is not loaded or accessible
    /// - Raw inventory components cannot be converted to Python dictionaries
    fn inventory_raw(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let inventory = self.inner.inventory().map_err(|err| {
            PyValueError::new_err(format!("failed to access loaded inventory: {err}"))
        })?;
        raw_inventory_to_py_dict(py, inventory)
    }

    /// Iterates over all hosts in the inventory and their transformed payloads.
    ///
    /// This method returns a list of tuples containing host IDs and their corresponding
    /// transformed host payloads (with groups and defaults applied). Unlike `iter_selected_hosts`,
    /// this method returns all hosts in the inventory regardless of any filter criteria that
    /// may have been applied to the runtime. The host payloads are converted to Python
    /// dictionaries for easy access in Python code.
    ///
    /// # Parameters
    ///
    /// * `py` - The Python GIL token required for creating Python objects and performing
    ///   conversions between Rust and Python types.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Vec<(String, Py<PyAny>)>>` containing a list of tuples where:
    /// - The first element is the host ID as a `String`
    /// - The second element is the transformed host payload as a Python dictionary (`Py<PyAny>`)
    ///
    /// Returns a `PyValueError` if the inventory hosts cannot be iterated or if payload
    /// conversion fails.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The inventory is not loaded or accessible
    /// - Host payloads cannot be converted to Python dictionaries
    /// - The inventory hosts cannot be retrieved from the runtime
    fn iter_inventory_hosts(&self, py: Python<'_>) -> PyResult<Vec<(String, Py<PyAny>)>> {
        let hosts = self.inner.iter_inventory_hosts().map_err(|err| {
            PyValueError::new_err(format!("failed to iterate inventory hosts: {err}"))
        })?;
        hosts
            .into_iter()
            .map(|(host_id, host)| {
                Ok((
                    host_id.to_string(),
                    entity_to_py_dict(py, &host, "failed to convert inventory host payload")?,
                ))
            })
            .collect()
    }

    /// Retrieves the raw inventory hosts as a Python dictionary.
    ///
    /// This method returns the raw hosts from the inventory (before group and default
    /// transformations are applied) as a Python dictionary mapping host IDs to host payloads.
    /// This is useful when you need to access the original host definitions without any
    /// inherited values from groups or defaults.
    ///
    /// # Parameters
    ///
    /// * `py` - The Python GIL token required for creating Python objects and performing
    ///   conversions between Rust and Python types.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary where keys are host IDs
    /// and values are raw host payload dictionaries on success, or a `PyValueError` if the
    /// inventory cannot be accessed or converted.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The inventory is not loaded or accessible
    /// - Raw host payloads cannot be converted to Python dictionaries
    fn hosts_raw(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let inventory = self.inner.inventory().map_err(|err| {
            PyValueError::new_err(format!("failed to access loaded inventory: {err}"))
        })?;
        inventory_hosts_to_py_dict(py, inventory.hosts_raw())
    }

    /// Executes a task across all selected hosts in the runtime.
    ///
    /// This method runs the provided task class on all hosts that match the current filter
    /// criteria. Tasks are executed according to the configured runner plugin (serial, threaded,
    /// or custom). The task class must inherit from the `Task` base class and implement the
    /// required task methods. Task execution supports automatic parallelization, error handling,
    /// and result aggregation.
    ///
    /// # Parameters
    ///
    /// * `py` - The Python GIL token required for executing Python code and performing
    ///   conversions between Rust and Python types during task execution.
    /// * `task_class` - A Python class that inherits from `Task` and implements the task
    ///   execution logic. The class will be instantiated for each host and its methods will
    ///   be called according to the task lifecycle (start, run, etc.).
    /// * `max_depth` - Optional maximum depth limit for sub-task execution. If `None`, a
    ///   default depth limit is used. This prevents infinite recursion when tasks spawn
    ///   other tasks. A depth of 0 means no sub-tasks are allowed, while higher values
    ///   allow nested task execution up to the specified depth.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<PyTaskResults>` containing the aggregated results from all host
    /// executions on success, or a `PyValueError` if the task class is invalid, task execution
    /// fails, or the runner encounters an error. The results include information about passed,
    /// failed, and skipped hosts, along with any data returned by the task implementations.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The `task_class` is not a valid Python class or does not inherit from `Task`
    /// - The task class cannot be instantiated or its methods cannot be called
    /// - The runner plugin encounters an error during task execution
    /// - Task execution exceeds the maximum depth limit for sub-tasks
    /// - The task implementation raises an unhandled exception
    #[pyo3(signature = (task_class, max_depth=None))]
    fn run_task(
        &self,
        py: Python<'_>,
        task_class: Bound<'_, PyAny>,
        max_depth: Option<usize>,
    ) -> PyResult<PyTaskResults> {
        task::run_task(py, &self.inner, task_class, max_depth)
    }

    /// Returns a string representation of the Genja runtime instance.
    ///
    /// This method provides a human-readable representation of the runtime state,
    /// showing whether plugins and inventory have been successfully loaded. It's
    /// primarily used for debugging and logging purposes, and is automatically
    /// called by Python's `repr()` function and when displaying the object in
    /// interactive environments.
    ///
    /// # Returns
    ///
    /// Returns a `String` in the format `"Genja(plugins_loaded=<bool>, inventory_loaded=<bool>)"`,
    /// where `<bool>` is either `true` or `false` indicating the loading state of plugins
    /// and inventory respectively.
    fn __repr__(&self) -> String {
        format!(
            "Genja(plugins_loaded={}, inventory_loaded={})",
            self.inner.plugins_loaded(),
            self.inner.inventory_loaded()
        )
    }
}

/// Internal state for the Genja runtime builder.
///
/// This structure holds all the configuration components needed to construct a Genja runtime
/// instance. It is wrapped in an `Option` within `PyGenjaBuilder` to enable the builder
/// consumption pattern, where the state is taken out when `build()` is called.
///
/// # Fields
///
/// * `inventory` - The inventory structure containing hosts, groups, and defaults that will
///   be used by the runtime for task execution.
/// * `settings` - Optional runtime settings for configuring behavior such as logging, runner
///   selection, and plugin options. If `None`, default settings will be used when building
///   the runtime.
/// * `plugin_manager` - The plugin manager instance that provides access to inventory, runner,
///   and transform plugins. This is used to load and manage all plugins available to the runtime.
/// * `runner` - Optional name of the runner plugin to use for task execution. If `None`, the
///   default runner (typically "threaded") will be selected when building the runtime.
struct PyGenjaBuilderState {
    inventory: Inventory,
    settings: Option<genja_core::Settings>,
    plugin_manager: genja_plugin_manager::PluginManager,
    runner: Option<String>,
}

/// Python wrapper for the Genja runtime builder.
///
/// This class provides a builder pattern interface for constructing Genja runtime instances
/// with flexible configuration options. The builder allows method chaining to configure
/// various aspects of the runtime before calling `build()` to create the final instance.
///
/// The builder follows a consumption pattern where it can only be used once. After `build()`
/// is called, the builder is consumed and cannot be reused. This is enforced by wrapping the
/// internal state in a `Mutex<Option<PyGenjaBuilderState>>` and taking the state out on build.
///
/// # Thread Safety
///
/// The builder is thread-safe and uses a mutex to protect its internal state. However, since
/// the builder is consumed on `build()`, concurrent access is typically not an issue in practice.
///
/// # Examples
///
/// ```python
/// # Create and configure a builder
/// builder = Genja.builder(hosts)
/// builder = builder.with_runner("threaded")
/// runtime = builder.build()
///
/// # Builder is consumed after build()
/// # builder.build()  # This would raise an error
/// ```
#[pyclass(name = "GenjaBuilder")]
pub struct PyGenjaBuilder {
    inner: Mutex<Option<PyGenjaBuilderState>>,
}

#[pymethods]
impl PyGenjaBuilder {
    /// Registers a Python plugin with the builder's plugin manager.
    ///
    /// This method adds a custom Python plugin to the builder's plugin manager, allowing
    /// the plugin to be used by the runtime once it's built. The plugin can be an inventory
    /// plugin, runner plugin, or transform plugin. The builder is consumed and a new builder
    /// instance with the registered plugin is returned, following the builder pattern.
    ///
    /// # Parameters
    ///
    /// * `plugin` - A Python object representing the plugin to register. The plugin must
    ///   implement the appropriate plugin interface (e.g., `InventoryPlugin`, `RunnerPlugin`,
    ///   or `TransformPlugin`) and will be registered based on its type and name.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing a new builder instance with the plugin registered
    /// on success, or a `PyValueError` if the builder has already been consumed, the plugin
    /// cannot be registered, or the plugin manager is inaccessible.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The builder has already been consumed by a previous `build()` call
    /// - The plugin object does not implement a valid plugin interface
    /// - The plugin registration fails due to naming conflicts or invalid plugin structure
    /// - The plugin manager lock is poisoned or inaccessible
    fn with_plugin(&self, plugin: Bound<'_, PyAny>) -> PyResult<Self> {
        let mut state = self.take_state()?;
        register_python_plugin_on_manager(&mut state.plugin_manager, plugin.unbind())?;
        Ok(Self {
            inner: Mutex::new(Some(state)),
        })
    }

    /// Replaces the builder's plugin manager with a new one.
    ///
    /// This method allows you to provide a pre-configured plugin manager to the builder,
    /// replacing the default or previously set plugin manager. This is useful when you need
    /// to share a plugin manager across multiple runtime instances or when you've already
    /// configured a plugin manager with custom plugins. The builder is consumed and a new
    /// builder instance with the specified plugin manager is returned.
    ///
    /// # Parameters
    ///
    /// * `plugin_manager` - A reference to a `PyPluginManager` instance that will replace
    ///   the current plugin manager. The plugin manager's internal state is taken, so the
    ///   provided plugin manager instance will be consumed.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing a new builder instance with the specified plugin
    /// manager on success, or a `PyValueError` if the builder has already been consumed or
    /// the plugin manager cannot be accessed.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The builder has already been consumed by a previous `build()` call
    /// - The plugin manager's internal state cannot be taken (e.g., if it's locked or poisoned)
    /// - The builder's internal lock is poisoned or inaccessible
    fn with_plugin_manager(&self, plugin_manager: PyRef<'_, PyPluginManager>) -> PyResult<Self> {
        let mut state = self.take_state()?;
        state.plugin_manager = plugin_manager.take_inner()?;
        Ok(Self {
            inner: Mutex::new(Some(state)),
        })
    }

    /// Configures the builder to use a specific runner plugin.
    ///
    /// This method sets the runner plugin that will be used by the runtime for task execution.
    /// The runner determines how tasks are executed across hosts (e.g., serial, threaded, or
    /// custom). The builder is consumed and a new builder instance with the specified runner
    /// is returned, following the builder pattern.
    ///
    /// # Parameters
    ///
    /// * `runner` - The name of the runner plugin to use. Common values include "serial" for
    ///   sequential execution, "threaded" for parallel execution, or the name of a custom
    ///   runner plugin registered with the plugin manager.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<Self>` containing a new builder instance configured with the
    /// specified runner on success, or a `PyValueError` if the builder has already been
    /// consumed or the builder's internal state is inaccessible.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The builder has already been consumed by a previous `build()` call
    /// - The builder's internal lock is poisoned or inaccessible
    fn with_runner(&self, runner: &str) -> PyResult<Self> {
        let mut state = self.take_state()?;
        state.runner = Some(runner.to_string());
        Ok(Self {
            inner: Mutex::new(Some(state)),
        })
    }

    /// Builds and returns the configured Genja runtime instance.
    ///
    /// This method consumes the builder and constructs a `PyGenja` runtime instance using
    /// the accumulated configuration (inventory, settings, plugin manager, and runner).
    /// After calling this method, the builder is consumed and cannot be reused. The runtime
    /// is fully initialized and ready to execute tasks.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<PyGenja>` containing the fully configured runtime instance on
    /// success, or a `PyValueError` if the builder has already been consumed, the runtime
    /// cannot be built from the current configuration, or the specified runner plugin cannot
    /// be found or initialized.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The builder has already been consumed by a previous `build()` call
    /// - The runtime cannot be constructed from the provided inventory and settings
    /// - The specified runner plugin is not registered in the plugin manager
    /// - The runner plugin fails to initialize or configure
    /// - The builder's internal lock is poisoned or inaccessible
    fn build(&self) -> PyResult<PyGenja> {
        let state = self.take_state()?;
        build_runtime(
            state.inventory,
            state.settings,
            state.plugin_manager,
            state.runner.as_deref(),
        )
    }

    /// Returns a string representation of the builder instance.
    ///
    /// This method provides a human-readable representation of the builder's state,
    /// showing whether the builder has been consumed and which runner (if any) has been
    /// configured. It's primarily used for debugging and logging purposes, and is
    /// automatically called by Python's `repr()` function and when displaying the object
    /// in interactive environments.
    ///
    /// # Returns
    ///
    /// Returns a `String` in the format `"GenjaBuilder(consumed=<bool>, runner=<runner>)"`,
    /// where `<bool>` indicates whether the builder has been consumed by a `build()` call,
    /// and `<runner>` is the name of the configured runner plugin or "None" if no runner
    /// has been set. If the builder's internal lock is poisoned or inaccessible, returns
    /// `"GenjaBuilder(<unavailable>)"`.
    fn __repr__(&self) -> String {
        match self.lock_inner() {
            Ok(guard) => {
                let consumed = guard.is_none();
                let runner = guard
                    .as_ref()
                    .and_then(|state| state.runner.as_deref())
                    .unwrap_or("None");
                format!("GenjaBuilder(consumed={consumed}, runner={runner})")
            }
            Err(_) => "GenjaBuilder(<unavailable>)".to_string(),
        }
    }
}

impl PyGenjaBuilder {
    /// Acquires a lock on the builder's internal state.
    ///
    /// This method obtains a mutex guard that provides exclusive access to the builder's
    /// internal state. The state is wrapped in an `Option` to support the consumption
    /// pattern used by the builder, where the state is taken out when `build()` is called.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<std::sync::MutexGuard<'_, Option<PyGenjaBuilderState>>>` containing
    /// the mutex guard on success, or a `PyValueError` if the mutex is poisoned (which occurs
    /// when a thread panicked while holding the lock).
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The mutex is poisoned due to a panic in another thread while holding the lock
    fn lock_inner(&self) -> PyResult<std::sync::MutexGuard<'_, Option<PyGenjaBuilderState>>> {
        self.inner
            .lock()
            .map_err(|_| PyValueError::new_err("genja builder lock is poisoned"))
    }

    /// Takes ownership of the builder's internal state.
    ///
    /// This method acquires the builder's lock and extracts the internal state, leaving
    /// `None` in its place. This implements the consumption pattern where the builder can
    /// only be used once. After this method is called successfully, subsequent calls will
    /// fail with an error indicating the builder has been consumed.
    ///
    /// # Returns
    ///
    /// Returns a `PyResult<PyGenjaBuilderState>` containing the builder's internal state
    /// on success, or a `PyValueError` if the builder has already been consumed or if the
    /// mutex lock cannot be acquired.
    ///
    /// # Errors
    ///
    /// This method will return an error if:
    /// - The builder has already been consumed by a previous call to this method or `build()`
    /// - The mutex is poisoned due to a panic in another thread while holding the lock
    fn take_state(&self) -> PyResult<PyGenjaBuilderState> {
        let mut guard = self.lock_inner()?;
        guard
            .take()
            .ok_or_else(|| PyValueError::new_err("genja builder has already been consumed"))
    }
}
/// Registers the Genja runtime classes with the Python module.
///
/// This function adds the `PyGenja` and `PyGenjaBuilder` classes to the specified
/// Python module, making them available for import and use in Python code. This is
/// typically called during module initialization to expose the Rust-based Genja
/// runtime functionality to Python.
///
/// # Parameters
///
/// * `module` - A reference to the Python module where the Genja classes will be
///   registered. The module must be a valid `PyModule` instance.
///
/// # Returns
///
/// Returns `PyResult<()>` indicating success, or a `PyErr` if class registration
/// fails (e.g., due to naming conflicts or module initialization errors).
pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyGenja>()?;
    module.add_class::<PyGenjaBuilder>()?;
    Ok(())
}

/// Converts a serializable Rust entity to a Python dictionary.
///
/// This function serializes a Rust type to JSON and then converts it to a Python
/// dictionary object. It's used internally to transform Rust data structures into
/// Python-compatible representations for use in the Python API.
///
/// # Parameters
///
/// * `py` - The Python GIL token required for creating Python objects and performing
///   conversions between Rust and Python types.
/// * `entity` - A reference to the Rust entity to convert. The entity must implement
///   the `serde::Serialize` trait.
/// * `error_context` - A string describing the context of the conversion, used to
///   provide meaningful error messages if the conversion fails.
///
/// # Returns
///
/// Returns a `PyResult<Py<PyAny>>` containing the Python dictionary representation
/// of the entity on success, or a `PyValueError` if serialization or conversion fails.
fn entity_to_py_dict<T>(py: Python<'_>, entity: &T, error_context: &str) -> PyResult<Py<PyAny>>
where
    T: serde::Serialize,
{
    let value = serde_json::to_value(entity)
        .map_err(|err| PyValueError::new_err(format!("{error_context}: {err}")))?;
    task::json_value_to_py(py, &value)
}

/// Converts inventory hosts to a Python dictionary.
///
/// This function transforms a collection of inventory hosts into a Python dictionary
/// where keys are host IDs and values are host payload dictionaries. Each host is
/// serialized and converted to a Python-compatible format.
///
/// # Parameters
///
/// * `py` - The Python GIL token required for creating Python objects and performing
///   conversions between Rust and Python types.
/// * `hosts` - A reference to the `Hosts` collection containing the inventory hosts
///   to convert.
///
/// # Returns
///
/// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary mapping host IDs
/// to host payload dictionaries on success, or a `PyValueError` if host conversion
/// fails.
fn inventory_hosts_to_py_dict(py: Python<'_>, hosts: &Hosts) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    for (host_id, host) in hosts.iter() {
        payload.set_item(
            host_id.as_str(),
            entity_to_py_dict(py, &host, "failed to convert host payload")?,
        )?;
    }
    Ok(payload.into_any().unbind())
}

/// Converts inventory groups to a Python dictionary.
///
/// This function transforms a collection of inventory groups into a Python dictionary
/// where keys are group IDs and values are group payload dictionaries. Each group is
/// serialized and converted to a Python-compatible format.
///
/// # Parameters
///
/// * `py` - The Python GIL token required for creating Python objects and performing
///   conversions between Rust and Python types.
/// * `groups` - A reference to the `Groups` collection containing the inventory groups
///   to convert.
///
/// # Returns
///
/// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary mapping group IDs
/// to group payload dictionaries on success, or a `PyValueError` if group conversion
/// fails.
fn groups_to_py_dict(py: Python<'_>, groups: &Groups) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    for (group_id, group) in groups.iter() {
        payload.set_item(
            group_id.as_str(),
            entity_to_py_dict(py, &group, "failed to convert group payload")?,
        )?;
    }
    Ok(payload.into_any().unbind())
}

/// Converts inventory defaults to a Python object.
///
/// This function transforms the inventory defaults structure into a Python dictionary
/// by serializing and converting it to a Python-compatible format.
///
/// # Parameters
///
/// * `py` - The Python GIL token required for creating Python objects and performing
///   conversions between Rust and Python types.
/// * `defaults` - A reference to the `Defaults` structure containing the inventory
///   defaults to convert.
///
/// # Returns
///
/// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary representing the
/// defaults on success, or a `PyValueError` if defaults conversion fails.
fn defaults_to_py(py: Python<'_>, defaults: &Defaults) -> PyResult<Py<PyAny>> {
    entity_to_py_dict(py, defaults, "failed to convert defaults payload")
}

/// Converts a complete inventory structure to a Python dictionary.
///
/// This function transforms the full inventory including transformed hosts (with groups
/// and defaults applied), groups, and defaults into a Python dictionary. The returned
/// structure contains three keys: "hosts", "groups", and "defaults". Groups and defaults
/// are set to `None` if not present in the inventory.
///
/// # Parameters
///
/// * `py` - The Python GIL token required for creating Python objects and performing
///   conversions between Rust and Python types.
/// * `inventory` - A reference to the `Inventory` structure containing the complete
///   inventory to convert, including transformed hosts, groups, and defaults.
///
/// # Returns
///
/// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary with keys "hosts",
/// "groups", and "defaults" on success, or a `PyValueError` if any inventory component
/// cannot be converted to Python dictionaries.
fn inventory_to_py_dict(py: Python<'_>, inventory: &Inventory) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    let hosts = PyDict::new(py);
    for (host_id, host) in inventory.hosts().iter() {
        hosts.set_item(
            host_id.as_str(),
            entity_to_py_dict(py, &host, "failed to convert transformed host payload")?,
        )?;
    }
    payload.set_item("hosts", hosts)?;

    match inventory.groups() {
        Some(groups) => {
            let groups_payload = PyDict::new(py);
            for (group_id, group) in groups.iter() {
                groups_payload.set_item(
                    group_id.as_str(),
                    entity_to_py_dict(py, &group, "failed to convert transformed group payload")?,
                )?;
            }
            payload.set_item("groups", groups_payload)?;
        }
        None => payload.set_item("groups", py.None())?,
    }

    match inventory.defaults() {
        Some(defaults) => payload.set_item("defaults", defaults_to_py(py, &defaults)?)?,
        None => payload.set_item("defaults", py.None())?,
    }

    Ok(payload.into_any().unbind())
}

/// Converts a raw inventory structure to a Python dictionary.
///
/// This function transforms the complete raw inventory including hosts, groups, and defaults
/// before any transformations are applied into a Python dictionary. The returned structure
/// contains three keys: "hosts", "groups", and "defaults", all in their original form as
/// loaded from the inventory source. Groups and defaults are set to `None` if not present
/// in the inventory.
///
/// # Parameters
///
/// * `py` - The Python GIL token required for creating Python objects and performing
///   conversions between Rust and Python types.
/// * `inventory` - A reference to the `Inventory` structure containing the complete raw
///   inventory to convert, including untransformed hosts, groups, and defaults.
///
/// # Returns
///
/// Returns a `PyResult<Py<PyAny>>` containing a Python dictionary with keys "hosts",
/// "groups", and "defaults" representing the raw, untransformed inventory on success,
/// or a `PyValueError` if any raw inventory component cannot be converted to Python
/// dictionaries.
fn raw_inventory_to_py_dict(py: Python<'_>, inventory: &Inventory) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    payload.set_item(
        "hosts",
        inventory_hosts_to_py_dict(py, inventory.hosts_raw())?,
    )?;
    match inventory.groups_raw() {
        Some(groups) => payload.set_item("groups", groups_to_py_dict(py, groups)?)?,
        None => payload.set_item("groups", py.None())?,
    }
    match inventory.defaults_raw() {
        Some(defaults) => payload.set_item("defaults", defaults_to_py(py, defaults)?)?,
        None => payload.set_item("defaults", py.None())?,
    }
    Ok(payload.into_any().unbind())
}

/// Converts a Python dictionary of hosts to a Rust inventory structure.
///
/// This function transforms a Python dictionary mapping host IDs to host payloads into
/// a Rust `Inventory` structure. Each host payload is converted from Python to Rust
/// format and added to the inventory's hosts collection. The resulting inventory contains
/// only hosts without groups or defaults.
///
/// # Parameters
///
/// * `obj` - A Python object that must be a dictionary mapping host IDs (strings) to
///   host payload dictionaries. Each host payload should contain valid host configuration
///   fields such as hostname, platform, port, etc.
///
/// # Returns
///
/// Returns a `PyResult<Inventory>` containing the constructed inventory with the converted
/// hosts on success, or a `PyValueError` if the input is not a dictionary, host IDs cannot
/// be extracted as strings, or host payloads cannot be converted to Rust host structures.
///
/// # Errors
///
/// This method will return an error if:
/// - The input object is not a Python dictionary
/// - Any host ID cannot be extracted as a string
/// - Any host payload cannot be converted to a valid Rust host structure
pub(crate) fn python_hosts_to_inventory(obj: Bound<'_, PyAny>) -> PyResult<Inventory> {
    let dict = obj.downcast::<PyDict>().map_err(|_| {
        PyValueError::new_err("hosts must be a dict mapping host id to host payload")
    })?;

    let mut hosts = Hosts::new();
    for (host_id, host_obj) in dict.iter() {
        let host_id: String = host_id.extract()?;
        let host = task::python_host_to_rust_host(host_obj)?;
        hosts.add_host(host_id, host);
    }

    Ok(Inventory::builder().hosts(hosts).build())
}

/// Converts a Python inventory object to a Rust inventory structure.
///
/// This function transforms a Python object representing an inventory into a Rust `Inventory`
/// structure. It accepts two formats: a dictionary mapping host IDs to host payloads (simple
/// format), or a complete inventory structure with "hosts", "groups", and "defaults" keys
/// (full format). The function automatically detects which format is provided and converts
/// accordingly. Python objects with `model_dump()` or `to_dict()` methods (such as Pydantic
/// models) are automatically normalized before conversion.
///
/// # Parameters
///
/// * `obj` - A Python object representing the inventory. This can be:
///   - A dictionary mapping host IDs to host payloads (simple format)
///   - A dictionary with "hosts", "groups", and/or "defaults" keys (full format)
///   - A Pydantic model or object with `model_dump()` or `to_dict()` methods
///
/// # Returns
///
/// Returns a `PyResult<Inventory>` containing the constructed Rust inventory structure on
/// success, or a `PyValueError` if the input cannot be converted to a valid inventory
/// (e.g., due to invalid structure, missing required fields, or type mismatches).
///
/// # Errors
///
/// This method will return an error if:
/// - The input object cannot be normalized or converted to a dictionary
/// - The inventory structure contains invalid host, group, or defaults data
/// - Required fields are missing or have incorrect types
/// - JSON serialization or deserialization fails during conversion
pub(crate) fn python_inventory_to_rust_inventory(obj: Bound<'_, PyAny>) -> PyResult<Inventory> {
    let normalized = normalize_python_mapping_payload(obj)?;
    if let Ok(dict) = normalized.clone().downcast::<PyDict>() {
        let has_inventory_keys =
            dict.contains("hosts")? || dict.contains("groups")? || dict.contains("defaults")?;
        if has_inventory_keys {
            return python_json_to_rust(normalized, "invalid inventory payload");
        }
    }

    python_hosts_to_inventory(normalized)
}

/// Converts a Python object to a Rust type via JSON serialization.
///
/// This function converts a Python object to a Rust type by first normalizing the Python
/// object (handling Pydantic models and objects with `to_dict()` methods), then serializing
/// it to JSON using Python's `json` module, and finally deserializing it to the target Rust
/// type using `serde_json`. This approach ensures compatibility with complex Python objects
/// and provides consistent conversion behavior.
///
/// # Parameters
///
/// * `obj` - A Python object to convert. The object will be normalized before conversion
///   to handle Pydantic models and other special Python types.
/// * `error_context` - A string describing the context of the conversion, used to provide
///   meaningful error messages if the conversion fails.
///
/// # Returns
///
/// Returns a `PyResult<T>` containing the deserialized Rust value on success, or a
/// `PyValueError` if normalization, JSON serialization, or deserialization fails.
///
/// # Errors
///
/// This method will return an error if:
/// - The Python object cannot be normalized or converted to a JSON-serializable format
/// - Python's `json.dumps()` fails to serialize the normalized object
/// - The JSON string cannot be deserialized to the target Rust type `T`
/// - The deserialized structure does not match the expected schema for type `T`
fn python_json_to_rust<T>(obj: Bound<'_, PyAny>, error_context: &str) -> PyResult<T>
where
    T: DeserializeOwned,
{
    let normalized = normalize_python_mapping_payload(obj)?;

    let json_module = PyModule::import(normalized.py(), "json")?;
    let dumped: String = json_module
        .call_method1("dumps", (normalized,))?
        .extract()?;
    serde_json::from_str(&dumped)
        .map_err(|err| PyValueError::new_err(format!("{error_context}: {err}")))
}

/// Normalizes a Python mapping object to a standard dictionary format.
///
/// This function handles different Python object types and converts them to a standard
/// dictionary format suitable for further processing. It specifically handles Pydantic
/// models (via `model_dump()` method) and objects with `to_dict()` methods, ensuring
/// consistent conversion behavior across different Python object types. For Pydantic
/// models, the conversion uses JSON mode and excludes `None` values.
///
/// # Parameters
///
/// * `obj` - A Python object to normalize. This can be a dictionary, Pydantic model,
///   dataclass, or any object with a `to_dict()` method. If the object is already a
///   standard dictionary or has no special conversion methods, it is returned as-is.
///
/// # Returns
///
/// Returns a `PyResult<Bound<'_, PyAny>>` containing the normalized Python object on
/// success, or a `PyErr` if attribute checking or method invocation fails.
///
/// # Errors
///
/// This method will return an error if:
/// - Attribute checking (`hasattr`) fails due to Python exceptions
/// - The `model_dump()` or `to_dict()` method invocation raises an exception
/// - Method arguments cannot be constructed or passed correctly
fn normalize_python_mapping_payload(obj: Bound<'_, PyAny>) -> PyResult<Bound<'_, PyAny>> {
    if obj.hasattr("model_dump")? {
        let kwargs = PyDict::new(obj.py());
        kwargs.set_item("mode", "json")?;
        kwargs.set_item("exclude_none", true)?;
        obj.call_method("model_dump", (), Some(&kwargs))
    } else if obj.hasattr("to_dict")? {
        obj.call_method0("to_dict")
    } else {
        Ok(obj)
    }
}

/// Builds a Genja runtime instance from the provided components.
///
/// This function constructs a `PyGenja` runtime by assembling the inventory, settings,
/// plugin manager, and optional runner configuration. It uses the builder pattern to
/// configure the runtime and handles errors during construction. If a runner is specified,
/// it will be selected and configured for the runtime.
///
/// # Parameters
///
/// * `inventory` - The inventory structure containing hosts, groups, and defaults that
///   will be used by the runtime for task execution.
/// * `settings` - Optional runtime settings for configuring behavior such as logging,
///   runner selection, and plugin options. If `None`, default settings will be used.
/// * `plugin_manager` - The plugin manager instance that provides access to inventory,
///   runner, and transform plugins used by the runtime.
/// * `runner` - Optional name of the runner plugin to use for task execution. If `None`,
///   the default runner will be used. Common values include "serial" for sequential
///   execution or "threaded" for parallel execution.
///
/// # Returns
///
/// Returns a `PyResult<PyGenja>` containing the fully configured runtime instance on
/// success, or a `PyValueError` if the runtime cannot be built from the provided
/// configuration or if the specified runner plugin cannot be found or initialized.
///
/// # Errors
///
/// This function will return an error if:
/// - The runtime cannot be constructed from the provided inventory and settings
/// - The specified runner plugin is not registered in the plugin manager
/// - The runner plugin fails to initialize or configure
fn build_runtime(
    inventory: Inventory,
    settings: Option<genja_core::Settings>,
    plugin_manager: genja_plugin_manager::PluginManager,
    runner: Option<&str>,
) -> PyResult<PyGenja> {
    let mut builder = RuntimeGenja::builder(inventory).with_plugin_manager(plugin_manager);
    if let Some(settings) = settings {
        builder = builder.with_settings(settings);
    }
    let mut inner = builder
        .build()
        .map_err(|err| PyValueError::new_err(format!("failed to build Genja runtime: {err}")))?;
    if let Some(runner) = runner {
        inner = inner.with_runner(runner).map_err(|err| {
            PyValueError::new_err(format!("failed to select runner {runner}: {err}"))
        })?;
    }
    Ok(PyGenja { inner })
}

/// Builds a Genja runtime instance from settings configuration.
///
/// This function constructs a `PyGenja` runtime by first loading the inventory from
/// the provided settings using the appropriate inventory plugin, then building the
/// runtime with the loaded inventory, settings, plugin manager, and optional runner.
/// This is a convenience function that combines inventory loading and runtime building
/// into a single operation.
///
/// # Parameters
///
/// * `settings` - The runtime settings containing inventory configuration, runner
///   selection, and other runtime options. The settings specify which inventory plugin
///   to use and how to configure it.
/// * `plugin_manager` - The plugin manager instance that provides access to inventory,
///   runner, and transform plugins used by the runtime.
/// * `runner` - Optional name of the runner plugin to use for task execution. If `None`,
///   the runner specified in settings or the default runner will be used.
///
/// # Returns
///
/// Returns a `PyResult<PyGenja>` containing the fully configured runtime instance on
/// success, or a `PyValueError` if the inventory cannot be loaded from settings or if
/// the runtime cannot be built.
///
/// # Errors
///
/// This function will return an error if:
/// - The inventory cannot be loaded from the settings configuration
/// - The specified inventory plugin is not found or fails to load
/// - The runtime cannot be constructed from the loaded inventory and settings
/// - The specified runner plugin cannot be found or initialized
fn build_runtime_from_settings(
    settings: Settings,
    plugin_manager: genja_plugin_manager::PluginManager,
    runner: Option<&str>,
) -> PyResult<PyGenja> {
    let inventory = load_inventory_from_settings(&settings, &plugin_manager)
        .map_err(|err| PyValueError::new_err(format!("failed to build Genja runtime: {err}")))?;
    build_runtime(inventory, Some(settings), plugin_manager, runner)
}

/// Loads inventory from settings using the configured inventory plugin.
///
/// This function loads the inventory by selecting and invoking the appropriate inventory
/// plugin based on the settings configuration. It first checks if a specific plugin is
/// configured in the settings. If no plugin is specified, it falls back to the default
/// "FileInventoryPlugin". The function validates that the selected plugin exists and is
/// actually an inventory plugin before attempting to load.
///
/// # Parameters
///
/// * `settings` - A reference to the runtime settings containing the inventory plugin
///   configuration and options. The settings specify which inventory plugin to use and
///   provide plugin-specific configuration options.
/// * `plugin_manager` - A reference to the plugin manager that provides access to
///   registered inventory plugins. The plugin manager is used to look up and retrieve
///   the configured inventory plugin.
///
/// # Returns
///
/// Returns a `Result<Inventory, GenjaError>` containing the loaded inventory structure
/// on success, or a `GenjaError` if the plugin cannot be found, is not an inventory
/// plugin, or fails to load the inventory.
///
/// # Errors
///
/// This function will return an error if:
/// - The configured inventory plugin is not found in the plugin manager
/// - The configured plugin exists but is not an inventory plugin
/// - The inventory plugin fails to load the inventory from the settings
/// - The default "FileInventoryPlugin" cannot be found when no plugin is specified
fn load_inventory_from_settings(
    settings: &Settings,
    plugin_manager: &genja_plugin_manager::PluginManager,
) -> Result<Inventory, GenjaError> {
    let inventory_cfg = settings.inventory();
    let plugin_name = inventory_cfg.plugin();

    if !plugin_name.is_empty() {
        if let Some(plugin) = plugin_manager.get_inventory_plugin(plugin_name) {
            return plugin
                .load(settings, plugin_manager)
                .map_err(GenjaError::from);
        }

        if plugin_manager.get_plugin(plugin_name).is_some() {
            return Err(GenjaError::NotInventoryPlugin(plugin_name.to_string()));
        }

        return Err(GenjaError::PluginNotFound(plugin_name.to_string()));
    }

    let default_name = "FileInventoryPlugin";
    if let Some(plugin) = plugin_manager.get_inventory_plugin(default_name) {
        return plugin
            .load(settings, plugin_manager)
            .map_err(GenjaError::from);
    }

    if plugin_manager.get_plugin(default_name).is_some() {
        return Err(GenjaError::NotInventoryPlugin(default_name.to_string()));
    }

    Err(GenjaError::PluginNotFound(default_name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugin_manager::PyPluginManager;
    use pyo3::types::PyString;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::Once;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn init_python() {
        static INIT: Once = Once::new();
        INIT.call_once(pyo3::prepare_freethreaded_python);
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock should be after unix epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!(
            "genja-core-python-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).expect("temp test dir should be created");
        dir
    }

    #[test]
    fn python_hosts_to_inventory_converts_host_dict() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);

            let router1 = PyDict::new(py);
            router1.set_item("hostname", "10.0.0.1").unwrap();
            router1.set_item("platform", "ios").unwrap();

            let router2 = PyDict::new(py);
            router2.set_item("hostname", "10.0.0.2").unwrap();
            router2.set_item("port", 2222).unwrap();
            router2.set_item("platform", "nxos").unwrap();

            hosts.set_item("router1", router1).unwrap();
            hosts.set_item("router2", router2).unwrap();

            let inventory =
                python_hosts_to_inventory(hosts.into_any()).expect("hosts should convert");
            let inventory_hosts = inventory.hosts();

            assert_eq!(inventory_hosts.len(), 2);
            assert_eq!(
                inventory_hosts
                    .get("router1")
                    .expect("router1 should exist")
                    .hostname(),
                Some("10.0.0.1")
            );
            assert_eq!(
                inventory_hosts
                    .get("router2")
                    .expect("router2 should exist")
                    .port(),
                Some(2222)
            );
        });
    }

    #[test]
    fn python_hosts_to_inventory_rejects_non_dict_input() {
        init_python();
        Python::with_gil(|py| {
            let not_a_dict = PyString::new(py, "not-a-dict");

            let err = python_hosts_to_inventory(not_a_dict.into_any())
                .err()
                .expect("non-dict input should fail");
            assert!(err
                .to_string()
                .contains("hosts must be a dict mapping host id to host payload"));
        });
    }

    #[test]
    fn python_inventory_to_rust_inventory_accepts_hosts_key() {
        init_python();
        Python::with_gil(|py| {
            let inventory = PyDict::new(py);
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router).unwrap();
            inventory.set_item("hosts", hosts).unwrap();

            let inventory = python_inventory_to_rust_inventory(inventory.into_any())
                .expect("inventory payload should convert");
            assert_eq!(
                inventory
                    .hosts()
                    .get("router1")
                    .expect("router1 should exist")
                    .hostname(),
                Some("10.0.0.1")
            );
        });
    }

    #[test]
    fn python_inventory_to_rust_inventory_accepts_groups_and_defaults() {
        init_python();
        Python::with_gil(|py| {
            let inventory = PyDict::new(py);

            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("groups", vec!["core", "site-a"]).unwrap();
            hosts.set_item("router1", router).unwrap();

            let groups = PyDict::new(py);
            let core = PyDict::new(py);
            core.set_item("platform", "ios").unwrap();
            groups.set_item("core", core).unwrap();
            let site = PyDict::new(py);
            site.set_item(
                "data",
                task::json_value_to_py(py, &serde_json::json!({"site": "a"})).unwrap(),
            )
            .unwrap();
            groups.set_item("site-a", site).unwrap();

            let defaults = PyDict::new(py);
            defaults.set_item("username", "admin").unwrap();
            defaults.set_item("port", 22).unwrap();

            inventory.set_item("hosts", hosts).unwrap();
            inventory.set_item("groups", groups).unwrap();
            inventory.set_item("defaults", defaults).unwrap();

            let inventory = python_inventory_to_rust_inventory(inventory.into_any())
                .expect("inventory payload should convert");
            assert_eq!(
                inventory
                    .hosts()
                    .get("router1")
                    .expect("router1 should exist")
                    .groups()
                    .expect("router1 groups should exist")
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>(),
                vec!["core", "site-a"]
            );
            assert_eq!(
                inventory
                    .groups()
                    .expect("groups should exist")
                    .get("core")
                    .expect("core group should exist")
                    .platform(),
                Some("ios")
            );
            assert_eq!(
                inventory
                    .defaults()
                    .expect("defaults should exist")
                    .username(),
                Some("admin")
            );
        });
    }

    #[test]
    fn python_inventory_to_rust_inventory_fails_with_invalid_host_structure() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            hosts.set_item("router1", "not-a-dict").unwrap();

            let err = python_inventory_to_rust_inventory(hosts.into_any())
                .err()
                .expect("invalid host structure should fail");
            assert!(err.to_string().contains("invalid host payload"));
        });
    }

    #[test]
    fn py_genja_from_hosts_builds_runtime() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router).unwrap();

            let runtime =
                PyGenja::from_hosts(hosts.into_any(), None, None).expect("runtime should build");

            assert!(runtime.plugins_loaded());
            assert!(runtime.inventory_loaded());
            assert_eq!(runtime.host_count(), 1);
            assert_eq!(runtime.host_ids(), vec!["router1".to_string()]);
            assert_eq!(runtime.settings().runner().plugin(), "threaded");
            assert!(runtime.__repr__().contains("Genja("));
        });
    }

    #[test]
    fn py_genja_from_inventory_builds_runtime_with_groups_and_defaults() {
        init_python();
        Python::with_gil(|py| {
            let inventory = PyDict::new(py);

            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("groups", vec!["core"]).unwrap();
            hosts.set_item("router1", router).unwrap();

            let groups = PyDict::new(py);
            let core = PyDict::new(py);
            core.set_item("platform", "ios").unwrap();
            groups.set_item("core", core).unwrap();

            let defaults = PyDict::new(py);
            defaults.set_item("username", "admin").unwrap();

            inventory.set_item("hosts", hosts).unwrap();
            inventory.set_item("groups", groups).unwrap();
            inventory.set_item("defaults", defaults).unwrap();

            let runtime = PyGenja::from_inventory(inventory.into_any(), None, None)
                .expect("runtime should build from full inventory");

            let full_inventory = runtime
                .inventory_full(py)
                .expect("inventory_full should work");
            let full_inventory: Bound<'_, PyDict> =
                full_inventory.bind(py).clone().downcast_into().unwrap();
            let full_groups: Bound<'_, PyDict> = full_inventory
                .get_item("groups")
                .unwrap()
                .expect("groups should exist")
                .downcast_into()
                .unwrap();
            assert_eq!(
                full_groups
                    .get_item("core")
                    .unwrap()
                    .expect("core group should exist")
                    .get_item("platform")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "ios"
            );

            let raw_inventory = runtime
                .inventory_raw(py)
                .expect("inventory_raw should work");
            let raw_inventory: Bound<'_, PyDict> =
                raw_inventory.bind(py).clone().downcast_into().unwrap();
            assert_eq!(
                raw_inventory
                    .get_item("defaults")
                    .unwrap()
                    .expect("defaults should exist")
                    .get_item("username")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "admin"
            );
        });
    }

    #[test]
    fn py_genja_builder_builds_runtime_with_runner() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router).unwrap();

            let builder =
                PyGenja::builder(hosts.into_any(), None, None).expect("builder should be created");
            let builder = builder
                .with_runner("serial")
                .expect("runner should be set on builder");
            let runtime = builder.build().expect("builder should produce runtime");

            assert!(runtime.inner.inventory_loaded());
            assert!(runtime.inner.get_runner_plugin("serial").is_ok());
        });
    }

    #[test]
    fn py_genja_from_hosts_accepts_plugin_manager() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            router.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router).unwrap();

            let plugin_manager =
                Py::new(py, PyPluginManager::new()).expect("plugin manager should be created");
            let plugin_manager_ref = plugin_manager.bind(py).borrow();

            let runtime = PyGenja::from_hosts(hosts.into_any(), None, Some(plugin_manager_ref))
                .expect("runtime should build with explicit plugin manager");

            assert!(runtime.inner.plugins_loaded());
            assert!(runtime.inner.inventory_loaded());
            assert!(runtime.inner.get_runner_plugin("serial").is_ok());
        });
    }

    #[test]
    fn py_genja_run_task_uses_python_runner_plugin() {
        init_python();
        Python::with_gil(|py| {
            let plugin_manager =
                Py::new(py, PyPluginManager::new()).expect("plugin manager should be created");
            let importlib = PyModule::import(py, "importlib").expect("importlib should import");
            let module = importlib
                .call_method1("import_module", ("tests.fixtures.runner_plugins",))
                .expect("runner fixture module should import");
            let plugin_class = module
                .getattr("FirstHostOnlyRunnerPlugin")
                .expect("runner plugin should exist");
            let plugin = plugin_class.call0().expect("plugin instance should build");
            plugin_manager
                .bind(py)
                .call_method1("register_plugin", (plugin,))
                .expect("runner plugin should register");

            let hosts = PyDict::new(py);
            let router1 = PyDict::new(py);
            router1.set_item("hostname", "10.0.0.1").unwrap();
            router1.set_item("platform", "ios").unwrap();
            let router2 = PyDict::new(py);
            router2.set_item("hostname", "10.0.0.2").unwrap();
            router2.set_item("platform", "nxos").unwrap();
            hosts.set_item("router1", router1).unwrap();
            hosts.set_item("router2", router2).unwrap();

            let runtime = PyGenja::from_hosts(
                hosts.into_any(),
                None,
                Some(plugin_manager.bind(py).borrow()),
            )
            .expect("runtime should build");
            let runtime = runtime
                .with_runner("python_runner")
                .expect("python runner should be selectable");

            let task_module = importlib
                .call_method1("import_module", ("tests.fixtures.task_definitions",))
                .expect("task fixture module should import");
            let task_class = task_module
                .getattr("AsyncRuntimeTask")
                .expect("task fixture should exist");
            let results = runtime
                .run_task(py, task_class, Some(5))
                .expect("task should execute through python runner");

            assert_eq!(
                results
                    .inner
                    .passed_hosts()
                    .into_iter()
                    .map(|host| host.to_string())
                    .collect::<Vec<_>>(),
                vec!["10.0.0.1".to_string()]
            );
            assert!(results.inner.failed_hosts().is_empty());
            assert!(results.inner.skipped_hosts().is_empty());
        });
    }

    #[test]
    fn py_genja_builder_consumes_previous_builder_instance() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router = PyDict::new(py);
            router.set_item("hostname", "10.0.0.1").unwrap();
            hosts.set_item("router1", router).unwrap();

            let builder =
                PyGenja::builder(hosts.into_any(), None, None).expect("builder should be created");
            let next_builder = builder
                .with_runner("serial")
                .expect("runner should be set on builder");
            let err = builder
                .build()
                .err()
                .expect("consumed builder should not build twice");
            assert!(err
                .to_string()
                .contains("genja builder has already been consumed"));
            assert!(next_builder.build().is_ok());
        });
    }

    #[test]
    fn py_genja_inventory_accessors_return_host_payloads() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);
            let router1 = PyDict::new(py);
            router1.set_item("hostname", "10.0.0.1").unwrap();
            router1.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router1).unwrap();
            let router2 = PyDict::new(py);
            router2.set_item("hostname", "10.0.0.2").unwrap();
            router2.set_item("platform", "nxos").unwrap();
            hosts.set_item("router2", router2).unwrap();

            let runtime =
                PyGenja::from_hosts(hosts.into_any(), None, None).expect("runtime should build");

            assert_eq!(runtime.host_count(), 2);
            assert_eq!(
                runtime.host_ids(),
                vec!["router1".to_string(), "router2".to_string()]
            );

            let inventory = runtime
                .inventory(py)
                .expect("inventory accessor should work");
            let inventory: Bound<'_, PyDict> = inventory.bind(py).clone().downcast_into().unwrap();
            assert_eq!(
                inventory
                    .get_item("router1")
                    .unwrap()
                    .expect("router1 inventory host should exist")
                    .get_item("hostname")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "10.0.0.1"
            );

            let raw_hosts = runtime
                .hosts_raw(py)
                .expect("hosts_raw accessor should work");
            let raw_hosts: Bound<'_, PyDict> = raw_hosts.bind(py).clone().downcast_into().unwrap();
            assert_eq!(
                raw_hosts
                    .get_item("router1")
                    .unwrap()
                    .expect("router1 raw host should exist")
                    .get_item("platform")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "ios"
            );

            let inventory_hosts = runtime
                .iter_inventory_hosts(py)
                .expect("iter_inventory_hosts should work");
            assert_eq!(inventory_hosts.len(), 2);
            assert_eq!(inventory_hosts[0].0, "router1");
            assert_eq!(
                inventory_hosts[0]
                    .1
                    .bind(py)
                    .get_item("hostname")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "10.0.0.1"
            );

            let selected_hosts = runtime
                .iter_selected_hosts(py)
                .expect("iter_selected_hosts should work");
            assert_eq!(selected_hosts.len(), 2);
            assert_eq!(selected_hosts[1].0, "router2");
            assert_eq!(
                selected_hosts[1]
                    .1
                    .bind(py)
                    .get_item("platform")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "nxos"
            );
        });
    }

    #[test]
    fn py_genja_iter_selected_hosts_respects_filters() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);

            let router1 = PyDict::new(py);
            router1.set_item("hostname", "10.0.0.1").unwrap();
            router1.set_item("platform", "ios").unwrap();
            hosts.set_item("router1", router1).unwrap();

            let router2 = PyDict::new(py);
            router2.set_item("hostname", "10.0.0.2").unwrap();
            router2.set_item("platform", "nxos").unwrap();
            hosts.set_item("router2", router2).unwrap();

            let runtime =
                PyGenja::from_hosts(hosts.into_any(), None, None).expect("runtime should build");
            let filtered = runtime
                .filter_by_key_value("platform", "^ios$")
                .expect("filter_by_key_value should work");

            assert_eq!(filtered.host_count(), 1);
            assert_eq!(filtered.host_ids(), vec!["router1".to_string()]);

            let selected_hosts = filtered
                .iter_selected_hosts(py)
                .expect("iter_selected_hosts should work on filtered runtime");
            assert_eq!(selected_hosts.len(), 1);
            assert_eq!(selected_hosts[0].0, "router1");
        });
    }

    #[test]
    fn py_genja_from_settings_file_accepts_python_inventory_plugin_manager() {
        init_python();
        Python::with_gil(|py| {
            let plugin_manager =
                Py::new(py, PyPluginManager::new()).expect("plugin manager should be created");
            let importlib = PyModule::import(py, "importlib").expect("importlib should import");
            let module = importlib
                .call_method1("import_module", ("tests.fixtures.inventory_plugins",))
                .expect("inventory fixture module should import");
            let plugin_class = module
                .getattr("StaticInventoryPlugin")
                .expect("inventory plugin should exist");
            let plugin = plugin_class.call0().expect("plugin instance should build");
            plugin_manager
                .bind(py)
                .call_method1("register_plugin", (plugin,))
                .expect("inventory plugin should register");

            let temp_dir = temp_test_dir("inventory-settings");
            let settings_path = temp_dir.join("settings.yaml");
            fs::write(
                &settings_path,
                "inventory:\n  plugin: python_inventory\n  options: {}\nrunner:\n  plugin: serial\n",
            )
            .expect("settings file should be written");

            let runtime = PyGenja::from_settings_file(
                settings_path.to_str().unwrap(),
                Some(plugin_manager.bind(py).borrow()),
            )
            .expect("runtime should build from python inventory plugin");

            assert!(runtime.inner.inventory_loaded());
            let inventory = runtime
                .inventory(py)
                .expect("inventory accessor should work");
            let inventory: Bound<'_, PyDict> = inventory.bind(py).clone().downcast_into().unwrap();
            assert_eq!(
                inventory
                    .get_item("router1")
                    .unwrap()
                    .expect("router1 should exist")
                    .get_item("hostname")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "10.10.10.1"
            );
            fs::remove_dir_all(&temp_dir).unwrap_or(());
        });
    }

    #[test]
    fn py_genja_from_settings_file_fails_with_invalid_path() {
        init_python();
        Python::with_gil(|_py| {
            let err = PyGenja::from_settings_file("/nonexistent/path/settings.yaml", None)
                .err()
                .expect("invalid path should fail");
            assert!(err
                .to_string()
                .contains("failed to build Genja runtime from settings file"));
        });
    }

    #[test]
    fn py_genja_from_settings_file_fails_with_malformed_content() {
        init_python();
        Python::with_gil(|_py| {
            let temp_dir = temp_test_dir("malformed-settings");
            let settings_path = temp_dir.join("settings.yaml");
            fs::write(&settings_path, "invalid: yaml: content: [")
                .expect("malformed file should be written");

            let err = PyGenja::from_settings_file(settings_path.to_str().unwrap(), None)
                .err()
                .expect("malformed settings should fail");
            assert!(err
                .to_string()
                .contains("failed to build Genja runtime from settings file"));
            fs::remove_dir_all(&temp_dir).unwrap_or(());
        });
    }

    #[test]
    fn py_genja_from_settings_file_applies_python_transform_plugin() {
        init_python();
        Python::with_gil(|py| {
            let plugin_manager =
                Py::new(py, PyPluginManager::new()).expect("plugin manager should be created");
            let importlib = PyModule::import(py, "importlib").expect("importlib should import");
            let module = importlib
                .call_method1("import_module", ("tests.fixtures.transform_plugins",))
                .expect("transform fixture module should import");
            let plugin_class = module
                .getattr("HostnameSuffixTransformPlugin")
                .expect("transform plugin should exist");
            let plugin = plugin_class.call0().expect("plugin instance should build");
            plugin_manager
                .bind(py)
                .call_method1("register_plugin", (plugin,))
                .expect("transform plugin should register");

            let temp_dir = temp_test_dir("transform-settings");
            let hosts_path = temp_dir.join("hosts.yaml");
            fs::write(
                &hosts_path,
                "router1:\n  hostname: 10.0.0.1\n  platform: ios\n",
            )
            .expect("hosts file should be written");
            let settings_path = temp_dir.join("settings.yaml");
            fs::write(
                &settings_path,
                format!(
                    "inventory:\n  plugin: FileInventoryPlugin\n  options:\n    hosts_file: {}\n  transform_function: python_transform\n  transform_function_options:\n    suffix: -lab\nrunner:\n  plugin: serial\n",
                    hosts_path.display()
                ),
            )
            .expect("settings file should be written");

            let runtime = PyGenja::from_settings_file(
                settings_path.to_str().unwrap(),
                Some(plugin_manager.bind(py).borrow()),
            )
            .expect("runtime should build from settings with python transform");

            let inventory_hosts = runtime
                .iter_inventory_hosts(py)
                .expect("iter_inventory_hosts should work");
            assert_eq!(inventory_hosts.len(), 1);
            assert_eq!(inventory_hosts[0].0, "router1");
            assert_eq!(
                inventory_hosts[0]
                    .1
                    .bind(py)
                    .get_item("hostname")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "10.0.0.1-lab"
            );

            let raw_hosts = runtime.hosts_raw(py).expect("hosts_raw should work");
            let raw_hosts: Bound<'_, PyDict> = raw_hosts.bind(py).clone().downcast_into().unwrap();
            assert_eq!(
                raw_hosts
                    .get_item("router1")
                    .unwrap()
                    .expect("router1 raw host should exist")
                    .get_item("hostname")
                    .unwrap()
                    .extract::<String>()
                    .unwrap(),
                "10.0.0.1"
            );
            fs::remove_dir_all(&temp_dir).unwrap_or(());
        });
    }

    #[test]
    fn py_genja_filter_methods_return_filtered_runtime() {
        init_python();
        Python::with_gil(|py| {
            let hosts = PyDict::new(py);

            let router1 = PyDict::new(py);
            router1.set_item("hostname", "10.0.0.1").unwrap();
            router1.set_item("platform", "ios").unwrap();
            let router1_data = PyDict::new(py);
            let router1_site = PyDict::new(py);
            router1_site.set_item("role", "core").unwrap();
            router1_data.set_item("site", router1_site).unwrap();
            router1.set_item("data", router1_data).unwrap();

            let router2 = PyDict::new(py);
            router2.set_item("hostname", "10.0.0.2").unwrap();
            router2.set_item("platform", "nxos").unwrap();
            let router2_data = PyDict::new(py);
            let router2_site = PyDict::new(py);
            router2_site.set_item("role", "edge").unwrap();
            router2_data.set_item("site", router2_site).unwrap();
            router2.set_item("data", router2_data).unwrap();

            hosts.set_item("router1", router1).unwrap();
            hosts.set_item("router2", router2).unwrap();

            let runtime =
                PyGenja::from_hosts(hosts.into_any(), None, None).expect("runtime should build");

            let filtered = runtime
                .filter_by_key_value("data.site.role", "^core$")
                .expect("filter_by_key_value should work");
            assert_eq!(filtered.inner.host_ids().len(), 1);
            assert_eq!(filtered.inner.host_ids()[0].as_str(), "router1");

            let key_filtered = runtime
                .filter_by_key("data.site.role")
                .expect("filter_by_key should work");
            assert_eq!(key_filtered.inner.host_ids().len(), 2);
        });
    }

    #[test]
    fn register_adds_genja_class_to_module() {
        init_python();
        Python::with_gil(|py| {
            let module =
                PyModule::new(py, "test_runtime_module").expect("test module should be created");

            register(&module).expect("runtime class should register");

            assert!(module.getattr("Genja").is_ok());
            assert!(module.getattr("GenjaBuilder").is_ok());
        });
    }
}
