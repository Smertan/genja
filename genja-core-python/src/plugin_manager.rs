//! Python bindings for the Genja plugin management system.
//!
//! This module provides PyO3-based wrappers that expose Rust plugin functionality
//! to Python code. It enables Python developers to create and register plugins
//! (connection, inventory, runner, transform-function, and processor types) that
//! integrate seamlessly with the Rust plugin system.
//!
//! # Architecture
//!
//! The module uses several adapter patterns to bridge Rust and Python:
//!
//! - **`PyPluginManager`**: A Python-facing wrapper around the Rust `PluginManager`
//!   that handles plugin registration and lifecycle management.
//!
//! - **`PyLoadedPluginRegistry`**: A read-only snapshot of registered plugins,
//!   passed to Python inventory plugins to prevent unsafe access to the full
//!   plugin manager. See the struct documentation for design rationale.
//!
//! - **`PyConnectionPlugin`/`PyConnectionInstance`**: Factory and instance adapters
//!   for Python connection plugins, implementing the two-phase creation pattern.
//!
//! - **`PyInventoryPlugin`**: Adapter for Python inventory plugins that handles
//!   async/sync detection and data conversion.
//!
//! - **`PyTransformFunctionPlugin`**: Adapter for Python transform plugins that
//!   bridge host/group/defaults transforms through JSON-compatible payloads.
//!
//! - **`PyRunnerPlugin`**: Adapter for Python runner plugins that orchestrate
//!   task execution using Python-defined host ordering or rollout behavior.
//!
//! - **`PyProcessorPlugin`**: Adapter for Python processor plugins with lifecycle
//!   hook support.
//!
//! # Safety Considerations
//!
//! Cross-language plugin systems require careful handling of ownership, lifetimes,
//! and thread safety. This module uses several patterns to ensure safety:
//!
//! - Immutable snapshots (`PyLoadedPluginRegistry`) instead of shared references
//! - `Arc<Py<PyAny>>` for shared ownership of Python objects across threads
//! - Mutex-based synchronization for mutable state access
//! - Explicit error conversion between Python and Rust error types

use async_trait::async_trait;
use genja::plugins::built_in_plugin_manager;
use genja_core::InventoryLoadError;
use genja_core::inventory::{
    Connection, ConnectionKey, Defaults, Group, Host, Inventory, ResolvedConnectionParams,
    Transform, TransformFunction, TransformFunctionOptions,
};
use genja_core::settings::{RunnerConfig, Settings};
use genja_core::task::{
    HostTaskResult, TaskConnectionResolver, TaskDefinition, TaskProcessor, TaskProcessorContext,
    TaskResults, Tasks,
};
use genja_plugin_manager::PluginManager;
use genja_plugin_manager::connection_factory::PluginConnectionAdapter;
use genja_plugin_manager::plugin_types::{
    Plugin, PluginConnection, PluginInventory, PluginProcessor, PluginRunner,
    PluginTransformFunction, Plugins,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3_async_runtimes::tokio::into_future;
use serde::{Serialize, de::DeserializeOwned};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::runtime::python_inventory_to_rust_inventory;
use crate::settings::{PyRunnerConfig, PySettings};
use crate::task::{
    PyHostTaskResult, PyTaskConnectionResolver, PyTaskDefinition, PyTaskResults, hosts_to_py_dict,
    python_result_to_host_task_result, python_result_to_task_results,
};

/// A Python-exposed wrapper around the Rust `PluginManager`.
///
/// This struct provides a thread-safe interface to the plugin management system,
/// allowing Python code to register, load, and manage both Rust and Python plugins.
/// The inner `PluginManager` is wrapped in a `Mutex` and `Option` to support
/// safe concurrent access and one-time consumption semantics.
///
/// # Fields
///
/// * `inner` - A mutex-protected optional `PluginManager`. The `Option` allows
///   the manager to be consumed (taken) exactly once, after which subsequent
///   operations will fail with an error indicating the manager has been consumed.
#[pyclass(name = "PluginManager")]
pub struct PyPluginManager {
    inner: Mutex<Option<PluginManager>>,
}

#[pymethods]
impl PyPluginManager {
    /// Creates a new `PyPluginManager` instance with built-in plugins pre-registered.
    ///
    /// This constructor initializes the plugin manager with a default set of built-in
    /// plugins provided by the `genja` crate. The manager is wrapped in a `Mutex` to
    /// ensure thread-safe access from Python code.
    ///
    /// # Returns
    ///
    /// Returns a new `PyPluginManager` instance containing a mutex-protected plugin
    /// manager initialized with built-in plugins. The manager can be used immediately
    /// to register additional plugins or query existing ones.
    #[new]
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Some(built_in_plugin_manager())),
        }
    }

    /// Loads Rust plugins from a specified directory.
    ///
    /// This method scans the given directory for dynamic library files containing
    /// Rust plugins and registers them with the plugin manager. The plugin manager
    /// is temporarily consumed during the loading process and then restored.
    ///
    /// # Parameters
    ///
    /// * `path` - A string slice representing the filesystem path to the directory
    ///   containing the plugin dynamic libraries to load.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all plugins in the directory were successfully loaded,
    /// or a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    /// - Loading plugins from the directory fails (e.g., directory doesn't exist,
    ///   invalid plugin format, or plugin initialization errors)
    ///
    /// # Errors
    ///
    /// This function will return an error if the plugin manager has been consumed
    /// or if any error occurs during the plugin loading process from the specified
    /// directory.
    fn load_rust_plugins_from_directory(&self, path: &str) -> PyResult<()> {
        let mut guard = self.lock_inner()?;
        let manager = guard
            .take()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))?;
        let manager = manager.load_plugins_from_directory(path).map_err(|err| {
            PyValueError::new_err(format!(
                "failed to load plugins from directory {path}: {err}"
            ))
        })?;
        *guard = Some(manager);
        Ok(())
    }

    /// Registers a Python plugin with the plugin manager.
    ///
    /// This method serves as a public interface for registering Python plugins,
    /// accepting a bound Python object and delegating to the internal registration
    /// logic. The plugin must expose the required plugin identity attributes
    /// (`name` and `group`) and belong to a supported plugin group
    /// (currently "ProcessorPlugin", "ConnectionPlugin", "InventoryPlugin",
    /// "RunnerPlugin",
    /// or "TransformFunctionPlugin").
    ///
    /// # Parameters
    ///
    /// * `plugin` - A bound reference to a Python object implementing the plugin
    ///   interface. The object will be unbound and stored internally for later use.
    ///   The plugin must expose non-empty string `name` and `group` attributes.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the plugin was successfully registered, or a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    /// - The plugin is missing required `name` or `group` attributes
    /// - The plugin's `name` or `group` attributes are callable
    /// - The plugin's `name` or `group` is an empty string
    /// - The plugin's group is not a supported type ("ProcessorPlugin",
    ///   "ConnectionPlugin", "InventoryPlugin", "RunnerPlugin", or
    ///   "TransformFunctionPlugin")
    ///
    /// # Errors
    ///
    /// This function will return an error if the plugin does not conform to the
    /// expected plugin interface or if the plugin manager is in an invalid state.
    fn register_plugin(&self, plugin: Bound<'_, PyAny>) -> PyResult<()> {
        self.register_python_plugin(plugin.unbind())
    }

    /// Loads Python plugins from a `pyproject.toml` file.
    ///
    /// This method reads plugin definitions from the `[tool.genja.plugins]` section
    /// of a `pyproject.toml` file and registers them with the plugin manager. It
    /// supports "processor", "connection", "inventory", "runner", and
    /// "transform" plugin
    /// types. Each plugin entry
    /// must specify an import path in the format `module:attribute`, and the plugin's
    /// declared name (from its `name` property) must match the key used in the manifest.
    ///
    /// The expected structure in `pyproject.toml` is:
    /// ```toml
    /// [tool.genja.plugins.processor]
    /// my_processor = "my_module:MyProcessorClass"
    ///
    /// [tool.genja.plugins.connection]
    /// my_connection = "my_module:MyConnectionClass"
    ///
    /// [tool.genja.plugins.inventory]
    /// my_inventory = "my_module:MyInventoryClass"
    ///
    /// [tool.genja.plugins.runner]
    /// my_runner = "my_module:MyRunnerClass"
    ///
    /// [tool.genja.plugins.transform]
    /// my_transform = "my_module:MyTransformClass"
    /// ```
    ///
    /// # Parameters
    ///
    /// * `path` - An optional string slice representing the filesystem path to the
    ///   `pyproject.toml` file. If `None`, defaults to `"pyproject.toml"` in the
    ///   current directory. The path can be absolute or relative.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all plugins were successfully loaded and registered, or
    /// a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    /// - The `pyproject.toml` file cannot be read or parsed
    /// - Any plugin import path is invalid or the plugin cannot be imported
    /// - A plugin's declared `name` does not match its manifest key
    /// - Any plugin registration fails (e.g., missing required methods, unsupported group)
    ///
    /// # Errors
    ///
    /// This function will return an error if the manifest file is invalid, if any
    /// plugin import fails, if there is a name mismatch between the manifest key
    /// and the plugin's declared name, or if plugin registration fails for any reason.
    #[pyo3(signature = (path=None))]
    fn load_python_plugins_from_pyproject(&self, path: Option<&str>) -> PyResult<()> {
        let manifest_path = path
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("pyproject.toml"));
        let manifest = fs::read_to_string(&manifest_path).map_err(|err| {
            PyValueError::new_err(format!(
                "failed to read pyproject file {}: {err}",
                manifest_path.display()
            ))
        })?;
        let value: toml::Value = toml::from_str(&manifest).map_err(|err| {
            PyValueError::new_err(format!(
                "failed to parse pyproject file {}: {err}",
                manifest_path.display()
            ))
        })?;

        for section_name in [
            "processor",
            "connection",
            "inventory",
            "runner",
            "transform",
        ] {
            let Some(entries) = value
                .get("tool")
                .and_then(|tool| tool.get("genja"))
                .and_then(|genja| genja.get("plugins"))
                .and_then(|plugins| plugins.get(section_name))
                .and_then(toml::Value::as_table)
            else {
                continue;
            };

            for (name, import_path) in entries {
                let import_path = import_path.as_str().ok_or_else(|| {
                    PyValueError::new_err(format!(
                        "{section_name} plugin entry '{name}' in {} must be a string import path",
                        manifest_path.display()
                    ))
                })?;
                let plugin = Python::attach(|py| import_python_plugin(py, import_path))?;
                let declared_name = Python::attach(|py| {
                    extract_plugin_identity_value(
                        plugin.bind(py),
                        "name",
                        &format!("{section_name} plugin name must not be empty"),
                        "plugin",
                    )
                })?;
                if declared_name != *name {
                    return Err(PyValueError::new_err(format!(
                        "{section_name} plugin name mismatch in {}: manifest key '{name}' does not match plugin.name value '{declared_name}'",
                        manifest_path.display()
                    )));
                }
                self.register_python_plugin(plugin)?;
            }
        }

        Ok(())
    }

    /// Removes a plugin from the plugin manager by name.
    ///
    /// This method deregisters a previously registered plugin, removing it from the
    /// plugin manager's internal registry. After deregistration, the plugin will no
    /// longer be available for use. The method returns the deregistered plugin's
    /// declared name if it was found, or `None` if no plugin with the given
    /// name was registered.
    ///
    /// # Parameters
    ///
    /// * `name` - A string slice representing the unique name of the plugin to
    ///   deregister. This should match the plugin's `name` property.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(String))` containing the deregistered plugin name
    /// plugin if a plugin with the given name was found and removed, or `Ok(None)`
    /// if no plugin with that name was registered. Returns a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    ///
    /// # Errors
    ///
    /// This function will return an error if the plugin manager has been consumed
    /// or if the internal lock is poisoned.
    fn deregister_plugin(&self, name: &str) -> PyResult<Option<String>> {
        let mut guard = self.lock_inner()?;
        let manager = guard
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))?;
        Ok(manager.deregister_plugin(name))
    }

    /// Retrieves the names of all registered plugins.
    ///
    /// This method returns a list of the unique names of all plugins currently
    /// registered with the plugin manager, including both built-in plugins and
    /// any plugins that have been registered via `register_plugin`,
    /// `load_rust_plugins_from_directory`, or `load_python_plugins_from_pyproject`.
    /// The names correspond to each plugin's `name` property.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<String>)` containing the names of all registered plugins,
    /// or a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    ///
    /// # Errors
    ///
    /// This function will return an error if the plugin manager has been consumed
    /// or if the internal lock is poisoned.
    fn plugin_names(&self) -> PyResult<Vec<String>> {
        let guard = self.lock_inner()?;
        let manager = guard
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))?;
        Ok(manager
            .get_all_plugin_names()
            .into_iter()
            .map(|name| name.to_string())
            .collect())
    }

    /// Retrieves the names and groups of all registered plugins.
    ///
    /// This method returns a list of tuples containing the unique name and group
    /// identifier for each plugin currently registered with the plugin manager.
    /// The information includes both built-in plugins and any plugins that have
    /// been registered via `register_plugin`, `load_rust_plugins_from_directory`,
    /// or `load_python_plugins_from_pyproject`. Each tuple contains the plugin's
    /// name (from its `name` property) and its group (from its `group` property).
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<(String, String)>)` containing tuples of (name, group) for
    /// all registered plugins, or a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    ///
    /// # Errors
    ///
    /// This function will return an error if the plugin manager has been consumed
    /// or if the internal lock is poisoned.
    fn plugin_names_and_groups(&self) -> PyResult<Vec<(String, String)>> {
        let guard = self.lock_inner()?;
        let manager = guard
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))?;
        Ok(manager.get_all_plugin_names_and_groups())
    }

    /// Returns a string representation of the plugin manager for Python's `repr()`.
    ///
    /// This method provides a human-readable representation of the plugin manager's
    /// state, including the number of registered plugins and whether the manager
    /// has been consumed. The format is `PluginManager(plugin_count=N, consumed=bool)`.
    /// If the internal lock cannot be acquired, returns `PluginManager(<unavailable>)`.
    ///
    /// # Returns
    ///
    /// Returns a `String` containing:
    /// - The number of registered plugins and consumption status if the lock is available
    /// - `"PluginManager(<unavailable>)"` if the lock cannot be acquired
    fn __repr__(&self) -> String {
        match self.lock_inner() {
            Ok(guard) => {
                let plugin_count = guard
                    .as_ref()
                    .map(|m| m.get_all_plugin_names().len())
                    .unwrap_or(0);
                let consumed = guard.is_none();
                format!("PluginManager(plugin_count={plugin_count}, consumed={consumed})")
            }
            Err(_) => "PluginManager(<unavailable>)".to_string(),
        }
    }
}

impl PyPluginManager {
    /// Consumes and returns the inner `PluginManager`, leaving `None` in its place.
    ///
    /// This method provides one-time consumption semantics for the plugin manager,
    /// allowing it to be moved out of the `PyPluginManager` wrapper. After this
    /// method is called, the plugin manager is no longer available and subsequent
    /// operations will fail with an error indicating the manager has been consumed.
    ///
    /// # Returns
    ///
    /// Returns `Ok(PluginManager)` containing the inner plugin manager if it has
    /// not been previously consumed, or a `PyErr` if:
    /// - The plugin manager has already been consumed (taken)
    /// - The plugin manager lock is poisoned
    ///
    /// # Errors
    ///
    /// This function will return an error if the plugin manager has already been
    /// consumed or if the internal mutex lock is poisoned.
    pub(crate) fn take_inner(&self) -> PyResult<PluginManager> {
        let mut guard = self.lock_inner()?;
        guard
            .take()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))
    }

    /// Acquires a mutex guard for the inner `Option<PluginManager>`.
    ///
    /// This method provides thread-safe access to the inner plugin manager by
    /// acquiring the mutex lock. The returned guard allows safe concurrent access
    /// to the plugin manager from multiple threads.
    ///
    /// # Returns
    ///
    /// Returns `Ok(MutexGuard<'_, Option<PluginManager>>)` containing a guard that
    /// provides access to the inner plugin manager, or a `PyErr` if the mutex lock
    /// is poisoned (indicating that a thread panicked while holding the lock).
    ///
    /// # Errors
    ///
    /// This function will return an error if the internal mutex lock is poisoned,
    /// which occurs when a thread panics while holding the lock.
    fn lock_inner(&self) -> PyResult<std::sync::MutexGuard<'_, Option<PluginManager>>> {
        self.inner
            .lock()
            .map_err(|_| PyValueError::new_err("plugin manager lock is poisoned"))
    }

    /// Registers a Python plugin with the plugin manager.
    ///
    /// This internal method handles the registration of Python plugins by acquiring
    /// the plugin manager lock, verifying the manager has not been consumed, and
    /// delegating to the registration logic. The plugin must implement the required
    /// plugin identity attributes (`name` and `group`) and belong to a supported
    /// plugin group.
    ///
    /// # Parameters
    ///
    /// * `plugin` - A Python object implementing the plugin interface. The object
    ///   must expose non-empty string `name` and `group` attributes. The plugin's
    ///   group must be one of
    ///   "ProcessorPlugin", "ConnectionPlugin", "InventoryPlugin",
    ///   "RunnerPlugin", or "TransformFunctionPlugin".
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the plugin was successfully registered, or a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    /// - The plugin is missing required `name` or `group` attributes
    /// - The plugin's `name` or `group` attributes are callable
    /// - The plugin's `name` or `group` is an empty string
    /// - The plugin's group is not a supported type
    ///
    /// # Errors
    ///
    /// This function will return an error if the plugin manager is in an invalid
    /// state or if the plugin does not conform to the expected plugin interface.
    fn register_python_plugin(&self, plugin: Py<PyAny>) -> PyResult<()> {
        let mut guard = self.lock_inner()?;
        let manager = guard
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))?;
        register_python_plugin_on_manager(manager, plugin)
    }
}

