use ::genja_core::settings::{
    CoreConfig, InventoryConfig, LoggingConfig, OptionsConfig, RunnerConfig, SSHConfig,
};
use ::genja_core::Settings;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyModule;

#[pyclass(name = "OptionsConfig")]
#[derive(Clone)]
pub struct PyOptionsConfig {
    pub(crate) inner: OptionsConfig,
}

#[pymethods]
impl PyOptionsConfig {
    #[getter]
    pub(crate) fn hosts_file(&self) -> Option<String> {
        self.inner.hosts_file().map(str::to_owned)
    }

    #[getter]
    pub(crate) fn groups_file(&self) -> Option<String> {
        self.inner.groups_file().map(str::to_owned)
    }

    #[getter]
    pub(crate) fn defaults_file(&self) -> Option<String> {
        self.inner.defaults_file().map(str::to_owned)
    }

    fn __repr__(&self) -> String {
        format!(
            "OptionsConfig(hosts_file={:?}, groups_file={:?}, defaults_file={:?})",
            self.hosts_file(),
            self.groups_file(),
            self.defaults_file()
        )
    }
}

#[pyclass(name = "CoreConfig")]
#[derive(Clone)]
pub struct PyCoreConfig {
    pub(crate) inner: CoreConfig,
}

#[pymethods]
impl PyCoreConfig {
    #[getter]
    pub(crate) fn raise_on_error(&self) -> bool {
        self.inner.raise_on_error()
    }

    fn __repr__(&self) -> String {
        format!("CoreConfig(raise_on_error={})", self.raise_on_error())
    }
}

#[pyclass(name = "InventoryConfig")]
#[derive(Clone)]
pub struct PyInventoryConfig {
    pub(crate) inner: InventoryConfig,
}

#[pymethods]
impl PyInventoryConfig {
    #[getter]
    pub(crate) fn plugin(&self) -> String {
        self.inner.plugin().to_owned()
    }

    #[getter]
    pub(crate) fn options(&self) -> PyOptionsConfig {
        PyOptionsConfig {
            inner: self.inner.options().clone(),
        }
    }

    #[getter]
    pub(crate) fn transform_function(&self) -> Option<String> {
        self.inner.transform_function().map(str::to_owned)
    }

    fn __repr__(&self) -> String {
        format!(
            "InventoryConfig(plugin={:?}, options={}, transform_function={:?})",
            self.plugin(),
            self.options().__repr__(),
            self.transform_function()
        )
    }
}

#[pyclass(name = "SSHConfig")]
#[derive(Clone)]
pub struct PySSHConfig {
    pub(crate) inner: SSHConfig,
}

#[pymethods]
impl PySSHConfig {
    #[getter]
    pub(crate) fn config_file(&self) -> Option<String> {
        self.inner.config_file().map(str::to_owned)
    }

    fn __repr__(&self) -> String {
        format!("SSHConfig(config_file={:?})", self.config_file())
    }
}

#[pyclass(name = "RunnerConfig")]
#[derive(Clone)]
pub struct PyRunnerConfig {
    pub(crate) inner: RunnerConfig,
}

#[pymethods]
impl PyRunnerConfig {
    #[getter]
    pub(crate) fn plugin(&self) -> String {
        self.inner.plugin().to_owned()
    }

    #[getter]
    pub(crate) fn worker_count(&self) -> Option<usize> {
        self.inner.worker_count()
    }

    #[getter]
    pub(crate) fn max_task_depth(&self) -> usize {
        self.inner.max_task_depth()
    }

    #[getter]
    pub(crate) fn max_connection_attempts(&self) -> usize {
        self.inner.max_connection_attempts()
    }

    fn __repr__(&self) -> String {
        format!(
            "RunnerConfig(plugin={:?}, worker_count={:?}, max_task_depth={}, max_connection_attempts={})",
            self.plugin(),
            self.worker_count(),
            self.max_task_depth(),
            self.max_connection_attempts()
        )
    }
}

#[pyclass(name = "LoggingConfig")]
#[derive(Clone)]
pub struct PyLoggingConfig {
    pub(crate) inner: LoggingConfig,
}

#[pymethods]
impl PyLoggingConfig {
    #[getter]
    pub(crate) fn enabled(&self) -> bool {
        self.inner.enabled()
    }

    #[getter]
    pub(crate) fn level(&self) -> String {
        self.inner.level().to_owned()
    }

    #[getter]
    pub(crate) fn log_file(&self) -> String {
        self.inner.log_file().to_owned()
    }

    #[getter]
    pub(crate) fn to_console(&self) -> bool {
        self.inner.to_console()
    }

