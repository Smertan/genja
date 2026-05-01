use genja::plugins::built_in_plugin_manager;
use genja_plugin_manager::PluginManager;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;
use std::sync::Mutex;

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
            PyValueError::new_err(format!("failed to load plugins from directory {path}: {err}"))
        })?;
        *guard = Some(manager);
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
                let plugin_count = guard.as_ref().map(|m| m.get_all_plugin_names().len()).unwrap_or(0);
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
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyPluginManager>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    fn init_python() {
        static INIT: Once = Once::new();
        INIT.call_once(pyo3::prepare_freethreaded_python);
    }

    #[test]
    fn py_plugin_manager_new_includes_built_in_plugins() {
        let manager = PyPluginManager::new();
        let names = manager.plugin_names().expect("built-in plugins should be available");

        assert!(names.iter().any(|name| name == "FileInventoryPlugin"));
        assert!(names.iter().any(|name| name == "serial"));
        assert!(names.iter().any(|name| name == "threaded"));
    }

    #[test]
    fn take_inner_consumes_plugin_manager() {
        let manager = PyPluginManager::new();

        let inner = manager.take_inner().expect("plugin manager should be consumable");
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
            let module =
                PyModule::new(py, "test_plugin_manager_module").expect("test module should be created");

            register(&module).expect("plugin manager class should register");

            assert!(module.getattr("PluginManager").is_ok());
        });
    }
}