/// Registers a Python plugin directly with a mutable `PluginManager` reference.
///
/// This function extracts the plugin's identity (name and group) from its
/// `name` and `group` properties, then wraps the plugin in the appropriate Rust
/// adapter type based on its group. The wrapped plugin is then registered with
/// the provided plugin manager. This function is used internally by the
/// `PyPluginManager` wrapper and can also be used directly when a mutable
/// reference to a `PluginManager` is available.
///
/// # Parameters
///
/// * `manager` - A mutable reference to the `PluginManager` where the plugin
///   will be registered. The manager maintains the registry of all plugins and
///   handles plugin lifecycle operations.
/// * `plugin` - A Python object implementing the plugin interface. The object
///   must expose non-empty string `name` and `group` attributes. The plugin's
///   group must be one of "ProcessorPlugin",
///   "ConnectionPlugin", "InventoryPlugin", "RunnerPlugin", or
///   "TransformFunctionPlugin". The plugin is wrapped in an `Arc` for shared
///   ownership across the plugin system.
///
/// # Returns
///
/// Returns `Ok(())` if the plugin was successfully registered, or a `PyErr` if:
/// - The plugin is missing required `name` or `group` attributes
/// - The plugin's `name` or `group` attributes are callable
/// - The plugin's `name` or `group` is an empty string
/// - The plugin's group is not "ProcessorPlugin", "ConnectionPlugin",
///   "InventoryPlugin", "RunnerPlugin", or "TransformFunctionPlugin"
/// - Any Python error occurs during identity extraction
///
/// # Errors
///
/// This function will return an error if the plugin does not conform to the
/// expected plugin interface or if its group type is not supported.
pub(crate) fn register_python_plugin_on_manager(
    manager: &mut PluginManager,
    plugin: Py<PyAny>,
) -> PyResult<()> {
    let (declared_name, declared_group) = Python::attach(|py| {
        let plugin_ref = plugin.bind(py);
        let declared_name = extract_plugin_identity_value(
            plugin_ref,
            "name",
            "plugin name must not be empty",
            "plugin",
        )?;
        let declared_group = extract_plugin_identity_value(
            plugin_ref,
            "group",
            "plugin group must not be empty",
            "plugin",
        )?;
        Ok::<_, PyErr>((declared_name, declared_group))
    })?;

    match declared_group.as_str() {
        "ConnectionPlugin" => {
            manager.register_plugin(Plugins::Connection(Box::new(PyConnectionPlugin {
                name: declared_name,
                group: declared_group,
                plugin: Arc::new(plugin),
            })));
        }
        "InventoryPlugin" => {
            manager.register_plugin(Plugins::Inventory(Box::new(PyInventoryPlugin {
                name: declared_name,
                group: declared_group,
                plugin: Arc::new(plugin),
            })));
        }
        "RunnerPlugin" => {
            manager.register_plugin(Plugins::Runner(Box::new(PyRunnerPlugin {
                name: declared_name,
                group: declared_group,
                plugin: Arc::new(plugin),
            })));
        }
        "TransformFunctionPlugin" => {
            manager.register_plugin(Plugins::TransformFunction(Box::new(
                PyTransformFunctionPlugin {
                    name: declared_name,
                    group: declared_group,
                    plugin: Arc::new(plugin),
                },
            )));
        }
        "ProcessorPlugin" => {
            manager.register_plugin(Plugins::Processor(Box::new(PyProcessorPlugin {
                name: declared_name,
                group: declared_group,
                processor: Arc::new(plugin),
            })));
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "unsupported python plugin group '{other}'; only 'ProcessorPlugin', 'ConnectionPlugin', 'InventoryPlugin', 'RunnerPlugin', and 'TransformFunctionPlugin' are currently supported"
            )));
        }
    }

    Ok(())
}

/// A Rust adapter for Python connection plugins that serves as a factory.
///
/// This struct wraps a Python connection plugin object and implements the `Plugin`
/// and `PluginConnection` traits to integrate Python-based connection plugins into
/// the Rust plugin system. It acts as a factory that creates actual connection
/// instances via the `create` method. The factory itself cannot be opened or used
/// as a connection directly; it only produces `PyConnectionInstance` objects that
/// handle the actual connection lifecycle.
///
/// # Fields
///
/// * `name` - The unique identifier for this connection plugin, matching the Python
///   plugin's `name` property.
/// * `group` - The group identifier for this plugin, matching the Python plugin's
///   `group` property. For connection plugins, this is typically
///   "ConnectionPlugin".
/// * `plugin` - An `Arc`-wrapped Python object implementing the connection plugin
///   interface. The `Arc` allows the plugin to be shared across multiple connection
///   instances created by this factory.
struct PyConnectionPlugin {
    name: String,
    group: String,
    plugin: Arc<Py<PyAny>>,
}

/// A Python-exposed snapshot of registered plugins for safe cross-language access.
///
/// This struct provides a read-only, point-in-time view of the plugins currently
/// registered with the plugin manager. It serves as a safe boundary between Rust's
/// `PluginManager` and Python code, particularly for inventory plugins that need
/// to query available plugins during their `load()` operation.
///
/// # Purpose
///
/// The registry exists to solve several critical design challenges:
///
/// 1. **Safe Cross-Language Access**: Python code cannot safely hold references to
///    the Rust `PluginManager` due to lifetime and ownership constraints. This
///    snapshot provides owned data that Python can safely access.
///
/// 2. **Immutability Guarantee**: By providing only read access to plugin names
///    and groups, the registry prevents Python code from modifying the plugin
///    system state, avoiding potential race conditions and invariant violations.
///
/// 3. **Prevents Circular Dependencies**: Inventory plugins receive this snapshot
///    instead of the full manager, preventing deadlocks that could occur if they
///    tried to register additional plugins during loading.
///
/// 4. **Point-in-Time Consistency**: The snapshot captures the plugin state at a
///    specific moment, ensuring that inventory plugins see a consistent view even
///    if the plugin manager is modified concurrently.
///
/// # Usage
///
/// This type is primarily used internally when calling Python inventory plugins:
///
/// ```rust,ignore
/// impl PluginInventory for PyInventoryPlugin {
///     fn load(&self, settings: &Settings, plugins: &PluginManager) -> Result<...> {
///         let registry = PyLoadedPluginRegistry {
///             names: plugins.get_all_plugin_names()
///                 .into_iter()
///                 .map(|s| s.to_string())
///                 .collect(),
///             names_and_groups: plugins.get_all_plugin_names_and_groups(),
///         };
///
///         // Pass snapshot to Python instead of full manager
///         plugin.call_method1("load", (settings, registry))?;
///     }
/// }
/// ```
///
/// From Python, inventory plugins receive this as a parameter:
///
/// ```python
/// class MyInventoryPlugin:
///     def load(self, settings, plugin_registry):
///         # Query available connection plugins
///         connection_plugins = [
///             name for name, group in plugin_registry.plugin_names_and_groups()
///             if group == "Connection"
///         ]
///         # Use this information to validate inventory configuration
///         # ...
/// ```
///
/// # Fields
///
/// * `names` - A vector containing the unique names of all registered plugins.
///   Each name corresponds to a plugin's `name` property.
/// * `names_and_groups` - A vector of tuples containing both the name and group
///   identifier for each registered plugin. The group identifies the plugin type
///   (e.g., "Processor", "Connection", "Inventory").
#[pyclass(skip_from_py_object)]
#[derive(Clone)]
struct PyLoadedPluginRegistry {
    names: Vec<String>,
    names_and_groups: Vec<(String, String)>,
}

#[pymethods]
impl PyLoadedPluginRegistry {
    /// Retrieves the names of all registered plugins in the registry.
    ///
    /// This method returns a cloned list of the unique names of all plugins that
    /// were registered at the time this registry snapshot was created. The names
    /// correspond to each plugin's `name` property.
    ///
    /// # Returns
    ///
    /// Returns a `Vec<String>` containing the names of all registered plugins.
    /// The vector is a clone of the internal registry state, so modifications
    /// to the returned vector will not affect the registry.
    fn plugin_names(&self) -> Vec<String> {
        self.names.clone()
    }

    /// Retrieves the names and groups of all registered plugins in the registry.
    ///
    /// This method returns a cloned list of tuples containing both the unique name
    /// and group identifier for each plugin that was registered at the time this
    /// registry snapshot was created. Each tuple contains the plugin's name (from
    /// its `name` property) and its group (from its `group` property), which
    /// identifies the plugin type (e.g., "Processor", "Connection", "Inventory").
    ///
    /// # Returns
    ///
    /// Returns a `Vec<(String, String)>` containing tuples of (name, group) for
    /// all registered plugins. The vector is a clone of the internal registry
    /// state, so modifications to the returned vector will not affect the registry.
    fn plugin_names_and_groups(&self) -> Vec<(String, String)> {
        self.names_and_groups.clone()
    }

    /// Returns a string representation of the plugin registry for Python's `repr()`.
    ///
    /// This method provides a human-readable representation of the registry's state,
    /// showing the total number of registered plugins. The format is
    /// `LoadedPluginRegistry(plugin_count=N)`, where N is the number of plugins
    /// in the registry snapshot.
    ///
    /// # Returns
    ///
    /// Returns a `String` containing the registry representation with the plugin
    /// count, formatted as `LoadedPluginRegistry(plugin_count=N)`.
    fn __repr__(&self) -> String {
        format!(
            "LoadedPluginRegistry(plugin_count={})",
            self.names_and_groups.len()
        )
    }
}

/// A Rust adapter for Python inventory plugins.
///
/// This struct wraps a Python inventory plugin object and implements the `Plugin`
/// and `PluginInventory` traits to integrate Python-based inventory plugins into
/// the Rust plugin system. It enables Python code to provide inventory data by
/// implementing a `load()` method that returns host and connection information.
///
/// The adapter handles the conversion between Python and Rust data types, manages
/// async/sync detection for Python methods, and provides a safe snapshot of the
/// plugin registry to prevent circular dependencies during inventory loading.
///
/// # Fields
///
/// * `name` - The unique identifier for this inventory plugin, matching the Python
///   plugin's `name` property.
/// * `group` - The group identifier for this plugin, matching the Python plugin's
///   `group` property. For inventory plugins, this is typically
///   "InventoryPlugin".
/// * `plugin` - An `Arc`-wrapped Python object implementing the inventory plugin
///   interface. The `Arc` allows the plugin to be shared safely across threads and
///   ensures the Python object remains valid for the plugin's lifetime.
struct PyInventoryPlugin {
    name: String,
    group: String,
    plugin: Arc<Py<PyAny>>,
}

impl Plugin for PyInventoryPlugin {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn group(&self) -> String {
        self.group.clone()
    }
}

