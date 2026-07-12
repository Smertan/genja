use ::genja_core::Settings;
use ::genja_core::inventory::TransformFunctionOptions;
use ::genja_core::settings::{
    CoreConfig, InventoryConfig, LoggingConfig, OptionsConfig, RunnerConfig, SSHConfig,
};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyAny, PyModule};

use crate::task;

#[pyclass(name = "RunnerRetryConfig", skip_from_py_object)]
#[derive(Clone)]
pub struct PyRunnerRetryConfig {
    pub(crate) inner: ::genja_core::task::RetryConfig,
}

#[pymethods]
impl PyRunnerRetryConfig {
    #[new]
    #[pyo3(signature = (allow=None, max_attempts=None, delay_ms=None))]
    fn new(allow: Option<bool>, max_attempts: Option<usize>, delay_ms: Option<u64>) -> Self {
        Self {
            inner: ::genja_core::task::RetryConfig::new(
                allow,
                max_attempts.map(|value| value.max(1)),
                delay_ms,
            ),
        }
    }

    #[getter]
    pub(crate) fn allow(&self) -> Option<bool> {
        self.inner.allow()
    }

    #[getter]
    pub(crate) fn max_attempts(&self) -> Option<usize> {
        self.inner.max_attempts()
    }

    #[getter]
    pub(crate) fn delay_ms(&self) -> Option<u64> {
        self.inner.delay_ms()
    }

    fn __repr__(&self) -> String {
        format!(
            "RunnerRetryConfig(allow={:?}, max_attempts={:?}, delay_ms={:?})",
            self.allow(),
            self.max_attempts(),
            self.delay_ms()
        )
    }
}

#[pyclass(name = "OptionsConfig", skip_from_py_object)]
#[derive(Clone)]
pub struct PyOptionsConfig {
    pub(crate) inner: OptionsConfig,
}

#[pymethods]
impl PyOptionsConfig {
    #[new]
    #[pyo3(signature = (hosts_file=None, groups_file=None, defaults_file=None))]
    fn new(
        hosts_file: Option<String>,
        groups_file: Option<String>,
        defaults_file: Option<String>,
    ) -> Self {
        let mut builder = OptionsConfig::builder();
        if let Some(hosts_file) = hosts_file {
            builder = builder.hosts_file(hosts_file);
        }
        if let Some(groups_file) = groups_file {
            builder = builder.groups_file(groups_file);
        }
        if let Some(defaults_file) = defaults_file {
            builder = builder.defaults_file(defaults_file);
        }
        Self {
            inner: builder.build(),
        }
    }

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

#[pyclass(name = "CoreConfig", skip_from_py_object)]
#[derive(Clone)]
pub struct PyCoreConfig {
    pub(crate) inner: CoreConfig,
}

#[pymethods]
impl PyCoreConfig {
    #[new]
    #[pyo3(signature = (raise_on_error=None))]
    fn new(raise_on_error: Option<bool>) -> Self {
        let mut builder = CoreConfig::builder();
        if let Some(raise_on_error) = raise_on_error {
            builder = builder.raise_on_error(raise_on_error);
        }
        Self {
            inner: builder.build(),
        }
    }

    #[getter]
    pub(crate) fn raise_on_error(&self) -> bool {
        self.inner.raise_on_error()
    }

    fn __repr__(&self) -> String {
        format!("CoreConfig(raise_on_error={})", self.raise_on_error())
    }
}

#[pyclass(name = "InventoryConfig", skip_from_py_object)]
#[derive(Clone)]
pub struct PyInventoryConfig {
    pub(crate) inner: InventoryConfig,
}

#[pymethods]
impl PyInventoryConfig {
    #[new]
    #[pyo3(signature = (plugin=None, options=None, transform_function=None, transform_function_options=None))]
    fn new(
        plugin: Option<String>,
        options: Option<PyRef<'_, PyOptionsConfig>>,
        transform_function: Option<String>,
        transform_function_options: Option<Py<PyAny>>,
    ) -> PyResult<Self> {
        let mut builder = InventoryConfig::builder();
        if let Some(plugin) = plugin {
            builder = builder.plugin(plugin);
        }
        if let Some(options) = options {
            builder = builder.options(options.inner.clone());
        }
        if let Some(transform_function) = transform_function {
            builder = builder.transform_function(transform_function);
        }
        if let Some(transform_function_options) = transform_function_options {
            let value = Python::attach(|py| {
                task::py_any_to_json_value(transform_function_options.bind(py))
            })?;
            builder = builder.transform_function_options(TransformFunctionOptions::new(value));
        }
        Ok(Self {
            inner: builder.build(),
        })
    }

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