    #[getter]
    pub(crate) fn file_size(&self) -> u64 {
        self.inner.file_size()
    }

    #[getter]
    pub(crate) fn max_file_count(&self) -> usize {
        self.inner.max_file_count()
    }

    fn __repr__(&self) -> String {
        format!(
            "LoggingConfig(enabled={}, level={:?}, log_file={:?}, to_console={}, file_size={}, max_file_count={})",
            self.enabled(),
            self.level(),
            self.log_file(),
            self.to_console(),
            self.file_size(),
            self.max_file_count()
        )
    }
}

#[pyclass(name = "Settings")]
#[derive(Clone)]
pub struct PySettings {
    pub(crate) inner: Settings,
}

#[pymethods]
impl PySettings {
    #[new]
    fn new() -> Self {
        Self {
            inner: Settings::default(),
        }
    }

    #[staticmethod]
    fn from_file(path: &str) -> PyResult<Self> {
        let inner = Settings::from_file(path).map_err(|err| {
            PyValueError::new_err(format!("failed to load settings from {path}: {err}"))
        })?;
        Ok(Self { inner })
    }

    #[getter]
    pub(crate) fn core(&self) -> PyCoreConfig {
        PyCoreConfig {
            inner: self.inner.core().clone(),
        }
    }

    #[getter]
    pub(crate) fn inventory(&self) -> PyInventoryConfig {
        PyInventoryConfig {
            inner: self.inner.inventory().clone(),
        }
    }

    #[getter]
    pub(crate) fn ssh(&self) -> PySSHConfig {
        PySSHConfig {
            inner: self.inner.ssh().clone(),
        }
    }

    #[getter]
    pub(crate) fn runner(&self) -> PyRunnerConfig {
        PyRunnerConfig {
            inner: self.inner.runner().clone(),
        }
    }

    #[getter]
    pub(crate) fn logging(&self) -> PyLoggingConfig {
        PyLoggingConfig {
            inner: self.inner.logging().clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Settings(core={}, inventory={}, ssh={}, runner={}, logging={})",
            self.core().__repr__(),
            self.inventory().__repr__(),
            self.ssh().__repr__(),
            self.runner().__repr__(),
            self.logging().__repr__()
        )
    }
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySettings>()?;
    module.add_class::<PyCoreConfig>()?;
    module.add_class::<PyOptionsConfig>()?;
    module.add_class::<PyInventoryConfig>()?;
    module.add_class::<PySSHConfig>()?;
    module.add_class::<PyRunnerConfig>()?;
    module.add_class::<PyLoggingConfig>()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn settings_fixture_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("python")
            .join("tests")
            .join("fixtures")
            .join("settings.yaml")
    }

    #[test]
    fn py_settings_new_wraps_default_settings() {
        let settings = PySettings::new();

        assert!(!settings.core().raise_on_error());
        assert_eq!(settings.inventory().plugin(), "FileInventoryPlugin");
        assert_eq!(settings.runner().plugin(), "threaded");
        assert_eq!(settings.logging().level(), "info");
        assert!(settings.logging().enabled());
        assert!(settings.__repr__().contains("Settings("));
    }

    #[test]
    fn py_settings_from_file_exposes_loaded_values() {
        let settings = PySettings::from_file(settings_fixture_path().to_str().unwrap())
            .expect("settings fixture should load");

        assert!(!settings.core().raise_on_error());
        assert_eq!(settings.inventory().plugin(), "FileInventoryPlugin");
        assert_eq!(
            settings.inventory().options().hosts_file(),
            Some("./inventory/hosts.yaml".to_string())
        );
        assert_eq!(settings.ssh().config_file(), None);
        assert_eq!(settings.runner().plugin(), "threaded");
        assert_eq!(settings.runner().worker_count(), Some(10));
        assert_eq!(settings.logging().level(), "info");
        assert_eq!(settings.logging().max_file_count(), 10);
    }

    #[test]
    fn register_adds_settings_classes_to_module() {
        pyo3::prepare_freethreaded_python();
        Python::with_gil(|py| {
            let module =
                PyModule::new(py, "test_settings_module").expect("test module should be created");

            register(&module).expect("settings classes should register");

            assert!(module.getattr("Settings").is_ok());
            assert!(module.getattr("CoreConfig").is_ok());
            assert!(module.getattr("OptionsConfig").is_ok());
            assert!(module.getattr("InventoryConfig").is_ok());
            assert!(module.getattr("SSHConfig").is_ok());
            assert!(module.getattr("RunnerConfig").is_ok());
            assert!(module.getattr("LoggingConfig").is_ok());
        });
    }
}