impl PluginInventory for PyInventoryPlugin {
    /// Loads inventory data by calling the Python plugin's `load()` method.
    ///
    /// This method bridges the Rust inventory loading interface to Python by:
    /// 1. Converting Rust `Settings` and `PluginManager` to Python-compatible types
    /// 2. Calling the Python plugin's `load()` method with these converted parameters
    /// 3. Handling both synchronous and asynchronous Python implementations
    /// 4. Converting the Python inventory data back to Rust's `Inventory` type
    ///
    /// The method provides a safe snapshot of the plugin registry to prevent circular
    /// dependencies and ensures the Python plugin cannot modify the plugin manager state.
    ///
    /// # Parameters
    ///
    /// * `settings` - A reference to the `Settings` object containing configuration
    ///   data for the inventory plugin. This is wrapped in a `PySettings` object and
    ///   passed to the Python plugin's `load()` method.
    /// * `plugins` - A reference to the `PluginManager` containing all registered
    ///   plugins. A read-only snapshot (`PyLoadedPluginRegistry`) is created from this
    ///   manager and passed to the Python plugin, allowing it to query available plugins
    ///   without modifying the plugin system state.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Inventory)` containing the loaded inventory data with hosts and
    /// connections if the Python plugin successfully loads and returns valid inventory
    /// data. Returns `Err(InventoryLoadError)` if:
    /// - The Python plugin's `load()` method raises an exception
    /// - The Python plugin returns invalid inventory data that cannot be converted
    /// - Any Python object conversion or method call fails
    /// - The async resolution of the Python method fails (if the plugin uses async)
    ///
    /// # Errors
    ///
    /// This function will return an error if the Python plugin's `load()` method fails,
    /// returns invalid data, or if any cross-language conversion fails during the
    /// inventory loading process.
    fn load(
        &self,
        settings: &Settings,
        plugins: &PluginManager,
    ) -> Result<Inventory, InventoryLoadError> {
        Python::attach(|py| {
            let plugin = self.plugin.bind(py);
            let settings_payload = Py::new(
                py,
                PySettings {
                    inner: settings.clone(),
                },
            )
            .map_err(|err| InventoryLoadError::from(err.to_string()))?;
            let plugin_registry = Py::new(
                py,
                PyLoadedPluginRegistry {
                    names: plugins
                        .get_all_plugin_names()
                        .into_iter()
                        .map(|name| name.to_string())
                        .collect(),
                    names_and_groups: plugins.get_all_plugin_names_and_groups(),
                },
            )
            .map_err(|err| InventoryLoadError::from(err.to_string()))?;
            let result = plugin
                .call_method1(
                    "load",
                    (settings_payload.bind(py), plugin_registry.bind(py)),
                )
                .map_err(|err| InventoryLoadError::from(err.to_string()))?;
            let resolved = resolve_python_maybe_awaitable(py, result)
                .map_err(|err| InventoryLoadError::from(err.to_string()))?;
            python_inventory_to_rust_inventory(resolved.bind(py).clone())
                .map_err(|err| InventoryLoadError::from(err.to_string()))
        })
    }
}

struct PyTransformFunctionPlugin {
    name: String,
    group: String,
    plugin: Arc<Py<PyAny>>,
}

impl Plugin for PyTransformFunctionPlugin {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn group(&self) -> String {
        self.group.clone()
    }
}

impl PluginTransformFunction for PyTransformFunctionPlugin {
    fn transform_function(&self) -> TransformFunction {
        TransformFunction::new_full(PyTransformBridge {
            plugin: Arc::clone(&self.plugin),
        })
    }
}

struct PyTransformBridge {
    plugin: Arc<Py<PyAny>>,
}

impl PyTransformBridge {
    fn call_transform<T>(
        &self,
        method_name: &str,
        value: &T,
        options: Option<&TransformFunctionOptions>,
    ) -> Result<Option<T>, String>
    where
        T: Serialize + DeserializeOwned,
    {
        Python::attach(|py| {
            let plugin = self.plugin.bind(py);
            if !plugin.hasattr(method_name).map_err(|err| err.to_string())? {
                return Ok(None);
            }

            let value_payload =
                serde_to_python_payload(py, value).map_err(|err| err.to_string())?;
            let options_payload =
                transform_options_to_python_payload(py, options).map_err(|err| err.to_string())?;
            let result = plugin
                .call_method1(
                    method_name,
                    (value_payload.bind(py), options_payload.bind(py)),
                )
                .map_err(|err| err.to_string())?;
            let resolved =
                resolve_python_maybe_awaitable(py, result).map_err(|err| err.to_string())?;
            python_payload_to_rust_value(resolved.bind(py), "invalid transform payload")
                .map(Some)
                .map_err(|err| err.to_string())
        })
    }
}

impl Transform for PyTransformBridge {
    fn transform_host(&self, host: &Host, options: Option<&TransformFunctionOptions>) -> Host {
        match self.call_transform("transform_host", host, options) {
            Ok(Some(host)) => host,
            Ok(None) => host.clone(),
            Err(err) => panic!("python transform plugin transform_host failed: {err}"),
        }
    }

    fn transform_group(&self, group: &Group, options: Option<&TransformFunctionOptions>) -> Group {
        match self.call_transform("transform_group", group, options) {
            Ok(Some(group)) => group,
            Ok(None) => group.clone(),
            Err(err) => panic!("python transform plugin transform_group failed: {err}"),
        }
    }

    fn transform_defaults(
        &self,
        defaults: &Defaults,
        options: Option<&TransformFunctionOptions>,
    ) -> Defaults {
        match self.call_transform("transform_defaults", defaults, options) {
            Ok(Some(defaults)) => defaults,
            Ok(None) => defaults.clone(),
            Err(err) => panic!("python transform plugin transform_defaults failed: {err}"),
        }
    }
}

struct PyRunnerPlugin {
    name: String,
    group: String,
    plugin: Arc<Py<PyAny>>,
}

impl Plugin for PyRunnerPlugin {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn group(&self) -> String {
        self.group.clone()
    }
}

#[async_trait]
impl PluginRunner for PyRunnerPlugin {
    async fn run_task(
        &self,
        task: &TaskDefinition,
        hosts: &genja_core::inventory::Hosts,
        connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
        runner_config: &RunnerConfig,
        max_depth: usize,
    ) -> Result<TaskResults, genja_core::GenjaError> {
        let result = Python::attach(|py| {
            let plugin = self.plugin.bind(py);
            let task_payload = Py::new(py, PyTaskDefinition::from_runtime_definition(task.clone()))
                .map_err(python_processor_error)?;
            let hosts_payload = hosts_to_py_dict(py, hosts).map_err(python_processor_error)?;
            let resolver_payload = match connection_resolver {
                Some(ref resolver) => Py::new(
                    py,
                    PyTaskConnectionResolver {
                        inner: Some(Arc::clone(resolver)),
                    },
                )
                .map(|resolver| resolver.into_any())
                .map_err(python_processor_error)?,
                None => py.None(),
            };
            let runner_payload = Py::new(
                py,
                PyRunnerConfig {
                    inner: runner_config.clone(),
                },
            )
            .map_err(python_processor_error)?;
            plugin
                .call_method1(
                    "run_task",
                    (
                        task_payload.bind(py),
                        hosts_payload.bind(py),
                        resolver_payload.bind(py),
                        runner_payload.bind(py),
                        max_depth,
                    ),
                )
                .map(Bound::unbind)
                .map_err(python_processor_error)
        })?;
        let resolved = resolve_python_maybe_awaitable_async(result)
            .await
            .map_err(python_processor_error)?;
        Python::attach(|py| {
            python_result_to_task_results(resolved.bind(py).clone()).map_err(python_processor_error)
        })
    }

    async fn run_tasks(
        &self,
        tasks: &Tasks,
        hosts: &genja_core::inventory::Hosts,
        connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
        runner_config: &RunnerConfig,
        max_depth: usize,
    ) -> Result<Vec<TaskResults>, genja_core::GenjaError> {
        let has_run_tasks = Python::attach(|py| {
            self.plugin
                .bind(py)
                .hasattr("run_tasks")
                .map_err(python_processor_error)
        })?;
        if !has_run_tasks {
            let mut results = Vec::with_capacity(tasks.len());
            for task in tasks.iter() {
                results.push(
                    self.run_task(
                        task,
                        hosts,
                        connection_resolver.clone(),
                        runner_config,
                        max_depth,
                    )
                    .await?,
                );
            }
            return Ok(results);
        }

        let result = Python::attach(|py| {
            let plugin = self.plugin.bind(py);
            let task_payloads = tasks
                .iter()
                .map(|task| Py::new(py, PyTaskDefinition::from_runtime_definition(task.clone())))
                .collect::<PyResult<Vec<_>>>()
                .map_err(python_processor_error)?;
            let hosts_payload = hosts_to_py_dict(py, hosts).map_err(python_processor_error)?;
            let resolver_payload = match connection_resolver {
                Some(ref resolver) => Py::new(
                    py,
                    PyTaskConnectionResolver {
                        inner: Some(Arc::clone(resolver)),
                    },
                )
                .map(|resolver| resolver.into_any())
                .map_err(python_processor_error)?,
                None => py.None(),
            };
            let runner_payload = Py::new(
                py,
                PyRunnerConfig {
                    inner: runner_config.clone(),
                },
            )
            .map_err(python_processor_error)?;
            plugin
                .call_method1(
                    "run_tasks",
                    (
                        task_payloads,
                        hosts_payload.bind(py),
                        resolver_payload.bind(py),
                        runner_payload.bind(py),
                        max_depth,
                    ),
                )
                .map(Bound::unbind)
                .map_err(python_processor_error)
        })?;
        let resolved = resolve_python_maybe_awaitable_async(result)
            .await
            .map_err(python_processor_error)?;
        Python::attach(|py| {
            let sequence = resolved
                .bind(py)
                .try_iter()
                .map_err(python_processor_error)?;
            let mut results = Vec::new();
            for item in sequence {
                let item = item.map_err(python_processor_error)?;
                results.push(python_result_to_task_results(item).map_err(python_processor_error)?);
            }
            Ok(results)
        })
    }
}

impl Plugin for PyConnectionPlugin {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn group(&self) -> String {
        self.group.clone()
    }
}

#[async_trait]
impl PluginConnection for PyConnectionPlugin {
    /// Creates a new connection instance from this factory.
    ///
    /// This method instantiates a new `PyConnectionInstance` by calling the Python
    /// plugin's `create()` method with the provided connection key. The resulting
    /// instance is the actual connection object that can be opened, used, and closed.
    /// This factory pattern allows multiple independent connections to be created
    /// from the same plugin definition.
    ///
    /// # Parameters
    ///
    /// * `key` - A reference to the `ConnectionKey` identifying the target connection.
    ///   This key contains the hostname and plugin name and is passed to the Python
    ///   plugin's `create()` method to initialize the connection instance.
    ///
    /// # Returns
    ///
    /// Returns a boxed `PyConnectionInstance` that wraps the Python connection object
    /// created by the plugin's `create()` method. The instance is ready to be opened
    /// and used for executing commands.
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        Box::new(PyConnectionInstance::from_factory(
            Arc::clone(&self.plugin),
            self.name.clone(),
            self.group.clone(),
            key.clone(),
        ))
    }

    /// Attempts to open the factory instance, which always fails.
    ///
    /// This method is not supported for factory instances and always returns an error.
    /// Connection plugin factories cannot be opened directly; only the instances
    /// created by the `create()` method can be opened. This design enforces the
    /// factory pattern and prevents misuse of the factory as a connection.
    ///
    /// # Parameters
    ///
    /// * `_params` - Connection parameters (unused, as this operation is not supported).
    ///
    /// # Returns
    ///
    /// Always returns `Err` with a message indicating that factory instances cannot
    /// be opened directly.
    async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
        Err("connection plugin factory instances cannot be opened directly".to_string())
    }

    /// Closes the factory instance and returns a minimal connection key.
    ///
    /// This method is called when the factory is being shut down. Since the factory
    /// itself is not an active connection, this operation simply returns a connection
    /// key with an empty hostname and the plugin's name. This satisfies the trait
    /// requirement while indicating that no actual connection was closed.
    ///
    /// # Returns
    ///
    /// Returns a `ConnectionKey` with an empty hostname and the plugin's name,
    /// indicating that this is a factory instance rather than an active connection.
    fn close(&mut self) -> ConnectionKey {
        ConnectionKey::new("", self.name.clone())
    }

    /// Checks if the factory instance is alive.
    ///
    /// This method always returns `false` because the factory itself is not an active
    /// connection. Only the instances created by the `create()` method can be alive.
    /// This design ensures that liveness checks are only meaningful for actual
    /// connection instances, not for the factory that creates them.
    ///
    /// # Returns
    ///
    /// Always returns `false`, indicating that the factory is not an active connection.
    fn is_alive(&self) -> bool {
        false
    }
}

/// A Rust adapter for Python connection instances created by connection plugin factories.
///
/// This struct represents an actual connection instance created by a Python connection
/// plugin factory. Unlike `PyConnectionPlugin` which acts as a factory, this struct
/// wraps an individual connection that can be opened, used to execute commands, and
/// closed. It implements the `Plugin` and `PluginConnection` traits to integrate
/// Python-based connection instances into the Rust plugin system.
///
/// The instance is created by calling the factory's `create()` method and stores any
/// errors that occur during creation for later reporting. This design allows connection
/// creation failures to be deferred until the connection is actually used, providing
/// better error context.
///
/// # Fields
///
/// * `name` - The unique identifier for the connection plugin, inherited from the
///   factory that created this instance.
/// * `group` - The group identifier for the plugin, inherited from the factory.
///   For connection plugins, this is typically "ConnectionPlugin".
/// * `factory_plugin` - An `Arc`-wrapped reference to the Python factory plugin that
///   created this instance. This reference is maintained to allow creating additional
///   instances from the same factory.
/// * `key` - The `ConnectionKey` identifying this specific connection, containing the
///   hostname and plugin name.
/// * `connection` - An optional `Arc`-wrapped Python connection object created by the
///   factory's `create()` method. This is `None` if connection creation failed.
/// * `create_error` - An optional error message captured if the factory's `create()`
///   method failed. This allows deferred error reporting when the connection is used.
struct PyConnectionInstance {
    name: String,
    group: String,
    factory_plugin: Arc<Py<PyAny>>,
    key: ConnectionKey,
    connection: Option<Py<PyAny>>,
    create_error: Option<String>,
}

impl PyConnectionInstance {
    /// Creates a new connection instance by calling the factory plugin's `create()` method.
    ///
    /// This method invokes the Python factory plugin's `create()` method with the provided
    /// connection key to instantiate a new connection object. The method handles both
    /// synchronous and asynchronous Python implementations by detecting and resolving
    /// awaitable return values. If the creation succeeds, the resulting Python connection
    /// object is stored for later use. If creation fails, the error is captured and stored
    /// to be reported when the connection is actually used.
    ///
    /// # Parameters
    ///
    /// * `factory_plugin` - An `Arc`-wrapped reference to the Python factory plugin that
    ///   will create the connection instance. This factory must implement a `create()`
    ///   method that accepts a connection key and returns a connection object.
    /// * `name` - The unique identifier for the connection plugin, used to identify this
    ///   instance in the plugin system.
    /// * `group` - The group identifier for the plugin, typically "ConnectionPlugin" for
    ///   connection plugins.
    /// * `key` - The `ConnectionKey` identifying the target connection, containing the
    ///   hostname and plugin name. This key is passed to the factory's `create()` method
    ///   and stored for later reference.
    ///
    /// # Returns
    ///
    /// Returns a new `PyConnectionInstance` containing either:
    /// - A successfully created Python connection object in the `connection` field with
    ///   `create_error` set to `None`, or
    /// - A `None` connection with the error message stored in `create_error` if the
    ///   factory's `create()` method failed or returned an invalid value.
    fn from_factory(
        factory_plugin: Arc<Py<PyAny>>,
        name: String,
        group: String,
        key: ConnectionKey,
    ) -> Self {
        let created = Python::attach(|py| {
            let plugin = factory_plugin.bind(py);
            let key_payload = build_python_connection_key(py, &key)?;
            let created = plugin.call_method1("create", (key_payload,))?;
            resolve_python_maybe_awaitable(py, created)
        });

        match created {
            Ok(connection) => Self {
                name,
                group,
                factory_plugin,
                key,
                connection: Some(connection),
                create_error: None,
            },
            Err(err) => Self {
                name,
                group,
                factory_plugin,
                key,
                connection: None,
                create_error: Some(err.to_string()),
            },
        }
    }
}

