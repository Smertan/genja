use ::genja_core::settings::{
    CoreConfig, InventoryConfig, LoggingConfig, OptionsConfig, RunnerConfig, SSHConfig,
};
use ::genja_core::Settings;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[pyclass(name = "OptionsConfig")]
#[derive(Clone)]
struct PyOptionsConfig {
    inner: OptionsConfig,
}

#[pymethods]
impl PyOptionsConfig {
    #[getter]
    fn hosts_file(&self) -> Option<String> {
        self.inner.hosts_file().map(str::to_owned)
    }

    #[getter]
    fn groups_file(&self) -> Option<String> {
        self.inner.groups_file().map(str::to_owned)
    }

    #[getter]
    fn defaults_file(&self) -> Option<String> {
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
struct PyCoreConfig {
    inner: CoreConfig,
}

#[pymethods]
impl PyCoreConfig {
    #[getter]
    fn raise_on_error(&self) -> bool {
        self.inner.raise_on_error()
    }

    fn __repr__(&self) -> String {
        format!("CoreConfig(raise_on_error={})", self.raise_on_error())
    }
}

#[pyclass(name = "InventoryConfig")]
#[derive(Clone)]
struct PyInventoryConfig {
    inner: InventoryConfig,
}

#[pymethods]
impl PyInventoryConfig {
    #[getter]
    fn plugin(&self) -> String {
        self.inner.plugin().to_owned()
    }

    #[getter]
    fn options(&self) -> PyOptionsConfig {
        PyOptionsConfig {
            inner: self.inner.options().clone(),
        }
    }

    #[getter]
    fn transform_function(&self) -> Option<String> {
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
struct PySSHConfig {
    inner: SSHConfig,
}

#[pymethods]
impl PySSHConfig {
    #[getter]
    fn config_file(&self) -> Option<String> {
        self.inner.config_file().map(str::to_owned)
    }

    fn __repr__(&self) -> String {
        format!("SSHConfig(config_file={:?})", self.config_file())
    }
}

#[pyclass(name = "RunnerConfig")]
#[derive(Clone)]
struct PyRunnerConfig {
    inner: RunnerConfig,
}

#[pymethods]
impl PyRunnerConfig {
    #[getter]
    fn plugin(&self) -> String {
        self.inner.plugin().to_owned()
    }

    #[getter]
    fn worker_count(&self) -> Option<usize> {
        self.inner.worker_count()
    }

    #[getter]
    fn max_task_depth(&self) -> usize {
        self.inner.max_task_depth()
    }

    #[getter]
    fn max_connection_attempts(&self) -> usize {
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
struct PyLoggingConfig {
    inner: LoggingConfig,
}

#[pymethods]
impl PyLoggingConfig {
    #[getter]
    fn enabled(&self) -> bool {
        self.inner.enabled()
    }

    #[getter]
    fn level(&self) -> String {
        self.inner.level().to_owned()
    }

    #[getter]
    fn log_file(&self) -> String {
        self.inner.log_file().to_owned()
    }

    #[getter]
    fn to_console(&self) -> bool {
        self.inner.to_console()
    }

    #[getter]
    fn file_size(&self) -> u64 {
        self.inner.file_size()
    }

    #[getter]
    fn max_file_count(&self) -> usize {
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
struct PySettings {
    inner: Settings,
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
    fn core(&self) -> PyCoreConfig {
        PyCoreConfig {
            inner: self.inner.core().clone(),
        }
    }

    #[getter]
    fn inventory(&self) -> PyInventoryConfig {
        PyInventoryConfig {
            inner: self.inner.inventory().clone(),
        }
    }

    #[getter]
    fn ssh(&self) -> PySSHConfig {
        PySSHConfig {
            inner: self.inner.ssh().clone(),
        }
    }

    #[getter]
    fn runner(&self) -> PyRunnerConfig {
        PyRunnerConfig {
            inner: self.inner.runner().clone(),
        }
    }

    #[getter]
    fn logging(&self) -> PyLoggingConfig {
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

#[pymodule]
fn genja_core(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PySettings>()?;
    module.add_class::<PyCoreConfig>()?;
    module.add_class::<PyOptionsConfig>()?;
    module.add_class::<PyInventoryConfig>()?;
    module.add_class::<PySSHConfig>()?;
    module.add_class::<PyRunnerConfig>()?;
    module.add_class::<PyLoggingConfig>()?;
    Ok(())
}
