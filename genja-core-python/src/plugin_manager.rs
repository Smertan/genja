use genja::plugins::built_in_plugin_manager;
use genja_core::task::{HostTaskResult, TaskProcessor, TaskProcessorContext, TaskResults};
use genja_plugin_manager::plugin_types::{Plugin, PluginProcessor, Plugins};
use genja_plugin_manager::PluginManager;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::task::{
    python_result_to_host_task_result, python_result_to_task_results, PyHostTaskResult,
    PyTaskResults,
};

#[pyclass(name = "PluginManager")]
pub struct PyPluginManager {
    inner: Mutex<Option<PluginManager>>,
}

#[pymethods]
impl PyPluginManager {
    #[new]
    pub(crate) fn new() -> Self {
        Self {
            inner: Mutex::new(Some(built_in_plugin_manager())),
        }
    }

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

    fn register_plugin(&self, plugin: Bound<'_, PyAny>) -> PyResult<()> {
        self.register_python_plugin(plugin.unbind())
    }

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

        let Some(processors) = value
            .get("tool")
            .and_then(|tool| tool.get("genja"))
            .and_then(|genja| genja.get("plugins"))
            .and_then(|plugins| plugins.get("processor"))
            .and_then(toml::Value::as_table)
        else {
            return Ok(());
        };

        for (name, import_path) in processors {
            let import_path = import_path.as_str().ok_or_else(|| {
                PyValueError::new_err(format!(
                    "processor plugin entry '{name}' in {} must be a string import path",
                    manifest_path.display()
                ))
            })?;
            let processor = Python::with_gil(|py| import_python_plugin(py, import_path))?;
            let declared_name = Python::with_gil(|py| {
                extract_plugin_identity_value(
                    processor.bind(py),
                    "name",
                    "processor plugin name must not be empty",
                    "plugin",
                )
            })?;
            if declared_name != *name {
                return Err(PyValueError::new_err(format!(
                    "processor plugin name mismatch in {}: manifest key '{name}' does not match plugin.name() value '{declared_name}'",
                    manifest_path.display()
                )));
            }
            self.register_python_plugin(processor)?;
        }

        Ok(())
    }

    fn deregister_plugin(&self, name: &str) -> PyResult<Option<String>> {
        let mut guard = self.lock_inner()?;
        let manager = guard
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))?;
        Ok(manager.deregister_plugin(name))
    }

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

    fn plugin_names_and_groups(&self) -> PyResult<Vec<(String, String)>> {
        let guard = self.lock_inner()?;
        let manager = guard
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))?;
        Ok(manager.get_all_plugin_names_and_groups())
    }

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
    pub(crate) fn take_inner(&self) -> PyResult<PluginManager> {
        let mut guard = self.lock_inner()?;
        guard
            .take()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))
    }

    fn lock_inner(&self) -> PyResult<std::sync::MutexGuard<'_, Option<PluginManager>>> {
        self.inner
            .lock()
            .map_err(|_| PyValueError::new_err("plugin manager lock is poisoned"))
    }

    fn register_python_plugin(&self, plugin: Py<PyAny>) -> PyResult<()> {
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

        let mut guard = self.lock_inner()?;
        let manager = guard
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("plugin manager has already been consumed"))?;
        match declared_group.as_str() {
            "ProcessorPlugin" => {
                manager.register_plugin(Plugins::Processor(Box::new(PyProcessorPlugin {
                    name: declared_name,
                    group: declared_group,
                    processor: Arc::new(plugin),
                })));
            }
            other => {
                return Err(PyValueError::new_err(format!(
                    "unsupported python plugin group '{other}'; only 'ProcessorPlugin' is currently supported"
                )));
            }
        }
        Ok(())
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
    use serde_json::Value;
    use std::path::PathBuf;
    use std::sync::Once;

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
    fn register_processor_adds_processor_plugin() {
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