/// Extracts the underlying Python connection object from a runtime connection.
///
/// This function attempts to unwrap a runtime `Connection` trait object to retrieve
/// the original Python connection object that may be wrapped inside. It performs a
/// series of downcasts to traverse the adapter layers, first converting the connection
/// to a `PluginConnectionAdapter`, then extracting the inner `PyConnectionInstance`,
/// and finally returning the wrapped Python object. This is useful when Python code
/// needs direct access to the connection object that was originally created by a
/// Python connection plugin.
///
/// The function uses type downcasting to safely navigate through the adapter pattern
/// used by the plugin system. If any downcast fails (indicating the connection is not
/// a Python-based connection), the function returns `None`.
///
/// # Parameters
///
/// * `connection` - A reference to a trait object implementing the `Connection` trait.
///   This is typically a runtime connection that may or may not be backed by a Python
///   connection plugin. The function will attempt to extract the Python object if the
///   connection was created by a Python plugin.
///
/// # Returns
///
/// Returns `Some(Py<PyAny>)` containing a reference to the Python connection object
/// if the provided connection is backed by a Python plugin and all downcasts succeed.
/// Returns `None` if:
/// - The connection is not a `PluginConnectionAdapter`
/// - The adapter does not contain a `PyConnectionInstance`
/// - The Python connection instance does not have a connection object
/// - Any downcast in the chain fails
pub(crate) fn python_connection_from_runtime_connection(
    connection: &dyn Connection,
) -> Option<Py<PyAny>> {
    let adapter = (connection as &dyn std::any::Any).downcast_ref::<PluginConnectionAdapter>()?;
    let py_connection = (adapter.inner_plugin_connection() as &dyn std::any::Any)
        .downcast_ref::<PyConnectionInstance>()?;
    Python::attach(|py| {
        py_connection
            .connection
            .as_ref()
            .map(|connection| connection.clone_ref(py))
    })
}

/// Resolves a Python value that may be awaitable (async) or synchronous.
///
/// This function inspects a Python value to determine if it is an awaitable object
/// (such as a coroutine or async function result). If the value is awaitable, it
/// uses Python's `asyncio.run()` to execute and resolve it synchronously. If the
/// value is not awaitable, it is returned as-is. This allows Rust code to handle
/// both synchronous and asynchronous Python plugin methods uniformly without
/// requiring the caller to know whether the Python implementation is async.
///
/// # Parameters
///
/// * `py` - A Python GIL token that provides access to the Python interpreter.
///   This token ensures that the Python interpreter is available and that the
///   operation is performed safely within the GIL context.
/// * `value` - A bound Python object that may or may not be awaitable. This is
///   typically the return value from calling a Python plugin method. If the value
///   is a coroutine or other awaitable object, it will be resolved using
///   `asyncio.run()`. If it is a regular value, it will be returned unchanged.
///
/// # Returns
///
/// Returns `Ok(Py<PyAny>)` containing the resolved Python value. If the input
/// was awaitable, this is the result of executing the coroutine. If the input
/// was not awaitable, this is the original value. Returns `Err(PyErr)` if:
/// - The `inspect` or `asyncio` modules cannot be imported
/// - The `inspect.isawaitable()` call fails
/// - The `asyncio.run()` call fails (for awaitable values)
/// - Any other Python error occurs during inspection or resolution
///
/// # Errors
///
/// This function will return an error if any Python operation fails, including
/// module imports, method calls, or async execution failures.
pub(crate) fn resolve_python_maybe_awaitable<'py>(
    py: Python<'py>,
    value: Bound<'py, PyAny>,
) -> PyResult<Py<PyAny>> {
    let inspect = PyModule::import(py, "inspect")?;
    let is_awaitable: bool = inspect.call_method1("isawaitable", (&value,))?.extract()?;
    if is_awaitable {
        let asyncio = PyModule::import(py, "asyncio")?;
        Ok(asyncio.call_method1("run", (value,))?.unbind())
    } else {
        Ok(value.unbind())
    }
}

pub(crate) async fn resolve_python_maybe_awaitable_async(value: Py<PyAny>) -> PyResult<Py<PyAny>> {
    let is_awaitable = Python::attach(|py| -> PyResult<bool> {
        let inspect = PyModule::import(py, "inspect")?;
        inspect
            .call_method1("isawaitable", (value.bind(py),))?
            .extract()
    })?;
    if !is_awaitable {
        return Ok(value);
    }
    let has_task_locals =
        Python::attach(|py| pyo3_async_runtimes::tokio::get_current_locals(py).map(|_| ())).is_ok();

    if has_task_locals {
        let future = Python::attach(|py| into_future(value.bind(py).clone()))?;
        future.await
    } else {
        Python::attach(|py| {
            let asyncio = PyModule::import(py, "asyncio")?;
            Ok(asyncio.call_method1("run", (value.bind(py),))?.unbind())
        })
    }
}

impl Plugin for PyConnectionInstance {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn group(&self) -> String {
        self.group.clone()
    }
}

#[async_trait]
impl PluginConnection for PyConnectionInstance {
    /// Creates a new connection instance from this connection instance's factory.
    ///
    /// This method creates a new independent connection instance by delegating to the
    /// factory plugin that originally created this instance. The new instance will be
    /// initialized with the same factory plugin, name, and group, but with the provided
    /// connection key. This allows multiple connections to be created from the same
    /// factory, each with its own state and lifecycle.
    ///
    /// # Parameters
    ///
    /// * `key` - A reference to the `ConnectionKey` identifying the target connection
    ///   for the new instance. This key contains the hostname and plugin name and will
    ///   be passed to the factory's `create()` method to initialize the new connection.
    ///
    /// # Returns
    ///
    /// Returns a boxed `PyConnectionInstance` that wraps the Python connection object
    /// created by the factory plugin's `create()` method. The instance is ready to be
    /// opened and used for executing commands. The returned instance is independent of
    /// this instance and has its own connection state.
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        Box::new(Self::from_factory(
            Arc::clone(&self.factory_plugin),
            self.name.clone(),
            self.group.clone(),
            key.clone(),
        ))
    }

    /// Opens the Python connection instance with the provided connection parameters.
    ///
    /// This method establishes the connection by calling the Python connection object's
    /// `open()` method with the resolved connection parameters. It first validates that
    /// the connection instance was successfully created (no creation errors) and that
    /// the connection object exists. The method handles both synchronous and asynchronous
    /// Python implementations by detecting and resolving awaitable return values.
    ///
    /// # Parameters
    ///
    /// * `params` - A reference to the `ResolvedConnectionParams` containing the connection
    ///   details such as hostname, port, username, password, platform, and any additional
    ///   extras. These parameters are converted to a Python-compatible format and passed
    ///   to the Python connection's `open()` method.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the connection was successfully opened. Returns `Err(String)`
    /// containing an error message if:
    /// - The connection instance was not successfully created (has a creation error)
    /// - The connection object is missing from the instance
    /// - The Python connection's `open()` method raises an exception
    /// - The parameter conversion to Python format fails
    /// - The async resolution of the Python method fails (if the plugin uses async)
    ///
    /// # Errors
    ///
    /// This function will return an error if the connection instance is invalid, if the
    /// Python `open()` method fails, or if any cross-language conversion fails during
    /// the opening process.
    async fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String> {
        if let Some(error) = self.create_error.as_ref() {
            return Err(format!(
                "failed to create python connection plugin instance: {error}"
            ));
        }
        let Some(connection) = self.connection.as_ref() else {
            return Err("python connection plugin instance is missing a connection".to_string());
        };

        let result = Python::attach(|py| {
            let connection = connection.bind(py);
            let params_payload = build_python_resolved_connection_params(py, params)
                .map_err(|err| err.to_string())?;
            connection
                .call_method1("open", (params_payload,))
                .map(Bound::unbind)
                .map_err(|err| err.to_string())
        })?;
        resolve_python_maybe_awaitable_async(result)
            .await
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    /// Executes a command on the Python connection instance and returns its output.
    ///
    /// This method delegates command execution to the Python connection object's
    /// `execute_command()` method. It first validates that the connection instance
    /// was successfully created (no creation errors) and that the connection object
    /// exists. The method handles both synchronous and asynchronous Python implementations
    /// by detecting and resolving awaitable return values. The command output is expected
    /// to be a string that can be extracted from the Python return value.
    ///
    /// # Parameters
    ///
    /// * `command` - A string slice containing the command to execute on the connection.
    ///   This command is passed directly to the Python connection's `execute_command()`
    ///   method without modification.
    ///
    /// # Returns
    ///
    /// Returns `Ok(String)` containing the command output if execution succeeds.
    /// Returns `Err(String)` containing an error message if:
    /// - The connection instance was not successfully created (has a creation error)
    /// - The connection object is missing from the instance
    /// - The Python connection's `execute_command()` method raises an exception
    /// - The async resolution of the Python method fails (if the plugin uses async)
    /// - The Python return value cannot be extracted as a string
    ///
    /// # Errors
    ///
    /// This function will return an error if the connection instance is invalid, if the
    /// Python `execute_command()` method fails, or if the return value cannot be converted
    /// to a Rust string.
    async fn execute_command(&mut self, command: &str) -> Result<String, String> {
        if let Some(error) = self.create_error.as_ref() {
            return Err(format!(
                "failed to create python connection plugin instance: {error}"
            ));
        }
        let Some(connection) = self.connection.as_ref() else {
            return Err("python connection plugin instance is missing a connection".to_string());
        };

        let result = Python::attach(|py| {
            let connection = connection.bind(py);
            connection
                .call_method1("execute_command", (command,))
                .map(Bound::unbind)
                .map_err(|err| err.to_string())
        })?;
        let resolved = resolve_python_maybe_awaitable_async(result)
            .await
            .map_err(|err| err.to_string())?;
        Python::attach(|py| {
            resolved
                .bind(py)
                .extract::<String>()
                .map_err(|err| err.to_string())
        })
    }

    /// Closes the Python connection instance and returns its connection key.
    ///
    /// This method terminates the connection by calling the Python connection object's
    /// `close()` method. It handles both synchronous and asynchronous Python implementations
    /// by detecting and resolving awaitable return values. The method attempts to extract
    /// a connection key from the Python `close()` method's return value if one is provided.
    /// If the connection object is missing, if the `close()` method fails, if it returns
    /// `None`, or if the returned value cannot be converted to a connection key, the
    /// method falls back to returning this instance's stored connection key.
    ///
    /// # Returns
    ///
    /// Returns a `ConnectionKey` identifying the closed connection. This is either:
    /// - A connection key extracted from the Python `close()` method's return value if
    ///   the method succeeds and returns a valid connection key object
    /// - The connection key stored in this instance (from when it was created) if the
    ///   connection object is missing, the `close()` method fails, returns `None`, or
    ///   returns a value that cannot be converted to a connection key
    fn close(&mut self) -> ConnectionKey {
        let Some(connection) = self.connection.as_ref() else {
            return self.key.clone();
        };

        Python::attach(|py| {
            let connection = connection.bind(py);
            match connection.call_method0("close") {
                Ok(value) => match resolve_python_maybe_awaitable(py, value) {
                    Ok(value) => {
                        let value = value.bind(py);
                        if value.is_none() {
                            self.key.clone()
                        } else {
                            py_any_to_connection_key(value).unwrap_or_else(|_| self.key.clone())
                        }
                    }
                    Err(_) => self.key.clone(),
                },
                Err(_) => self.key.clone(),
            }
        })
    }

    /// Checks if the Python connection instance is currently alive and operational.
    ///
    /// This method determines the connection's liveness by calling the Python connection
    /// object's `is_alive()` method. It first validates that the connection instance was
    /// successfully created (no creation errors) and that the connection object exists.
    /// The method handles both synchronous and asynchronous Python implementations by
    /// detecting and resolving awaitable return values. If any step in the liveness check
    /// fails (missing connection, method call error, async resolution error, or invalid
    /// return value), the method returns `false`.
    ///
    /// # Returns
    ///
    /// Returns `true` if the connection instance was successfully created, the connection
    /// object exists, the Python `is_alive()` method executes successfully, and returns
    /// a truthy value. Returns `false` if:
    /// - The connection instance has a creation error
    /// - The connection object is missing from the instance
    /// - The Python connection's `is_alive()` method raises an exception
    /// - The async resolution of the Python method fails (if the plugin uses async)
    /// - The Python return value cannot be extracted as a boolean
    /// - The Python return value is `false` or falsy
    fn is_alive(&self) -> bool {
        if self.create_error.is_some() {
            return false;
        }
        let Some(connection) = self.connection.as_ref() else {
            return false;
        };

        Python::attach(|py| {
            let connection = connection.bind(py);
            let value = match connection.call_method0("is_alive") {
                Ok(value) => value,
                Err(_) => return false,
            };
            let value = match resolve_python_maybe_awaitable(py, value) {
                Ok(value) => value,
                Err(_) => return false,
            };
            value.bind(py).extract::<bool>().unwrap_or(false)
        })
    }
}

struct PyProcessorPlugin {
    name: String,
    group: String,
    processor: Arc<Py<PyAny>>,
}

impl Plugin for PyProcessorPlugin {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn group(&self) -> String {
        self.group.clone()
    }
}

