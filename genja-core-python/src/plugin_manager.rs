use async_trait::async_trait;
use genja::plugins::built_in_plugin_manager;
use genja_core::inventory::{Connection, ConnectionKey, Inventory, ResolvedConnectionParams};
use genja_core::settings::Settings;
use genja_core::task::{HostTaskResult, TaskProcessor, TaskProcessorContext, TaskResults};
use genja_core::InventoryLoadError;
use genja_plugin_manager::connection_factory::PluginConnectionAdapter;
use genja_plugin_manager::plugin_types::{
    Plugin, PluginConnection, PluginInventory, PluginProcessor, Plugins,
};
use genja_plugin_manager::PluginManager;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::runtime::python_inventory_to_rust_inventory;
use crate::settings::PySettings;
use crate::task::{
    python_result_to_host_task_result, python_result_to_task_results, PyHostTaskResult,
    PyTaskResults,
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
    /// logic. The plugin must implement the required plugin interface methods
    /// (`name()` and `group()`) and belong to a supported plugin group
    /// (currently "ProcessorPlugin", "ConnectionPlugin", or "InventoryPlugin").
    ///
    /// # Parameters
    ///
    /// * `plugin` - A bound reference to a Python object implementing the plugin
    ///   interface. The object will be unbound and stored internally for later use.
    ///   The plugin must define callable `name()` and `group()` methods that return
    ///   non-empty strings identifying the plugin.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the plugin was successfully registered, or a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    /// - The plugin is missing required `name()` or `group()` methods
    /// - The plugin's `name()` or `group()` methods are not callable
    /// - The plugin's `name()` or `group()` returns an empty string
    /// - The plugin's group is not a supported type ("ProcessorPlugin", "ConnectionPlugin", or "InventoryPlugin")
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
    /// supports "processor", "connection", and "inventory" plugin types. Each plugin entry
    /// must specify an import path in the format `module:attribute`, and the plugin's
    /// declared name (from its `name()` method) must match the key used in the manifest.
    ///
    /// The expected structure in `pyproject.toml` is:
    /// ```toml
    /// [tool.genja.plugins.processor]
    /// my_processor = "my_module:MyProcessorClass"
    ///
    /// [tool.genja.plugins.connection]
    /// my_connection = "my_module:MyConnectionClass"
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
    /// - A plugin's declared `name()` does not match its manifest key
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

        for section_name in ["processor", "connection", "inventory"] {
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
                let plugin = Python::with_gil(|py| import_python_plugin(py, import_path))?;
                let declared_name = Python::with_gil(|py| {
                    extract_plugin_identity_value(
                        plugin.bind(py),
                        "name",
                        &format!("{section_name} plugin name must not be empty"),
                        "plugin",
                    )
                })?;
                if declared_name != *name {
                    return Err(PyValueError::new_err(format!(
                        "{section_name} plugin name mismatch in {}: manifest key '{name}' does not match plugin.name() value '{declared_name}'",
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
    /// longer be available for use. The method returns the group name of the
    /// deregistered plugin if it was found, or `None` if no plugin with the given
    /// name was registered.
    ///
    /// # Parameters
    ///
    /// * `name` - A string slice representing the unique name of the plugin to
    ///   deregister. This should match the name returned by the plugin's `name()`
    ///   method when it was registered.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Some(String))` containing the group name of the deregistered
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
    /// The names correspond to the values returned by each plugin's `name()` method.
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
    /// name (from its `name()` method) and its group (from its `group()` method).
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
    /// plugin interface methods (`name()` and `group()`) and belong to a supported
    /// plugin group.
    ///
    /// # Parameters
    ///
    /// * `plugin` - A Python object implementing the plugin interface. The object
    ///   must define callable `name()` and `group()` methods that return non-empty
    ///   strings identifying the plugin. The plugin's group must be either
    ///   "ProcessorPlugin", "ConnectionPlugin", or "InventoryPlugin".
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the plugin was successfully registered, or a `PyErr` if:
    /// - The plugin manager has already been consumed
    /// - The plugin manager lock is poisoned
    /// - The plugin is missing required `name()` or `group()` methods
    /// - The plugin's `name()` or `group()` methods are not callable
    /// - The plugin's `name()` or `group()` returns an empty string
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
/// This function extracts the plugin's identity (name and group) by calling its
/// `name()` and `group()` methods, then wraps the plugin in the appropriate Rust
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
///   must define callable `name()` and `group()` methods that return non-empty
///   strings. The plugin's group must be either "ProcessorPlugin" or
///   "ConnectionPlugin". The plugin is wrapped in an `Arc` for shared ownership
///   across the plugin system.
///
/// # Returns
///
/// Returns `Ok(())` if the plugin was successfully registered, or a `PyErr` if:
/// - The plugin is missing required `name()` or `group()` methods
/// - The plugin's `name()` or `group()` methods are not callable
/// - The plugin's `name()` or `group()` returns an empty string
/// - The plugin's group is not "ProcessorPlugin", "ConnectionPlugin", or "InventoryPlugin"
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
    let (declared_name, declared_group) = Python::with_gil(|py| {
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
        "ProcessorPlugin" => {
            manager.register_plugin(Plugins::Processor(Box::new(PyProcessorPlugin {
                name: declared_name,
                group: declared_group,
                processor: Arc::new(plugin),
            })));
        }
        other => {
            return Err(PyValueError::new_err(format!(
                "unsupported python plugin group '{other}'; only 'ProcessorPlugin', 'ConnectionPlugin', and 'InventoryPlugin' are currently supported"
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
/// * `name` - The unique identifier for this connection plugin, matching the value
///   returned by the Python plugin's `name()` method.
/// * `group` - The group identifier for this plugin, matching the value returned by
///   the Python plugin's `group()` method. For connection plugins, this is typically
///   "ConnectionPlugin".
/// * `plugin` - An `Arc`-wrapped Python object implementing the connection plugin
///   interface. The `Arc` allows the plugin to be shared across multiple connection
///   instances created by this factory.
struct PyConnectionPlugin {
    name: String,
    group: String,
    plugin: Arc<Py<PyAny>>,
}

#[pyclass]
#[derive(Clone)]
struct PyLoadedPluginRegistry {
    names: Vec<String>,
    names_and_groups: Vec<(String, String)>,
}

#[pymethods]
impl PyLoadedPluginRegistry {
    fn plugin_names(&self) -> Vec<String> {
        self.names.clone()
    }

    fn plugin_names_and_groups(&self) -> Vec<(String, String)> {
        self.names_and_groups.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "LoadedPluginRegistry(plugin_count={})",
            self.names_and_groups.len()
        )
    }
}

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
    fn load(
        &self,
        settings: &Settings,
        plugins: &PluginManager,
    ) -> Result<Inventory, InventoryLoadError> {
        Python::with_gil(|py| {
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

struct PyConnectionInstance {
    name: String,
    group: String,
    factory_plugin: Arc<Py<PyAny>>,
    key: ConnectionKey,
    connection: Option<Py<PyAny>>,
    create_error: Option<String>,
}

impl PyConnectionInstance {
    fn from_factory(
        factory_plugin: Arc<Py<PyAny>>,
        name: String,
        group: String,
        key: ConnectionKey,
    ) -> Self {
        let created = Python::with_gil(|py| {
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

pub(crate) fn python_connection_from_runtime_connection(
    connection: &dyn Connection,
) -> Option<Py<PyAny>> {
    let adapter = (connection as &dyn std::any::Any).downcast_ref::<PluginConnectionAdapter>()?;
    let py_connection = (adapter.inner_plugin_connection() as &dyn std::any::Any)
        .downcast_ref::<PyConnectionInstance>()?;
    Python::with_gil(|py| {
        py_connection
            .connection
            .as_ref()
            .map(|connection| connection.clone_ref(py))
    })
}

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
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        Box::new(Self::from_factory(
            Arc::clone(&self.factory_plugin),
            self.name.clone(),
            self.group.clone(),
            key.clone(),
        ))
    }

    async fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String> {
        if let Some(error) = self.create_error.as_ref() {
            return Err(format!(
                "failed to create python connection plugin instance: {error}"
            ));
        }
        let Some(connection) = self.connection.as_ref() else {
            return Err("python connection plugin instance is missing a connection".to_string());
        };

        Python::with_gil(|py| {
            let connection = connection.bind(py);
            let params_payload = build_python_resolved_connection_params(py, params)
                .map_err(|err| err.to_string())?;
            let result = connection
                .call_method1("open", (params_payload,))
                .map_err(|err| err.to_string())?;
            resolve_python_maybe_awaitable(py, result).map_err(|err| err.to_string())?;
            Ok(())
        })
    }

    async fn execute_command(&mut self, command: &str) -> Result<String, String> {
        if let Some(error) = self.create_error.as_ref() {
            return Err(format!(
                "failed to create python connection plugin instance: {error}"
            ));
        }
        let Some(connection) = self.connection.as_ref() else {
            return Err("python connection plugin instance is missing a connection".to_string());
        };

        Python::with_gil(|py| {
            let connection = connection.bind(py);
            let result = connection
                .call_method1("execute_command", (command,))
                .map_err(|err| err.to_string())?;
            let resolved =
                resolve_python_maybe_awaitable(py, result).map_err(|err| err.to_string())?;
            resolved
                .bind(py)
                .extract::<String>()
                .map_err(|err| err.to_string())
        })
    }

    fn close(&mut self) -> ConnectionKey {
        let Some(connection) = self.connection.as_ref() else {
            return self.key.clone();
        };

        Python::with_gil(|py| {
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

    fn is_alive(&self) -> bool {
        if self.create_error.is_some() {
            return false;
        }
        let Some(connection) = self.connection.as_ref() else {
            return false;
        };

        Python::with_gil(|py| {
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

fn extract_plugin_identity_value(
    plugin: &Bound<'_, PyAny>,
    method_name: &str,
    empty_message: &str,
    plugin_kind: &str,
) -> PyResult<String> {
    let method = plugin.getattr(method_name).map_err(|_| {
        PyValueError::new_err(format!(
            "{plugin_kind} must define a callable '{method_name}()' method"
        ))
    })?;
    if !method.is_callable() {
        return Err(PyValueError::new_err(format!(
            "{plugin_kind} attribute '{method_name}' must be callable"
        )));
    }
    let value: String = method.call0()?.extract()?;
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

struct PyTaskProcessor {
    processor: Arc<Py<PyAny>>,
}

impl TaskProcessor for PyTaskProcessor {
    fn on_task_start(
        &self,
        context: &TaskProcessorContext,
        results: &mut TaskResults,
    ) -> Result<(), genja_core::GenjaError> {
        self.call_task_results_hook("on_task_start", context, results)
    }

    fn on_task_finish(
        &self,
        context: &TaskProcessorContext,
        results: &mut TaskResults,
    ) -> Result<(), genja_core::GenjaError> {
        self.call_task_results_hook("on_task_finish", context, results)
    }

    fn on_instance_start(
        &self,
        context: &TaskProcessorContext,
    ) -> Result<(), genja_core::GenjaError> {
        Python::with_gil(|py| {
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

    fn on_instance_finish(
        &self,
        context: &TaskProcessorContext,
        result: &mut HostTaskResult,
    ) -> Result<(), genja_core::GenjaError> {
        Python::with_gil(|py| {
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
    fn call_task_results_hook(
        &self,
        method_name: &str,
        context: &TaskProcessorContext,
        results: &mut TaskResults,
    ) -> Result<(), genja_core::GenjaError> {
        Python::with_gil(|py| {
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
    build_python_model(py, "genja_core.processor", "TaskProcessorContext", payload)
}

fn build_python_connection_key<'py>(py: Python<'py>, key: &ConnectionKey) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    payload.set_item("hostname", &key.hostname)?;
    payload.set_item("plugin_name", &key.plugin_name)?;
    build_python_model(py, "genja_core.connection", "ConnectionKey", payload)
}

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
    build_python_model(
        py,
        "genja_core.connection",
        "ResolvedConnectionParams",
        payload,
    )
}

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

fn python_processor_error(err: PyErr) -> genja_core::GenjaError {
    genja_core::GenjaError::Message(err.to_string())
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyPluginManager>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use genja_core::inventory::ConnectionManager;
    use genja_plugin_manager::connection_factory::build_connection_factory;
    use serde_json::Value;
    use std::env;
    use std::path::PathBuf;
    use std::sync::Arc;
    use std::sync::Once;
    use std::time::{SystemTime, UNIX_EPOCH};
    use tokio::runtime::Builder;

    fn run_async<F: std::future::Future>(future: F) -> F::Output {
        Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("test runtime should build")
            .block_on(future)
    }

    fn init_python() {
        static INIT: Once = Once::new();
        INIT.call_once(|| {
            pyo3::prepare_freethreaded_python();
            Python::with_gil(|py| {
                let sys = PyModule::import(py, "sys").expect("sys module should import");
                let path = sys.getattr("path").expect("sys.path should exist");
                let python_source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("python");
                path.call_method1("insert", (0, python_source.display().to_string()))
                    .expect("python source path should be inserted");
                let modules = sys.getattr("modules").expect("sys.modules should exist");
                let genja_core = PyModule::from_code(
                    py,
                    pyo3::ffi::c_str!("__path__ = []\n"),
                    pyo3::ffi::c_str!("genja_core/__init__.py"),
                    pyo3::ffi::c_str!("genja_core"),
                )
                .expect("genja_core stub should build");
                let processor = PyModule::from_code(
                    py,
                    pyo3::ffi::c_str!(
                        "class TaskProcessorContext:\n    def __init__(self, **kwargs):\n        self.__dict__.update(kwargs)\n    def to_dict(self):\n        return dict(self.__dict__)\n"
                    ),
                    pyo3::ffi::c_str!("genja_core/processor.py"),
                    pyo3::ffi::c_str!("genja_core.processor"),
                )
                .expect("processor stub should build");
                genja_core
                    .add("processor", &processor)
                    .expect("processor module should attach to package");
                modules
                    .set_item("genja_core", &genja_core)
                    .expect("genja_core stub should register");
                modules
                    .set_item("genja_core.processor", &processor)
                    .expect("processor stub should register");
                let connection = PyModule::from_code(
                    py,
                    pyo3::ffi::c_str!(
                        "class ConnectionKey:\n    def __init__(self, **kwargs):\n        self.__dict__.update(kwargs)\n    def to_dict(self):\n        return dict(self.__dict__)\n\nclass ResolvedConnectionParams:\n    def __init__(self, **kwargs):\n        self.__dict__.update(kwargs)\n    def to_dict(self):\n        return dict(self.__dict__)\n"
                    ),
                    pyo3::ffi::c_str!("genja_core/connection.py"),
                    pyo3::ffi::c_str!("genja_core.connection"),
                )
                .expect("connection stub should build");
                genja_core
                    .add("connection", &connection)
                    .expect("connection module should attach to package");
                modules
                    .set_item("genja_core.connection", &connection)
                    .expect("connection stub should register");
            });
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
            .err()
            .expect("consumed manager should reject access");
        assert!(err.to_string().contains("already been consumed"));
    }

    #[test]
    fn register_adds_plugin_manager_class_to_module() {
        init_python();
        Python::with_gil(|py| {
            let module = PyModule::new(py, "test_plugin_manager_module")
                .expect("test module should be created");

            register(&module).expect("plugin manager class should register");

            assert!(module.getattr("PluginManager").is_ok());
        });
    }

    #[test]
    fn register_plugin_adds_processor_plugin() {
        init_python();
        Python::with_gil(|py| {
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
            assert!(groups
                .iter()
                .any(|(name, group)| name == "audit" && group == "Processor"));
        });
    }

    #[test]
    fn register_plugin_adds_inventory_plugin() {
        init_python();
        Python::with_gil(|py| {
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
            assert!(groups
                .iter()
                .any(|(name, group)| name == "python_inventory" && group == "Inventory"));
        });
    }

    #[test]
    fn register_plugin_requires_name_and_group_methods() {
        init_python();
        Python::with_gil(|py| {
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
            assert!(err
                .to_string()
                .contains("plugin must define a callable 'name()' method"));
        });
    }

    #[test]
    fn register_plugin_rejects_unsupported_group() {
        init_python();
        Python::with_gil(|py| {
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
            assert!(err
                .to_string()
                .contains("unsupported python plugin group 'RunnerPlugin'"));
        });
    }

    #[test]
    fn register_connection_plugin_supports_factory_open_and_close() {
        init_python();
        Python::with_gil(|py| {
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
    fn register_connection_plugin_supports_async_factory_and_methods() {
        init_python();
        Python::with_gil(|py| {
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

            let connection = run_async(connection_manager.open_connection(&key, &params))
                .expect("open should succeed")
                .expect("connection should be created");

            let output = run_async(async {
                let mut guard = connection.lock().await;
                guard.execute_command("show version").await
            })
            .expect("execute_command should succeed");
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
        Python::with_gil(|py| {
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
    fn python_task_processor_context_model_exposes_expected_fields() {
        init_python();
        Python::with_gil(|py| {
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