    #[getter]
    pub(crate) fn transform_function_options(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self.inner.transform_function_options() {
            Some(options) => task::json_value_to_py(py, options),
            None => Ok(py.None()),
        }
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

#[pyclass(name = "SSHConfig", skip_from_py_object)]
#[derive(Clone)]
pub struct PySSHConfig {
    pub(crate) inner: SSHConfig,
}

#[pymethods]
impl PySSHConfig {
    #[new]
    #[pyo3(signature = (config_file=None))]
    fn new(config_file: Option<String>) -> Self {
        let mut builder = SSHConfig::builder();
        if let Some(config_file) = config_file {
            builder = builder.config_file(config_file);
        }
        Self {
            inner: builder.build(),
        }
    }

    #[getter]
    pub(crate) fn config_file(&self) -> Option<String> {
        self.inner.config_file().map(str::to_owned)
    }

    fn __repr__(&self) -> String {
        format!("SSHConfig(config_file={:?})", self.config_file())
    }
}

#[pyclass(name = "RunnerConfig", skip_from_py_object)]
#[derive(Clone)]
pub struct PyRunnerConfig {
    pub(crate) inner: RunnerConfig,
}

#[pymethods]
impl PyRunnerConfig {
    #[new]
    #[pyo3(signature = (plugin=None, worker_count=None, max_task_depth=None, max_connection_attempts=None, retry=None))]
    fn new(
        plugin: Option<String>,
        worker_count: Option<usize>,
        max_task_depth: Option<usize>,
        max_connection_attempts: Option<usize>,
        retry: Option<PyRef<'_, PyRunnerRetryConfig>>,
    ) -> Self {
        let mut builder = RunnerConfig::builder();
        if let Some(plugin) = plugin {
            builder = builder.plugin(plugin);
        }
        if let Some(worker_count) = worker_count {
            builder = builder.worker_count(worker_count);
        }
        if let Some(max_task_depth) = max_task_depth {
            builder = builder.max_task_depth(max_task_depth);
        }
        if let Some(max_connection_attempts) = max_connection_attempts {
            builder = builder.max_connection_attempts(max_connection_attempts);
        }
        if let Some(retry) = retry {
            builder = builder.retry(retry.inner);
        }
        Self {
            inner: builder.build(),
        }
    }

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

    #[getter]
    pub(crate) fn retry(&self) -> PyRunnerRetryConfig {
        PyRunnerRetryConfig {
            inner: *self.inner.retry(),
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "RunnerConfig(plugin={:?}, worker_count={:?}, max_task_depth={}, max_connection_attempts={}, retry={})",
            self.plugin(),
            self.worker_count(),
            self.max_task_depth(),
            self.max_connection_attempts(),
            self.retry().__repr__()
        )
    }
}

#[pyclass(name = "LoggingConfig", skip_from_py_object)]
#[derive(Clone)]
pub struct PyLoggingConfig {
    pub(crate) inner: LoggingConfig,
}

#[pymethods]
impl PyLoggingConfig {
    #[new]
    #[pyo3(signature = (enabled=None, level=None, log_file=None, to_console=None, file_size=None, max_file_count=None))]
    fn new(
        enabled: Option<bool>,
        level: Option<String>,
        log_file: Option<String>,
        to_console: Option<bool>,
        file_size: Option<u64>,
        max_file_count: Option<usize>,
    ) -> Self {
        let mut builder = LoggingConfig::builder();
        if let Some(enabled) = enabled {
            builder = builder.enabled(enabled);
        }
        if let Some(level) = level {
            builder = builder.level(level);
        }
        if let Some(log_file) = log_file {
            builder = builder.log_file(log_file);
        }
        if let Some(to_console) = to_console {
            builder = builder.to_console(to_console);
        }
        if let Some(file_size) = file_size {
            builder = builder.file_size(file_size);
        }
        if let Some(max_file_count) = max_file_count {
            builder = builder.max_file_count(max_file_count);
        }
        Self {
            inner: builder.build(),
        }
    }

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

#[pyclass(name = "Settings", skip_from_py_object)]
#[derive(Clone)]
pub struct PySettings {
    pub(crate) inner: Settings,
}

#[pymethods]
impl PySettings {
    #[new]
    #[pyo3(signature = (core=None, inventory=None, ssh=None, runner=None, logging=None))]
    fn new(
        core: Option<PyRef<'_, PyCoreConfig>>,
        inventory: Option<PyRef<'_, PyInventoryConfig>>,
        ssh: Option<PyRef<'_, PySSHConfig>>,
        runner: Option<PyRef<'_, PyRunnerConfig>>,
        logging: Option<PyRef<'_, PyLoggingConfig>>,
    ) -> Self {
        let mut builder = Settings::builder();
        if let Some(core) = core {
            builder = builder.core(core.inner.clone());
        }
        if let Some(inventory) = inventory {
            builder = builder.inventory(inventory.inner.clone());
        }
        if let Some(ssh) = ssh {
            builder = builder.ssh(ssh.inner.clone());
        }
        if let Some(runner) = runner {
            builder = builder.runner(runner.inner.clone());
        }
        if let Some(logging) = logging {
            builder = builder.logging(logging.inner.clone());
        }
        Self {
            inner: builder.build(),
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
    module.add_class::<PyRunnerRetryConfig>()?;
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
        let settings = PySettings::new(None, None, None, None, None);

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
        pyo3::Python::initialize();
        Python::attach(|py| {
            let module =
                PyModule::new(py, "test_settings_module").expect("test module should be created");

            register(&module).expect("settings classes should register");

            assert!(module.getattr("Settings").is_ok());
            assert!(module.getattr("CoreConfig").is_ok());
            assert!(module.getattr("OptionsConfig").is_ok());
            assert!(module.getattr("InventoryConfig").is_ok());
            assert!(module.getattr("SSHConfig").is_ok());
            assert!(module.getattr("RunnerConfig").is_ok());
            assert!(module.getattr("RunnerRetryConfig").is_ok());
            assert!(module.getattr("LoggingConfig").is_ok());
        });
    }
}