/// Extracts and validates a plugin identity value from an attribute.
///
/// Python plugin base classes expose `name` as a string attribute and `group`
/// as a locked property.
///
/// # Parameters
///
/// * `plugin` - A reference to the bound Python plugin object from which to extract the
///   identity value. This object must have the specified attribute available as
///   a string value.
/// * `method_name` - The name of the identity attribute to read from the plugin
///   object. Common examples include "name" for the plugin name or "group" for
///   the plugin group.
/// * `empty_message` - The error message to return if the attribute contains an empty or
///   whitespace-only string. This allows callers to provide context-specific error
///   messages for different identity values (e.g., "plugin name cannot be empty").
/// * `plugin_kind` - A descriptive string identifying the type of plugin being validated,
///   used in error messages to provide context. Examples include "InventoryPlugin",
///   "RunnerPlugin", or "ProcessorPlugin". This helps users identify which plugin type
///   is causing validation failures.
///
/// # Returns
///
/// Returns `Ok(String)` containing the extracted identity value if all validation passes:
/// - The attribute exists on the plugin object
/// - The attribute is a string value
/// - The string is not empty or whitespace-only
///
/// Returns `Err(PyErr)` if any validation fails:
/// - `PyValueError` if the attribute does not exist on the plugin object
/// - `PyValueError` if the attribute is callable
/// - `PyErr` if the identity value cannot be extracted as a string
/// - `PyValueError` if the extracted string is empty or contains only whitespace
///
/// # Errors
///
/// This function will return an error if the plugin does not conform to the expected
/// interface or if the value is invalid.
fn extract_plugin_identity_value(
    plugin: &Bound<'_, PyAny>,
    method_name: &str,
    empty_message: &str,
    plugin_kind: &str,
) -> PyResult<String> {
    let attribute = plugin.getattr(method_name).map_err(|_| {
        PyValueError::new_err(format!(
            "{plugin_kind} must define a '{method_name}' string property"
        ))
    })?;
    if attribute.is_callable() {
        return Err(PyValueError::new_err(format!(
            "{plugin_kind} attribute '{method_name}' must be a string property"
        )));
    }
    let value: String = attribute.extract()?;
    if value.trim().is_empty() {
        return Err(PyValueError::new_err(empty_message.to_string()));
    }
    Ok(value)
}

impl PluginProcessor for PyProcessorPlugin {
    fn processor(&self) -> Arc<dyn TaskProcessor> {
        Arc::new(PyTaskProcessor {
            processor: Arc::clone(&self.processor),
        })
    }
}

/// A Rust adapter for Python task processor implementations.
///
/// This struct wraps a Python task processor object and implements the `TaskProcessor`
/// trait to integrate Python-based processors into the Rust task execution system. It
/// delegates all task processing lifecycle events to the corresponding methods on the
/// wrapped Python processor object, handling cross-language communication and error
/// conversion. Processor hooks are synchronous, matching the Rust `TaskProcessor`
/// trait.
///
/// The adapter checks for the presence of each lifecycle method on the Python processor
/// before attempting to call it, allowing Python implementations to selectively implement
/// only the hooks they need. This design provides flexibility for processor plugins to
/// focus on specific aspects of task execution without requiring boilerplate for unused
/// hooks.
///
/// # Fields
///
/// * `processor` - An `Arc`-wrapped reference to the Python processor object that
///   implements the task processing lifecycle methods. This reference is shared across
///   all task processing operations and allows the processor to maintain state between
///   lifecycle events.
struct PyTaskProcessor {
    processor: Arc<Py<PyAny>>,
}

impl TaskProcessor for PyTaskProcessor {
    /// Invokes the Python processor's `on_task_start` hook when a task begins execution.
    ///
    /// This method is called at the beginning of task execution, before any host instances
    /// are processed. It delegates to the Python processor's `on_task_start()` method if
    /// it exists, allowing Python plugins to inspect and potentially modify the task results
    /// before execution begins. The method handles cross-language communication and converts
    /// any Python errors to Rust errors.
    ///
    /// # Parameters
    ///
    /// * `context` - A reference to the `TaskProcessorContext` containing metadata about the
    ///   current task execution, including task name, parent task name, depth, and hostname.
    ///   This context is converted to a Python-compatible format and passed to the Python hook.
    /// * `results` - A mutable reference to the `TaskResults` that will be populated during
    ///   task execution. The Python hook can modify these results before execution begins.
    ///   If the Python hook returns a non-None value, the results will be replaced with the
    ///   returned value.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the hook executes successfully or if the Python processor does not
    /// implement the `on_task_start` method. Returns `Err(genja_core::GenjaError)` if the
    /// Python hook raises an exception or if any cross-language conversion fails.
    fn on_task_start(
        &self,
        context: &TaskProcessorContext,
        results: &mut TaskResults,
    ) -> Result<(), genja_core::GenjaError> {
        self.call_task_results_hook("on_task_start", context, results)
    }

    /// Invokes the Python processor's `on_task_finish` hook when a task completes execution.
    ///
    /// This method is called after all host instances have been processed and the task
    /// execution is complete. It delegates to the Python processor's `on_task_finish()` method
    /// if it exists, allowing Python plugins to inspect and potentially modify the final task
    /// results. The method handles cross-language communication and converts any Python errors
    /// to Rust errors.
    ///
    /// # Parameters
    ///
    /// * `context` - A reference to the `TaskProcessorContext` containing metadata about the
    ///   completed task execution, including task name, parent task name, depth, and hostname.
    ///   This context is converted to a Python-compatible format and passed to the Python hook.
    /// * `results` - A mutable reference to the `TaskResults` containing the final execution
    ///   results for all host instances. The Python hook can modify these results after
    ///   execution completes. If the Python hook returns a non-None value, the results will
    ///   be replaced with the returned value.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the hook executes successfully or if the Python processor does not
    /// implement the `on_task_finish` method. Returns `Err(genja_core::GenjaError)` if the
    /// Python hook raises an exception or if any cross-language conversion fails.
    fn on_task_finish(
        &self,
        context: &TaskProcessorContext,
        results: &mut TaskResults,
    ) -> Result<(), genja_core::GenjaError> {
        self.call_task_results_hook("on_task_finish", context, results)
    }

    /// Invokes the Python processor's `on_instance_start` hook when a host instance begins execution.
    ///
    /// This method is called before executing a task on a specific host instance. It checks if
    /// the Python processor implements the `on_instance_start()` method and calls it if present,
    /// allowing Python plugins to perform setup or logging before the instance executes. Unlike
    /// the task-level hooks, this method does not modify any results but provides a notification
    /// point for instance-level processing.
    ///
    /// # Parameters
    ///
    /// * `context` - A reference to the `TaskProcessorContext` containing metadata about the
    ///   current instance execution, including task name, parent task name, depth, and the
    ///   specific hostname being processed. This context is converted to a Python-compatible
    ///   format and passed to the Python hook.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the hook executes successfully or if the Python processor does not
    /// implement the `on_instance_start` method. Returns `Err(genja_core::GenjaError)` if the
    /// Python hook raises an exception, if checking for the method's existence fails, or if
    /// any cross-language conversion fails.
    fn on_instance_start(
        &self,
        context: &TaskProcessorContext,
    ) -> Result<(), genja_core::GenjaError> {
        Python::attach(|py| {
            let processor = self.processor.bind(py);
            if !processor
                .hasattr("on_instance_start")
                .map_err(python_processor_error)?
            {
                return Ok(());
            }
            let context_payload =
                build_python_processor_context(py, context).map_err(python_processor_error)?;
            processor
                .call_method1("on_instance_start", (context_payload,))
                .map_err(python_processor_error)?;
            Ok(())
        })
    }

    /// Invokes the Python processor's `on_instance_finish` hook when a host instance completes execution.
    ///
    /// This method is called after executing a task on a specific host instance. It checks if
    /// the Python processor implements the `on_instance_finish()` method and calls it if present,
    /// allowing Python plugins to inspect and potentially modify the execution result for that
    /// specific host. If the Python hook returns a non-None value, the host result will be
    /// replaced with the returned value.
    ///
    /// # Parameters
    ///
    /// * `context` - A reference to the `TaskProcessorContext` containing metadata about the
    ///   completed instance execution, including task name, parent task name, depth, and the
    ///   specific hostname that was processed. This context is converted to a Python-compatible
    ///   format and passed to the Python hook.
    /// * `result` - A mutable reference to the `HostTaskResult` containing the execution result
    ///   for the specific host instance. The Python hook can modify this result after execution
    ///   completes. If the Python hook returns a non-None value, the result will be replaced
    ///   with the returned value.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the hook executes successfully or if the Python processor does not
    /// implement the `on_instance_finish` method. Returns `Err(genja_core::GenjaError)` if the
    /// Python hook raises an exception, if checking for the method's existence fails, or if
    /// any cross-language conversion fails.
    fn on_instance_finish(
        &self,
        context: &TaskProcessorContext,
        result: &mut HostTaskResult,
    ) -> Result<(), genja_core::GenjaError> {
        Python::attach(|py| {
            let processor = self.processor.bind(py);
            if !processor
                .hasattr("on_instance_finish")
                .map_err(python_processor_error)?
            {
                return Ok(());
            }
            let context_payload =
                build_python_processor_context(py, context).map_err(python_processor_error)?;
            let result_payload = Py::new(
                py,
                PyHostTaskResult {
                    inner: result.clone(),
                },
            )
            .map_err(python_processor_error)?;
            let replacement = processor
                .call_method1(
                    "on_instance_finish",
                    (context_payload, result_payload.bind(py)),
                )
                .map_err(python_processor_error)?;
            if !replacement.is_none() {
                *result = python_result_to_host_task_result(replacement)
                    .map_err(python_processor_error)?;
            }
            Ok(())
        })
    }
}

impl PyTaskProcessor {
    /// Invokes a Python processor hook method that operates on task-level results.
    ///
    /// This helper method provides a common implementation for calling Python processor
    /// lifecycle hooks that receive task context and task results as parameters. It checks
    /// if the specified method exists on the Python processor, calls it with the provided
    /// context and results, and optionally replaces the results if the Python hook returns
    /// a non-None value. This method handles cross-language communication, error conversion,
    /// and result replacement logic that is shared between `on_task_start` and `on_task_finish`.
    ///
    /// # Parameters
    ///
    /// * `method_name` - The name of the Python processor method to invoke, such as
    ///   "on_task_start" or "on_task_finish". The method must accept two parameters:
    ///   a task processor context and task results object.
    /// * `context` - A reference to the `TaskProcessorContext` containing metadata about
    ///   the current task execution, including task name, parent task name, depth, and
    ///   hostname. This context is converted to a Python-compatible format and passed
    ///   to the Python hook.
    /// * `results` - A mutable reference to the `TaskResults` containing the execution
    ///   results for all host instances. The Python hook can modify these results, and
    ///   if the hook returns a non-None value, the results will be replaced with the
    ///   returned value.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the hook executes successfully or if the Python processor does
    /// not implement the specified method. Returns `Err(genja_core::GenjaError)` if:
    /// - Checking for the method's existence fails
    /// - Converting the context to Python format fails
    /// - Creating the Python results wrapper fails
    /// - The Python hook raises an exception
    /// - Converting the Python return value back to Rust `TaskResults` fails
    fn call_task_results_hook(
        &self,
        method_name: &str,
        context: &TaskProcessorContext,
        results: &mut TaskResults,
    ) -> Result<(), genja_core::GenjaError> {
        Python::attach(|py| {
            let processor = self.processor.bind(py);
            if !processor
                .hasattr(method_name)
                .map_err(python_processor_error)?
            {
                return Ok(());
            }
            let context_payload =
                build_python_processor_context(py, context).map_err(python_processor_error)?;
            let results_payload = Py::new(
                py,
                PyTaskResults {
                    inner: results.clone(),
                },
            )
            .map_err(python_processor_error)?;
            let replacement = processor
                .call_method1(method_name, (context_payload, results_payload.bind(py)))
                .map_err(python_processor_error)?;
            if !replacement.is_none() {
                *results =
                    python_result_to_task_results(replacement).map_err(python_processor_error)?;
            }
            Ok(())
        })
    }
}

/// Imports and instantiates a Python plugin from a module path specification.
///
/// This function dynamically imports a Python plugin by parsing an import path string
/// in the format "module:attribute.path" and using Python's `importlib` to load the
/// specified module and traverse to the target attribute. The import path consists of
/// a module name (which can include dots for nested packages) followed by a colon and
/// an attribute path (which can also include dots for nested attributes). Once the
/// target attribute is located, if it is callable (such as a class), it will be
/// instantiated by calling it with no arguments. If it is not callable (such as an
/// already-instantiated object), it will be returned as-is.
///
/// This function is used during plugin registration to load Python plugin classes or
/// instances from configuration files, allowing plugins to be specified as import paths
/// rather than requiring direct Python code execution.
///
/// # Parameters
///
/// * `py` - A Python GIL token that provides access to the Python interpreter. This
///   token ensures that the Python interpreter is available and that the operation is
///   performed safely within the GIL context.
/// * `import_path` - A string slice specifying the plugin location in the format
///   "module:attribute.path". The module name (before the colon) is passed to
///   `importlib.import_module()`, and the attribute path (after the colon) is traversed
///   using `getattr()` calls. For example, "my_plugins:ConnectionPlugin" would import
///   the `my_plugins` module and retrieve the `ConnectionPlugin` attribute, while
///   "my_plugins:nested.ConnectionPlugin" would import `my_plugins` and traverse through
///   `nested` to reach `ConnectionPlugin`.
///
/// # Returns
///
/// Returns `Ok(Py<PyAny>)` containing the imported and potentially instantiated plugin
/// object. If the target attribute is callable, this will be the result of calling it
/// with no arguments. If the target attribute is not callable, this will be the
/// attribute itself. Returns `Err(PyErr)` if:
/// - The import path does not contain a colon separator
/// - The module cannot be imported
/// - Any attribute in the path does not exist
/// - Calling the target attribute (if callable) raises an exception
///
/// # Errors
///
/// This function will return an error if the import path format is invalid, if the
/// module or any attribute in the path cannot be found, or if instantiating a callable
/// attribute fails.
fn import_python_plugin<'py>(py: Python<'py>, import_path: &str) -> PyResult<Py<PyAny>> {
    let (module_name, attr_path) = import_path.split_once(':').ok_or_else(|| {
        PyValueError::new_err(format!(
            "python plugin import path '{import_path}' must be in 'module:attribute' form"
        ))
    })?;
    let importlib = PyModule::import(py, "importlib")?;
    let module = importlib.call_method1("import_module", (module_name,))?;
    let mut current = module;
    for attr in attr_path.split('.') {
        current = current.getattr(attr)?;
    }

    let instance = if current.is_callable() {
        current.call0()?
    } else {
        current
    };
    Ok(instance.unbind())
}

/// Converts a Rust `TaskProcessorContext` into a Python model object.
///
/// This function creates a Python representation of a task processor context by extracting
/// the context's fields (task name, parent task name, depth, and hostname) and packaging
/// them into a Python dictionary. The dictionary is then used to instantiate a Python
/// `TaskProcessorContext` model object from the `genja.processor` module. This
/// conversion enables Rust task processor contexts to be passed to Python processor
/// plugins, allowing Python code to access task execution metadata.
///
/// The function handles optional fields (parent task name and hostname) by converting
/// `None` values to Python's `None` object, ensuring that the Python model receives
/// properly typed null values rather than missing keys.
///
/// # Parameters
///
/// * `py` - A Python GIL token that provides access to the Python interpreter. This
///   token ensures that the Python interpreter is available and that the operation is
///   performed safely within the GIL context.
/// * `context` - A reference to the `TaskProcessorContext` containing the task execution
///   metadata to be converted. This includes the task name, optional parent task name,
///   execution depth, and optional hostname. All fields are extracted and converted to
///   Python-compatible types.
///
/// # Returns
///
/// Returns `Ok(Py<PyAny>)` containing a Python `TaskProcessorContext` model object
/// with all fields populated from the Rust context. The returned object can be passed
/// to Python processor plugin methods. Returns `Err(PyErr)` if:
/// - Setting any dictionary item fails
/// - Importing the `genja.processor` module fails
/// - Instantiating the `TaskProcessorContext` class fails
/// - Any other Python operation encounters an error
///
/// # Errors
///
/// This function will return an error if any Python operation fails during the
/// conversion process, including dictionary manipulation or model instantiation.
fn build_python_processor_context<'py>(
    py: Python<'py>,
    context: &TaskProcessorContext,
) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    payload.set_item("task_name", context.task_name())?;
    match context.parent_task_name() {
        Some(parent_task_name) => payload.set_item("parent_task_name", parent_task_name)?,
        None => payload.set_item("parent_task_name", py.None())?,
    }
    payload.set_item("depth", context.depth())?;
    match context.hostname() {
        Some(hostname) => payload.set_item("hostname", hostname)?,
        None => payload.set_item("hostname", py.None())?,
    }
    build_python_model(py, "genja.processor", "TaskProcessorContext", payload)
}

/// Converts a Rust value that implements `Serialize` into a Python object.
///
/// This function serializes a Rust value to JSON using `serde_json`, then deserializes
/// it into a Python object using Python's `json.loads()`. This provides a generic way
/// to convert Rust data structures into Python-compatible representations that can be
/// passed to Python plugin methods or used in cross-language communication. The function
/// handles the serialization and deserialization process, converting any serialization
/// errors into Python exceptions.
///
/// # Parameters
///
/// * `py` - A Python GIL token that provides access to the Python interpreter. This
///   token ensures that the Python interpreter is available and that the operation is
///   performed safely within the GIL context.
/// * `value` - A reference to the Rust value to be converted to Python. The value must
///   implement the `Serialize` trait from `serde`, allowing it to be serialized to JSON.
///   Common types include structs, enums, collections, and primitive types that derive
///   or implement `Serialize`.
///
/// # Returns
///
/// Returns `Ok(Py<PyAny>)` containing the Python object representation of the serialized
/// value. The returned object can be passed to Python functions or methods. Returns
/// `Err(PyErr)` if:
/// - The Rust value cannot be serialized to JSON (wrapped as `PyValueError`)
/// - The `json` module cannot be imported
/// - The `json.loads()` call fails to parse the serialized JSON
/// - Any other Python operation encounters an error
///
/// # Errors
///
/// This function will return an error if serialization fails or if any Python operation
/// fails during the conversion process.
fn serde_to_python_payload<T>(py: Python<'_>, value: &T) -> PyResult<Py<PyAny>>
where
    T: Serialize,
{
    let dumped = serde_json::to_string(value)
        .map_err(|err| PyValueError::new_err(format!("failed to serialize payload: {err}")))?;
    let json = PyModule::import(py, "json")?;
    Ok(json.call_method1("loads", (dumped,))?.unbind())
}

/// Converts optional transform function options into a Python object or None.
///
/// This function handles the conversion of Rust transform function options into a
/// Python-compatible representation that can be passed to Python transform plugins.
/// If options are provided, they are serialized to JSON and then deserialized into
/// a Python object using the `serde_to_python_payload` helper. If no options are
/// provided, the function returns Python's `None` object. This allows transform
/// plugins to receive either a populated options object or `None`, matching the
/// optional nature of transform function configuration.
///
/// # Parameters
///
/// * `py` - A Python GIL token that provides access to the Python interpreter. This
///   token ensures that the Python interpreter is available and that the operation is
///   performed safely within the GIL context.
/// * `options` - An optional reference to `TransformFunctionOptions` containing the
///   configuration parameters for a transform function. If `Some`, the options will
///   be serialized and converted to a Python object. If `None`, Python's `None` will
///   be returned.
///
/// # Returns
///
/// Returns `Ok(Py<PyAny>)` containing either a Python object representation of the
/// transform options (if provided) or Python's `None` object (if no options were
/// provided). The returned object can be passed to Python transform plugin methods.
/// Returns `Err(PyErr)` if:
/// - The options cannot be serialized to JSON (wrapped as `PyValueError`)
/// - The `json` module cannot be imported
/// - The `json.loads()` call fails to parse the serialized JSON
/// - Any other Python operation encounters an error during conversion
///
/// # Errors
///
/// This function will return an error if serialization fails or if any Python operation
/// fails during the conversion process when options are provided.
fn transform_options_to_python_payload(
    py: Python<'_>,
    options: Option<&TransformFunctionOptions>,
) -> PyResult<Py<PyAny>> {
    match options {
        Some(options) => serde_to_python_payload(py, options),
        None => Ok(py.None()),
    }
}

/// Converts a Python object into a Rust value by deserializing through JSON.
///
/// This function converts a Python object into a Rust type by first normalizing the
/// Python object into a JSON-serializable format, then serializing it to a JSON string,
/// and finally deserializing it into the target Rust type using `serde_json`. The
/// normalization step handles different Python object types by attempting to call
/// `model_dump()` (for Pydantic models), `to_dict()` (for custom classes), or using
/// the object directly if neither method is available. This provides a flexible way
/// to convert Python plugin return values into strongly-typed Rust data structures.
///
/// # Parameters
///
/// * `obj` - A reference to the bound Python object to be converted. The object should
///   either implement `model_dump()` (Pydantic models), `to_dict()` (custom classes),
///   or be directly JSON-serializable by Python's `json.dumps()`. The object is
///   normalized before serialization to ensure compatibility with JSON conversion.
/// * `error_prefix` - A string slice that will be prepended to any deserialization
///   error messages. This allows callers to provide context-specific error messages
///   that help identify which conversion operation failed (e.g., "failed to convert
///   task results" or "invalid connection key payload").
///
/// # Returns
///
/// Returns `Ok(T)` containing the deserialized Rust value if the conversion succeeds.
/// The type `T` must implement `DeserializeOwned` from `serde`. Returns `Err(PyErr)`
/// if:
/// - Checking for the `model_dump` or `to_dict` attributes fails
/// - Calling `model_dump()` or `to_dict()` raises a Python exception
/// - Creating the `mode="json"` parameter dictionary fails
/// - Importing the `json` module fails
/// - The `json.dumps()` call fails to serialize the normalized object
/// - Extracting the JSON string from the Python return value fails
/// - The `serde_json::from_str()` deserialization fails (wrapped as `PyValueError`
///   with the provided error prefix)
///
/// # Errors
///
/// This function will return an error if any step in the normalization, serialization,
/// or deserialization process fails, or if the Python object cannot be converted to
/// the target Rust type.
fn python_payload_to_rust_value<T>(obj: &Bound<'_, PyAny>, error_prefix: &str) -> PyResult<T>
where
    T: DeserializeOwned,
{
    let normalized = if obj.hasattr("model_dump")? {
        obj.call_method(
            "model_dump",
            (),
            Some(&PyDict::from_sequence(
                &[("mode", "json")].into_pyobject(obj.py())?,
            )?),
        )?
    } else if obj.hasattr("to_dict")? {
        obj.call_method0("to_dict")?
    } else {
        obj.clone()
    };

    let json = PyModule::import(obj.py(), "json")?;
    let dumped: String = json.call_method1("dumps", (normalized,))?.extract()?;
    serde_json::from_str(&dumped)
        .map_err(|err| PyValueError::new_err(format!("{error_prefix}: {err}")))
}

/// Converts a Rust `ConnectionKey` into a Python model object.
///
/// This function creates a Python representation of a connection key by extracting
/// the key's fields (hostname and plugin name) and packaging them into a Python
/// dictionary. The dictionary is then used to instantiate a Python `ConnectionKey`
/// model object from the `genja.connection` module. This conversion enables
/// Rust connection keys to be passed to Python connection plugins, allowing Python
/// code to identify and reference specific connections.
///
/// # Parameters
///
/// * `py` - A Python GIL token that provides access to the Python interpreter. This
///   token ensures that the Python interpreter is available and that the operation is
///   performed safely within the GIL context.
/// * `key` - A reference to the `ConnectionKey` containing the connection identifier
///   to be converted. This includes the hostname and plugin name that uniquely
///   identify a connection instance. Both fields are extracted and converted to
///   Python-compatible types.
///
/// # Returns
///
/// Returns `Ok(Py<PyAny>)` containing a Python `ConnectionKey` model object with
/// all fields populated from the Rust connection key. The returned object can be
/// passed to Python connection plugin methods. Returns `Err(PyErr)` if:
/// - Setting any dictionary item fails
/// - Importing the `genja.connection` module fails
/// - Instantiating the `ConnectionKey` class fails
/// - Any other Python operation encounters an error
///
/// # Errors
///
/// This function will return an error if any Python operation fails during the
/// conversion process, including dictionary manipulation or model instantiation.
fn build_python_connection_key<'py>(py: Python<'py>, key: &ConnectionKey) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    payload.set_item("hostname", &key.hostname)?;
    payload.set_item("plugin_name", &key.plugin_name)?;
    build_python_model(py, "genja.connection", "ConnectionKey", payload)
}

/// Converts a Rust `ResolvedConnectionParams` into a Python model object.
///
/// This function creates a Python representation of resolved connection parameters by
/// extracting all parameter fields (hostname, port, username, password, platform, and
/// extras) and packaging them into a Python dictionary. The dictionary is then used to
/// instantiate a Python `ResolvedConnectionParams` model object from the
/// `genja.connection` module. This conversion enables Rust connection parameters
/// to be passed to Python connection plugins, allowing Python code to access all
/// connection configuration details needed to establish connections.
///
/// The function handles optional fields (port, username, password, platform, and extras)
/// by converting `None` values to Python's `None` object, ensuring that the Python model
/// receives properly typed null values rather than missing keys. The extras field, if
/// present, is serialized to JSON and then deserialized into a Python object to preserve
/// its structure and allow Python code to access nested configuration values.
///
/// # Parameters
///
/// * `py` - A Python GIL token that provides access to the Python interpreter. This
///   token ensures that the Python interpreter is available and that the operation is
///   performed safely within the GIL context.
/// * `params` - A reference to the `ResolvedConnectionParams` containing the connection
///   configuration to be converted. This includes the hostname, optional port, optional
///   username, optional password, optional platform identifier, and optional extras map.
///   All fields are extracted and converted to Python-compatible types.
///
/// # Returns
///
/// Returns `Ok(Py<PyAny>)` containing a Python `ResolvedConnectionParams` model object
/// with all fields populated from the Rust connection parameters. The returned object
/// can be passed to Python connection plugin methods. Returns `Err(PyErr)` if:
/// - Setting any dictionary item fails
/// - Serializing the extras map to JSON fails (wrapped as `PyValueError`)
/// - Importing the `json` module fails
/// - Deserializing the JSON extras string fails
/// - Importing the `genja.connection` module fails
/// - Instantiating the `ResolvedConnectionParams` class fails
/// - Any other Python operation encounters an error
///
/// # Errors
///
/// This function will return an error if any Python operation fails during the
/// conversion process, including dictionary manipulation, JSON serialization/deserialization,
/// or model instantiation.
fn build_python_resolved_connection_params<'py>(
    py: Python<'py>,
    params: &ResolvedConnectionParams,
) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    payload.set_item("hostname", &params.hostname)?;
    payload.set_item("port", params.port)?;
    payload.set_item("username", params.username.as_ref())?;
    payload.set_item("password", params.password.as_ref())?;
    payload.set_item("platform", params.platform.as_ref())?;
    match params.extras.as_ref() {
        Some(extras) => {
            let json_module = PyModule::import(py, "json")?;
            let dumped = serde_json::to_string(extras).map_err(|err| {
                PyValueError::new_err(format!("failed to serialize extras: {err}"))
            })?;
            payload.set_item("extras", json_module.call_method1("loads", (dumped,))?)?;
        }
        None => payload.set_item("extras", py.None())?,
    }
    build_python_model(py, "genja.connection", "ResolvedConnectionParams", payload)
}

/// Converts a Python object into a Rust `ConnectionKey` by deserializing through JSON.
///
/// This function converts a Python object representing a connection key into a Rust
/// `ConnectionKey` by first normalizing the Python object into a JSON-serializable format,
/// then serializing it to a JSON string, and finally extracting the required fields
/// (hostname and plugin_name) to construct a `ConnectionKey`. The normalization step
/// handles different Python object types by attempting to call `model_dump()` (for
/// Pydantic models), `to_dict()` (for custom classes), or using the object directly if
/// neither method is available. This provides a flexible way to convert Python connection
/// key representations into strongly-typed Rust connection keys.
///
/// # Parameters
///
/// * `obj` - A reference to the bound Python object to be converted into a connection key.
///   The object should either implement `model_dump()` (Pydantic models), `to_dict()`
///   (custom classes), or be directly JSON-serializable by Python's `json.dumps()`. The
///   object must contain `hostname` and `plugin_name` fields that can be extracted as
///   strings from the JSON representation.
///
/// # Returns
///
/// Returns `Ok(ConnectionKey)` containing the constructed connection key with hostname
/// and plugin_name extracted from the Python object. Returns `Err(PyErr)` if:
/// - Checking for the `model_dump` or `to_dict` attributes fails
/// - Calling `model_dump()` or `to_dict()` raises a Python exception
/// - Creating the `mode="json"` parameter dictionary fails
/// - Importing the `json` module fails
/// - The `json.dumps()` call fails to serialize the normalized object
/// - Extracting the JSON string from the Python return value fails
/// - The `serde_json::from_str()` deserialization fails (wrapped as `PyValueError`)
/// - The JSON payload is missing the required `hostname` field
/// - The JSON payload is missing the required `plugin_name` field
/// - Either field cannot be extracted as a string
///
/// # Errors
///
/// This function will return an error if any step in the normalization, serialization,
/// or field extraction process fails, or if the Python object does not contain the
/// required connection key fields.
fn py_any_to_connection_key(obj: &Bound<'_, PyAny>) -> PyResult<ConnectionKey> {
    let normalized = if obj.hasattr("model_dump")? {
        obj.call_method(
            "model_dump",
            (),
            Some(&PyDict::from_sequence(
                &[("mode", "json")].into_pyobject(obj.py())?,
            )?),
        )?
    } else if obj.hasattr("to_dict")? {
        obj.call_method0("to_dict")?
    } else {
        obj.clone()
    };
    let json_module = PyModule::import(obj.py(), "json")?;
    let dumped: String = json_module
        .call_method1("dumps", (normalized,))?
        .extract()?;
    let value: serde_json::Value = serde_json::from_str(&dumped)
        .map_err(|err| PyValueError::new_err(format!("invalid connection key payload: {err}")))?;
    let hostname = value
        .get("hostname")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| PyValueError::new_err("connection key payload is missing 'hostname'"))?;
    let plugin_name = value
        .get("plugin_name")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| PyValueError::new_err("connection key payload is missing 'plugin_name'"))?;
    Ok(ConnectionKey::new(hostname, plugin_name))
}

/// Instantiates a Python model class with keyword arguments from a specified module.
///
/// This function dynamically imports a Python module, retrieves a class from that module,
/// and instantiates it by calling the class constructor with the provided keyword arguments.
/// The function is used throughout the plugin system to create Python model objects (such as
/// `TaskProcessorContext`, `ConnectionKey`, or `ResolvedConnectionParams`) from Rust data
/// structures. It provides a generic way to construct Python objects that conform to expected
/// model interfaces, enabling cross-language data transfer between Rust and Python plugin code.
///
/// The function handles the complete instantiation process: importing the module, retrieving
/// the class attribute, calling the class constructor with keyword arguments, and unbinding
/// the result to create a GIL-independent reference that can be stored or passed across
/// Python calls.
///
/// # Parameters
///
/// * `py` - A Python GIL token that provides access to the Python interpreter. This token
///   ensures that the Python interpreter is available and that the operation is performed
///   safely within the GIL context.
/// * `module_name` - The fully qualified name of the Python module to import, such as
///   "genja.processor" or "genja.connection". The module must be available in
///   the Python environment and contain the specified class.
/// * `class_name` - The name of the class to retrieve from the imported module and instantiate.
///   The class must exist as an attribute of the module and be callable (typically a class
///   constructor). Common examples include "TaskProcessorContext", "ConnectionKey", or
///   "ResolvedConnectionParams".
/// * `kwargs` - A bound Python dictionary containing the keyword arguments to pass to the
///   class constructor. The dictionary keys should match the parameter names expected by
///   the class's `__init__` method, and the values should be Python-compatible objects.
///
/// # Returns
///
/// Returns `Ok(Py<PyAny>)` containing a GIL-independent reference to the instantiated Python
/// model object. The returned object can be stored, passed to other Python functions, or
/// converted back to Rust types. Returns `Err(PyErr)` if:
/// - The specified module cannot be imported
/// - The class attribute does not exist in the module
/// - Calling the class constructor with the provided keyword arguments fails
/// - Any other Python operation encounters an error during instantiation
///
/// # Errors
///
/// This function will return an error if the module import fails, if the class does not
/// exist in the module, or if the class constructor raises an exception when called with
/// the provided keyword arguments.
fn build_python_model<'py>(
    py: Python<'py>,
    module_name: &str,
    class_name: &str,
    kwargs: Bound<'py, PyDict>,
) -> PyResult<Py<PyAny>> {
    let module = PyModule::import(py, module_name)?;
    let class = module.getattr(class_name)?;
    Ok(class.call((), Some(&kwargs))?.unbind())
}

/// Converts a Python error into a Genja core error.
///
/// This function provides a simple conversion from PyO3's `PyErr` type to the
/// `genja_core::GenjaError` type by extracting the error message as a string.
/// It is used throughout the plugin system to convert Python exceptions raised
/// during plugin operations into Rust errors that can be propagated through the
/// Genja core error handling system. The conversion preserves the error message
/// but loses Python-specific error type information, presenting all Python errors
/// as generic message errors in the Rust error system.
///
/// # Parameters
///
/// * `err` - The Python error to be converted. This error typically originates
///   from Python plugin method calls, attribute access, or other Python operations
///   that can raise exceptions. The error's string representation (obtained via
///   `to_string()`) is extracted and wrapped in a Genja error.
///
/// # Returns
///
/// Returns a `genja_core::GenjaError::Message` variant containing the string
/// representation of the Python error. This error can be propagated through
/// Rust code and will display the original Python error message when formatted.
fn python_processor_error(err: PyErr) -> genja_core::GenjaError {
    genja_core::GenjaError::Message(err.to_string())
}

/// Registers the `PyPluginManager` class with a Python module.
///
/// This function adds the `PyPluginManager` class to the specified Python module,
/// making it available for import and use in Python code. It is typically called
/// during the module initialization process to expose the plugin manager functionality
/// to Python. The function handles the registration process and propagates any errors
/// that occur during class registration.
///
/// # Parameters
///
/// * `module` - A reference to the bound Python module where the `PyPluginManager`
///   class should be registered. The module must be valid and accessible within the
///   current Python GIL context. After successful registration, Python code can import
///   and instantiate `PyPluginManager` from this module.
///
/// # Returns
///
/// Returns `Ok(())` if the `PyPluginManager` class is successfully added to the module.
/// Returns `Err(PyErr)` if the class registration fails, which can occur if the module
/// is invalid or if there are conflicts with existing module attributes.
///
/// # Errors
///
/// This function will return an error if adding the class to the module fails, typically
/// due to module state issues or attribute conflicts.
pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyPluginManager>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use genja_core::inventory::{
        BaseBuilderHost, ConnectionManager, Defaults, Group, Host, TransformFunctionOptions,
    };
    use genja_core::task::{
        Task, TaskError, TaskExecutionMode, TaskInfo, TaskRuntimeContext, TaskSuccess, Tasks,
    };
    use genja_plugin_manager::connection_factory::build_connection_factory;
    use serde_json::{Value, json};
    use std::env;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::runtime::Builder;

    fn run_async<F: std::future::Future>(future: F) -> F::Output {
        Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(future)
    }

    fn run_python_async<F, T>(py: Python<'_>, future: F) -> T
    where
        F: std::future::Future<Output = T> + Send + 'static,
        T: Send + Sync + 'static,
    {
        pyo3_async_runtimes::tokio::run(py, async move { Ok(future.await) })
            .expect("test async runtime should complete")
    }

    fn init_python() {
        crate::init_embedded_python();
        Python::attach(|py| {
            let asyncio = PyModule::import(py, "asyncio").expect("asyncio module should import");
            let platform = py
                .import("sys")
                .expect("sys module should import")
                .getattr("platform")
                .expect("sys.platform should exist")
                .extract::<String>()
                .expect("sys.platform should be a string");
            if platform == "win32" {
                let policy = asyncio
                    .getattr("WindowsSelectorEventLoopPolicy")
                    .expect("Windows selector event loop policy should exist")
                    .call0()
                    .expect("Windows selector event loop policy should instantiate");
                asyncio
                    .call_method1("set_event_loop_policy", (policy,))
                    .expect("Windows selector event loop policy should be set");
            }

            let sys = PyModule::import(py, "sys").expect("sys module should import");
            let modules = sys.getattr("modules").expect("sys.modules should exist");
            let genja = PyModule::from_code(
                py,
                pyo3::ffi::c_str!("__path__ = []\n"),
                pyo3::ffi::c_str!("genja/__init__.py"),
                pyo3::ffi::c_str!("genja"),
            )
            .expect("genja stub should build");
            let processor = PyModule::from_code(
                py,
                pyo3::ffi::c_str!(
                    "class TaskProcessorContext:\n    def __init__(self, **kwargs):\n        self.__dict__.update(kwargs)\n    def to_dict(self):\n        return dict(self.__dict__)\n"
                ),
                pyo3::ffi::c_str!("genja/processor.py"),
                pyo3::ffi::c_str!("genja.processor"),
            )
            .expect("processor stub should build");
            genja
                .add("processor", &processor)
                .expect("processor module should attach to package");
            modules
                .set_item("genja", &genja)
                .expect("genja stub should register");
            modules
                .set_item("genja.processor", &processor)
                .expect("processor stub should register");
            let connection = PyModule::from_code(
                py,
                pyo3::ffi::c_str!(
                    "class ConnectionKey:\n    def __init__(self, **kwargs):\n        self.__dict__.update(kwargs)\n    def to_dict(self):\n        return dict(self.__dict__)\n\nclass ResolvedConnectionParams:\n    def __init__(self, **kwargs):\n        self.__dict__.update(kwargs)\n    def to_dict(self):\n        return dict(self.__dict__)\n"
                ),
                pyo3::ffi::c_str!("genja/connection.py"),
                pyo3::ffi::c_str!("genja.connection"),
            )
            .expect("connection stub should build");
            genja
                .add("connection", &connection)
                .expect("connection module should attach to package");
            modules
                .set_item("genja.connection", &connection)
                .expect("connection stub should register");
        });
    }

    fn import_fixture_attr<'py>(
        py: Python<'py>,
        module_name: &str,
        attr_name: &str,
    ) -> PyResult<Bound<'py, PyAny>> {
        let importlib = PyModule::import(py, "importlib")?;
        let module = importlib.call_method1("import_module", (module_name,))?;
        module.getattr(attr_name)
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

    struct TestTask {
        name: String,
    }

    impl TaskInfo for TestTask {
        fn name(&self) -> &str {
            &self.name
        }
    }

    #[async_trait]
    impl Task for TestTask {
        async fn start_async(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(
                TaskSuccess::new().with_summary(format!("handled {}", self.name)),
            ))
        }

        fn execution_mode(&self) -> TaskExecutionMode {
            TaskExecutionMode::Async
        }
    }

    #[test]
    fn py_plugin_manager_new_includes_built_in_plugins() {
        let manager = PyPluginManager::new();
        let names = manager
            .plugin_names()
            .expect("built-in plugins should be available");

        assert!(names.iter().any(|name| name == "FileInventoryPlugin"));
        assert!(names.iter().any(|name| name == "serial"));
        assert!(names.iter().any(|name| name == "threaded"));
    }

    #[test]
    fn take_inner_consumes_plugin_manager() {
        let manager = PyPluginManager::new();

        let inner = manager
            .take_inner()
            .expect("plugin manager should be consumable");
        assert!(inner.get_plugin("serial").is_some());

        let err = manager
            .plugin_names()
            .expect_err("consumed manager should reject access");
        assert!(err.to_string().contains("already been consumed"));
    }

    #[test]
    fn register_adds_plugin_manager_class_to_module() {
        init_python();
        Python::attach(|py| {
            let module = PyModule::new(py, "test_plugin_manager_module")
                .expect("test module should be created");

            register(&module).expect("plugin manager class should register");

            assert!(module.getattr("PluginManager").is_ok());
        });
    }

    #[test]
    fn register_plugin_adds_processor_plugin() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.processor_plugins",
                "MinimalAuditProcessor",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");

            manager
                .register_plugin(plugin)
                .expect("plugin should register");

            let names = manager
                .plugin_names()
                .expect("plugin names should be available");
            assert!(names.iter().any(|name| name == "audit"));
            let groups = manager
                .plugin_names_and_groups()
                .expect("plugin groups should be available");
            assert!(
                groups
                    .iter()
                    .any(|(name, group)| name == "audit" && group == "Processor")
            );
        });
    }

    #[test]
    fn register_plugin_adds_inventory_plugin() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.inventory_plugins",
                "StaticInventoryPlugin",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");

            manager
                .register_plugin(plugin)
                .expect("inventory plugin should register");

            let names = manager
                .plugin_names()
                .expect("plugin names should be available");
            assert!(names.iter().any(|name| name == "python_inventory"));
            let groups = manager
                .plugin_names_and_groups()
                .expect("plugin groups should be available");
            assert!(
                groups
                    .iter()
                    .any(|(name, group)| name == "python_inventory" && group == "Inventory")
            );
        });
    }

    #[test]
    fn register_plugin_adds_runner_plugin() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.runner_plugins",
                "FirstHostOnlyRunnerPlugin",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");

            manager
                .register_plugin(plugin)
                .expect("runner plugin should register");

            let names = manager
                .plugin_names()
                .expect("plugin names should be available");
            assert!(names.iter().any(|name| name == "python_runner"));
            let groups = manager
                .plugin_names_and_groups()
                .expect("plugin groups should be available");
            assert!(
                groups
                    .iter()
                    .any(|(name, group)| name == "python_runner" && group == "Runner")
            );
        });
    }

    #[test]
    fn register_plugin_adds_transform_plugin() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.transform_plugins",
                "HostnameSuffixTransformPlugin",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");

            manager
                .register_plugin(plugin)
                .expect("transform plugin should register");

            let names = manager
                .plugin_names()
                .expect("plugin names should be available");
            assert!(names.iter().any(|name| name == "python_transform"));
            let groups = manager
                .plugin_names_and_groups()
                .expect("plugin groups should be available");
            assert!(groups
                .iter()
                .any(|(name, group)| name == "python_transform" && group == "TransformFunction"));
        });
    }

    #[test]
    fn deregister_plugin_removes_registered_plugin() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.inventory_plugins",
                "StaticInventoryPlugin",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");
            manager
                .register_plugin(plugin)
                .expect("inventory plugin should register");

            let deregistered = manager
                .deregister_plugin("python_inventory")
                .expect("deregister should succeed");
            assert_eq!(deregistered, Some("python_inventory".to_string()));
            assert!(
                !manager
                    .plugin_names()
                    .expect("plugin names should be available")
                    .iter()
                    .any(|name| name == "python_inventory")
            );
            assert_eq!(
                manager
                    .deregister_plugin("python_inventory")
                    .expect("second deregister should succeed"),
                None
            );
        });
    }

    #[test]
    fn register_plugin_requires_name_and_group_methods() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.processor_plugins",
                "MissingIdentityPlugin",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");

            let err = manager
                .register_plugin(plugin)
                .expect_err("plugin without name/group should fail");
            assert!(
                err.to_string()
                    .contains("plugin must define a 'name' string property")
            );
        });
    }

    #[test]
    fn register_plugin_rejects_unsupported_group() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.processor_plugins",
                "UnsupportedGroupPlugin",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");

            let err = manager
                .register_plugin(plugin)
                .expect_err("unsupported plugin group should fail");
            assert!(
                err.to_string()
                    .contains("unsupported python plugin group 'UnknownPlugin'")
            );
        });
    }

    #[test]
    fn consumed_manager_rejects_remaining_public_methods() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.processor_plugins",
                "MinimalAuditProcessor",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");
            manager
                .take_inner()
                .expect("plugin manager should be consumable");

            let temp_dir = temp_test_dir("consumed-pyproject");
            let pyproject_path = temp_dir.join("pyproject.toml");
            fs::write(
                &pyproject_path,
                r#"
[tool.genja.plugins.processor]
audit = "tests.fixtures.processor_plugins:MinimalAuditProcessor"
"#,
            )
            .expect("pyproject should be written");

            assert!(manager.plugin_names_and_groups().is_err());
            assert!(manager.deregister_plugin("audit").is_err());
            assert!(manager.register_plugin(plugin).is_err());
            assert!(
                manager
                    .load_python_plugins_from_pyproject(Some(pyproject_path.to_str().unwrap()))
                    .is_err()
            );
            assert!(
                manager
                    .load_rust_plugins_from_directory("/definitely/missing")
                    .is_err()
            );
            fs::remove_dir_all(&temp_dir).unwrap_or(());
        });
    }

    #[test]
    fn register_connection_plugin_supports_factory_open_and_close() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class =
                import_fixture_attr(py, "tests.fixtures.connection_plugins", "ConnectionPlugin")
                    .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");

            manager
                .register_plugin(plugin)
                .expect("connection plugin should register");

            let inner = Arc::new(
                manager
                    .take_inner()
                    .expect("plugin manager should be consumable"),
            );
            let factory = build_connection_factory(Arc::clone(&inner));
            let connection_manager = ConnectionManager::with_connection_factory(factory);
            let key = ConnectionKey::new("router1", "ssh");
            let params = ResolvedConnectionParams {
                hostname: "10.0.0.1".to_string(),
                port: Some(22),
                username: Some("admin".to_string()),
                password: Some("secret".to_string()),
                platform: Some("ios".to_string()),
                extras: None,
            };

            let connection = run_async(connection_manager.open_connection(&key, &params))
                .expect("open should succeed")
                .expect("connection should be created");

            {
                let guard = connection.blocking_lock();
                assert!(guard.is_alive());
            }

            connection_manager.close_connection(&key);
            let counters = connection_manager
                .connection_counters_for("ssh")
                .expect("counters should exist");
            assert_eq!(counters.create_calls, 1);
            assert_eq!(counters.open_calls, 1);
            assert_eq!(counters.close_calls, 1);
        });
    }

    #[test]
    fn register_connection_plugin_supports_async_methods() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.connection_plugins",
                "AsyncConnectionPlugin",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");

            manager
                .register_plugin(plugin)
                .expect("connection plugin should register");

            let inner = Arc::new(
                manager
                    .take_inner()
                    .expect("plugin manager should be consumable"),
            );
            let factory = build_connection_factory(Arc::clone(&inner));
            let connection_manager = ConnectionManager::with_connection_factory(factory);
            let key = ConnectionKey::new("router1", "async_ssh");
            let params = ResolvedConnectionParams {
                hostname: "10.0.0.1".to_string(),
                port: Some(22),
                username: Some("admin".to_string()),
                password: Some("secret".to_string()),
                platform: Some("ios".to_string()),
                extras: None,
            };

            let (connection_manager, key, connection) = run_python_async(py, async move {
                let connection = connection_manager
                    .open_connection(&key, &params)
                    .await
                    .expect("open should succeed")
                    .expect("connection should be created");
                (connection_manager, key, connection)
            });

            let (connection, output) = run_python_async(py, async move {
                let output = {
                    let mut guard = connection.lock().await;
                    guard.execute_command("show version").await
                };
                (connection, output)
            });
            let output = output.expect("execute_command should succeed");
            assert_eq!(output, "10.0.0.1:show version");

            {
                let guard = connection.blocking_lock();
                assert!(guard.is_alive());
            }

            connection_manager.close_connection(&key);
            let counters = connection_manager
                .connection_counters_for("async_ssh")
                .expect("counters should exist");
            assert_eq!(counters.create_calls, 1);
            assert_eq!(counters.open_calls, 1);
            assert_eq!(counters.close_calls, 1);
        });
    }

    #[test]
    fn load_python_plugins_from_pyproject_registers_inventory_plugins() {
        init_python();
        Python::attach(|py| {
            let temp_dir = temp_test_dir("inventory-pyproject");
            let module_path = temp_dir.join("inventory_plugins.py");
            fs::write(
                &module_path,
                r#"
from tests.fixtures.inventory_plugins import StaticInventoryPlugin as BaseInventoryPlugin

class StaticInventoryPlugin(BaseInventoryPlugin):
    pass
"#,
            )
            .expect("inventory plugin fixture should be written");
            let pyproject_path = temp_dir.join("pyproject.toml");
            fs::write(
                &pyproject_path,
                r#"
[tool.genja.plugins.inventory]
python_inventory = "inventory_plugins:StaticInventoryPlugin"
"#,
            )
            .expect("pyproject should be written");

            let sys = PyModule::import(py, "sys").expect("sys module should import");
            let path = sys.getattr("path").expect("sys.path should exist");
            path.call_method1("insert", (0, temp_dir.display().to_string()))
                .expect("tempdir should be added to sys.path");

            let manager = PyPluginManager::new();
            manager
                .load_python_plugins_from_pyproject(Some(pyproject_path.to_str().unwrap()))
                .expect("inventory plugin should load from pyproject");

            path.call_method1("remove", (temp_dir.display().to_string(),))
                .expect("tempdir should be removed from sys.path");
            let modules = sys.getattr("modules").expect("sys.modules should exist");
            modules.del_item("inventory_plugins").unwrap_or(());
            fs::remove_dir_all(&temp_dir).unwrap_or(());

            let names = manager
                .plugin_names()
                .expect("plugin names should be available");
            assert!(names.iter().any(|name| name == "python_inventory"));
        });
    }

    #[test]
    fn load_python_plugins_from_pyproject_registers_runner_plugins() {
        init_python();
        Python::attach(|py| {
            let temp_dir = temp_test_dir("runner-pyproject");
            let module_path = temp_dir.join("runner_plugins.py");
            fs::write(
                &module_path,
                r#"
from tests.fixtures.runner_plugins import FirstHostOnlyRunnerPlugin as BaseRunnerPlugin

class FirstHostOnlyRunnerPlugin(BaseRunnerPlugin):
    pass
"#,
            )
            .expect("runner plugin fixture should be written");
            let pyproject_path = temp_dir.join("pyproject.toml");
            fs::write(
                &pyproject_path,
                r#"
[tool.genja.plugins.runner]
python_runner = "runner_plugins:FirstHostOnlyRunnerPlugin"
"#,
            )
            .expect("pyproject should be written");

            let sys = PyModule::import(py, "sys").expect("sys module should import");
            let path = sys.getattr("path").expect("sys.path should exist");
            path.call_method1("insert", (0, temp_dir.display().to_string()))
                .expect("tempdir should be added to sys.path");

            let manager = PyPluginManager::new();
            manager
                .load_python_plugins_from_pyproject(Some(pyproject_path.to_str().unwrap()))
                .expect("runner plugin should load from pyproject");

            path.call_method1("remove", (temp_dir.display().to_string(),))
                .expect("tempdir should be removed from sys.path");
            let modules = sys.getattr("modules").expect("sys.modules should exist");
            modules.del_item("runner_plugins").unwrap_or(());
            fs::remove_dir_all(&temp_dir).unwrap_or(());

            let names = manager
                .plugin_names()
                .expect("plugin names should be available");
            assert!(names.iter().any(|name| name == "python_runner"));
        });
    }

    #[test]
    fn load_python_plugins_from_pyproject_registers_transform_plugins() {
        init_python();
        Python::attach(|py| {
            let temp_dir = temp_test_dir("transform-pyproject");
            let module_path = temp_dir.join("transform_plugins.py");
            fs::write(
                &module_path,
                r#"
from tests.fixtures.transform_plugins import HostnameSuffixTransformPlugin as BaseTransformPlugin

class HostnameSuffixTransformPlugin(BaseTransformPlugin):
    pass
"#,
            )
            .expect("transform plugin fixture should be written");
            let pyproject_path = temp_dir.join("pyproject.toml");
            fs::write(
                &pyproject_path,
                r#"
[tool.genja.plugins.transform]
python_transform = "transform_plugins:HostnameSuffixTransformPlugin"
"#,
            )
            .expect("pyproject should be written");

            let sys = PyModule::import(py, "sys").expect("sys module should import");
            let path = sys.getattr("path").expect("sys.path should exist");
            path.call_method1("insert", (0, temp_dir.display().to_string()))
                .expect("tempdir should be added to sys.path");

            let manager = PyPluginManager::new();
            manager
                .load_python_plugins_from_pyproject(Some(pyproject_path.to_str().unwrap()))
                .expect("transform plugin should load from pyproject");

            path.call_method1("remove", (temp_dir.display().to_string(),))
                .expect("tempdir should be removed from sys.path");
            let modules = sys.getattr("modules").expect("sys.modules should exist");
            modules.del_item("transform_plugins").unwrap_or(());
            fs::remove_dir_all(&temp_dir).unwrap_or(());

            let names = manager
                .plugin_names()
                .expect("plugin names should be available");
            assert!(names.iter().any(|name| name == "python_transform"));
        });
    }

    #[test]
    fn load_python_plugins_from_pyproject_rejects_name_mismatch() {
        init_python();
        Python::attach(|py| {
            let temp_dir = temp_test_dir("name-mismatch-pyproject");
            let module_path = temp_dir.join("processor_plugins.py");
            fs::write(
                &module_path,
                r#"
from tests.fixtures.processor_plugins import MinimalAuditProcessor
"#,
            )
            .expect("processor plugin fixture should be written");
            let pyproject_path = temp_dir.join("pyproject.toml");
            fs::write(
                &pyproject_path,
                r#"
[tool.genja.plugins.processor]
wrong_name = "processor_plugins:MinimalAuditProcessor"
"#,
            )
            .expect("pyproject should be written");

            let sys = PyModule::import(py, "sys").expect("sys module should import");
            let path = sys.getattr("path").expect("sys.path should exist");
            path.call_method1("insert", (0, temp_dir.display().to_string()))
                .expect("tempdir should be added to sys.path");

            let manager = PyPluginManager::new();
            let err = manager
                .load_python_plugins_from_pyproject(Some(pyproject_path.to_str().unwrap()))
                .expect_err("name mismatch should fail");

            path.call_method1("remove", (temp_dir.display().to_string(),))
                .expect("tempdir should be removed from sys.path");
            let modules = sys.getattr("modules").expect("sys.modules should exist");
            modules.del_item("processor_plugins").unwrap_or(());
            fs::remove_dir_all(&temp_dir).unwrap_or(());

            assert!(err.to_string().contains("plugin name mismatch"));
        });
    }

    #[test]
    fn load_python_plugins_from_pyproject_rejects_non_string_entry() {
        init_python();
        Python::attach(|_py| {
            let temp_dir = temp_test_dir("non-string-pyproject");
            let pyproject_path = temp_dir.join("pyproject.toml");
            fs::write(
                &pyproject_path,
                r#"
[tool.genja.plugins.processor]
audit = { path = "processor_plugins:MinimalAuditProcessor" }
"#,
            )
            .expect("pyproject should be written");

            let manager = PyPluginManager::new();
            let err = manager
                .load_python_plugins_from_pyproject(Some(pyproject_path.to_str().unwrap()))
                .expect_err("non-string entry should fail");
            fs::remove_dir_all(&temp_dir).unwrap_or(());

            assert!(err.to_string().contains("must be a string import path"));
        });
    }

    #[test]
    fn transform_plugin_falls_back_for_missing_group_and_defaults_methods() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class = import_fixture_attr(
                py,
                "tests.fixtures.transform_plugins",
                "HostOnlyTransformPlugin",
            )
            .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");
            manager
                .register_plugin(plugin)
                .expect("transform plugin should register");

            let inner = manager
                .take_inner()
                .expect("plugin manager should be consumable");
            let transform = inner
                .get_transform_function_plugin("python_host_only_transform")
                .expect("transform plugin should exist")
                .transform_function();
            let options = TransformFunctionOptions::new(json!({"suffix": "-lab"}));

            let host = Host::builder().hostname("10.0.0.1").platform("ios").build();
            let group = Group::builder().platform("nxos").build();
            let defaults = Defaults::builder().port(22).build();

            let transformed_host = transform.transform_host(&host, Some(&options));
            let transformed_group = transform.transform_group(&group, Some(&options));
            let transformed_defaults = transform.transform_defaults(&defaults, Some(&options));

            assert_eq!(transformed_host.hostname(), Some("10.0.0.1-lab"));
            assert_eq!(transformed_group.platform(), group.platform());
            assert_eq!(transformed_group.port(), group.port());
            assert_eq!(transformed_defaults.port(), defaults.port());
            assert_eq!(transformed_defaults.platform(), defaults.platform());
        });
    }

    #[test]
    fn runner_plugin_run_tasks_uses_python_run_tasks_when_available() {
        init_python();
        Python::attach(|py| {
            let manager = PyPluginManager::new();
            let plugin_class =
                import_fixture_attr(py, "tests.fixtures.runner_plugins", "BatchRunnerPlugin")
                    .expect("fixture plugin class should import");
            let plugin = plugin_class.call0().expect("plugin instance should build");
            manager
                .register_plugin(plugin)
                .expect("runner plugin should register");

            let inner = manager
                .take_inner()
                .expect("plugin manager should be consumable");
            let runner = inner
                .get_runner_plugin("python_batch_runner")
                .expect("runner plugin should exist");

            let mut hosts = genja_core::inventory::Hosts::new();
            hosts.add_host(
                "router1",
                Host::builder().hostname("10.0.0.1").platform("ios").build(),
            );
            hosts.add_host(
                "router2",
                Host::builder()
                    .hostname("10.0.0.2")
                    .platform("nxos")
                    .build(),
            );

            let mut tasks = Tasks::new();
            tasks.add_task(TestTask {
                name: "task_a".to_string(),
            });
            tasks.add_task(TestTask {
                name: "task_b".to_string(),
            });

            let results = run_async(
                runner.run_tasks(
                    &tasks,
                    &hosts,
                    None,
                    &RunnerConfig::builder()
                        .plugin("python_batch_runner")
                        .build(),
                    2,
                ),
            )
            .expect("run_tasks should succeed");

            assert_eq!(results.len(), 2);
            assert_eq!(results[0].task_name(), "task_a");
            assert_eq!(results[1].task_name(), "task_b");
            assert_eq!(results[0].passed_hosts().len(), 2);
            assert_eq!(results[1].passed_hosts().len(), 2);
        });
    }

    #[test]
    fn python_task_processor_context_model_exposes_expected_fields() {
        init_python();
        Python::attach(|py| {
            let context = TaskProcessorContext::new("backup", Some("parent"), 1, Some("router1"));
            let payload = build_python_processor_context(py, &context)
                .expect("processor context should be built");
            let data: Value = payload
                .bind(py)
                .call_method0("to_dict")
                .and_then(|value| {
                    let json = PyModule::import(py, "json")?;
                    let dumped: String = json.call_method1("dumps", (value,))?.extract()?;
                    Ok(serde_json::from_str(&dumped).expect("context json should parse"))
                })
                .expect("context should serialize");

            assert_eq!(data["task_name"], "backup");
            assert_eq!(data["parent_task_name"], "parent");
            assert_eq!(data["depth"], 1);
            assert_eq!(data["hostname"], "router1");
        });
    }
}
