use ::genja::Genja as RuntimeGenja;
use ::genja_core::inventory::{ConnectionKey, Host, Hosts};
use ::genja_core::task::{
    HostTaskResult, MessageLevel, RetryConfig, Task, TaskConnectionResolver, TaskDefinition,
    TaskError, TaskExecutionMode, TaskFailure, TaskFailureKind, TaskInfo, TaskMessage, TaskResults,
    TaskResultsSummary, TaskRunOptions, TaskRuntimeContext, TaskSkip, TaskSuccess, Tasks,
};
use async_trait::async_trait;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use pyo3_async_runtimes::tokio::future_into_py;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::SystemTime;

use crate::plugin_manager::{
    python_connection_from_runtime_connection, resolve_python_maybe_awaitable_async,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PythonTaskExecutionMode {
    Blocking,
    Async,
}

#[pyclass(name = "HostTaskResult", skip_from_py_object)]
#[derive(Clone)]
pub struct PyHostTaskResult {
    pub(crate) inner: HostTaskResult,
}

#[pymethods]
impl PyHostTaskResult {
    #[staticmethod]
    fn from_python_result(result: Bound<'_, PyAny>) -> PyResult<Self> {
        Ok(Self {
            inner: python_result_to_host_task_result(result)?,
        })
    }

    #[getter]
    fn status(&self) -> &'static str {
        self.inner.status()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let value = host_task_result_to_json(&self.inner);
        json_value_to_py(py, &value)
    }

    fn __repr__(&self) -> String {
        format!("HostTaskResult(status={:?})", self.status())
    }
}

#[derive(Clone)]
struct PythonTaskSpec {
    name: String,
    connection_plugin_name: Option<String>,
    processor_names: Vec<String>,
    retry_config: Option<RetryConfig>,
    options: Option<Value>,
    supports_dry_run: bool,
    py_task_class: Arc<Py<PyAny>>,
    execution_mode: PythonTaskExecutionMode,
    sub_tasks: Vec<PythonTaskSpec>,
}

struct PythonBackedTask {
    spec: PythonTaskSpec,
    sub_tasks: Vec<Arc<dyn Task>>,
}

impl TaskInfo for PythonBackedTask {
    fn name(&self) -> &str {
        &self.spec.name
    }

    fn connection_plugin_name(&self) -> Option<&str> {
        self.spec.connection_plugin_name.as_deref()
    }

    fn get_connection_key(&self, hostname: &str) -> Option<ConnectionKey> {
        self.connection_plugin_name()
            .map(|plugin_name| ConnectionKey::new(hostname, plugin_name))
    }

    fn processor_names(&self) -> Vec<&str> {
        self.spec
            .processor_names
            .iter()
            .map(String::as_str)
            .collect()
    }

    fn options(&self) -> Option<&Value> {
        self.spec.options.as_ref()
    }

    fn retry_config(&self) -> Option<&RetryConfig> {
        self.spec.retry_config.as_ref()
    }

    fn supports_dry_run(&self) -> bool {
        self.spec.supports_dry_run
    }
}

#[async_trait]
impl Task for PythonBackedTask {
    fn start(
        &self,
        host: &Host,
        context: &::genja_core::task::BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        let python_connection = match context.connection() {
            Some(connection) => Some(connection.with_connection(|connection| {
                Ok(python_connection_from_runtime_connection(connection))
            })?),
            None => None,
        }
        .flatten();
        self.run_python_blocking(
            host,
            context.current_depth(),
            Some(context.max_depth()),
            context.current_attempt(),
            context.dry_run(),
            python_connection,
            "start",
        )
    }

    fn dry_run(
        &self,
        host: &Host,
        context: &::genja_core::task::BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        let python_connection = match context.connection() {
            Some(connection) => Some(connection.with_connection(|connection| {
                Ok(python_connection_from_runtime_connection(connection))
            })?),
            None => None,
        }
        .flatten();
        self.run_python_blocking(
            host,
            context.current_depth(),
            Some(context.max_depth()),
            context.current_attempt(),
            context.dry_run(),
            python_connection,
            "dry_run",
        )
    }

    async fn start_async(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        let python_connection = if let Some(connection) = context.connection() {
            let guard = connection.lock().await;
            python_connection_from_runtime_connection(&*guard)
        } else {
            None
        };
        self.run_python(
            host,
            context.current_depth(),
            Some(context.max_depth()),
            context.current_attempt(),
            context.dry_run(),
            python_connection,
            "start_async",
        )
        .await
    }

    async fn dry_run_async(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        let python_connection = if let Some(connection) = context.connection() {
            let guard = connection.lock().await;
            python_connection_from_runtime_connection(&*guard)
        } else {
            None
        };
        self.run_python(
            host,
            context.current_depth(),
            Some(context.max_depth()),
            context.current_attempt(),
            context.dry_run(),
            python_connection,
            "dry_run_async",
        )
        .await
    }

    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        self.sub_tasks.clone()
    }

    fn execution_mode(&self) -> TaskExecutionMode {
        match self.spec.execution_mode {
            PythonTaskExecutionMode::Blocking => TaskExecutionMode::Blocking,
            PythonTaskExecutionMode::Async => TaskExecutionMode::Async,
        }
    }
}

impl PythonBackedTask {
    fn run_python_blocking(
        &self,
        host: &Host,
        current_depth: usize,
        max_depth: Option<usize>,
        current_attempt: usize,
        dry_run: bool,
        connection: Option<Py<PyAny>>,
        method_name: &str,
    ) -> Result<HostTaskResult, TaskError> {
        let result = Python::attach(|py| {
            let class = self.spec.py_task_class.as_ref().bind(py);
            let instance = class.call0().map_err(python_task_error)?;
            let task_payload = build_python_task_model(
                py,
                "TaskInfo",
                python_task_spec_to_py_dict(py, &self.spec).map_err(python_task_error)?,
            )
            .map_err(python_task_error)?;
            let host_payload = build_python_task_model(
                py,
                "Host",
                host_to_py_dict(py, host).map_err(python_task_error)?,
            )
            .map_err(python_task_error)?;
            let context_payload = build_python_task_runtime_context(
                py,
                current_depth,
                max_depth,
                current_attempt,
                dry_run,
                connection,
            )
            .map_err(python_task_error)?;
            let result = instance
                .call_method1(method_name, (task_payload, host_payload, context_payload))
                .map_err(python_task_error)?;
            let is_awaitable: bool = PyModule::import(py, "inspect")
                .map_err(python_task_error)?
                .call_method1("isawaitable", (result.clone(),))
                .map_err(python_task_error)?
                .extract()
                .map_err(python_task_error)?;
            if is_awaitable {
                return Err(TaskError::new(std::io::Error::other(
                    "python blocking task start() must not return an awaitable",
                )));
            }
            python_result_to_host_task_result(result).map_err(python_task_error)
        })?;

        Ok(result)
    }

    async fn run_python(
        &self,
        host: &Host,
        current_depth: usize,
        max_depth: Option<usize>,
        current_attempt: usize,
        dry_run: bool,
        connection: Option<Py<PyAny>>,
        method_name: &str,
    ) -> Result<HostTaskResult, TaskError> {
        let result = Python::attach(|py| {
            let class = self.spec.py_task_class.as_ref().bind(py);
            let instance = class.call0().map_err(python_task_error)?;
            let task_payload = build_python_task_model(
                py,
                "TaskInfo",
                python_task_spec_to_py_dict(py, &self.spec).map_err(python_task_error)?,
            )
            .map_err(python_task_error)?;
            let host_payload = build_python_task_model(
                py,
                "Host",
                host_to_py_dict(py, host).map_err(python_task_error)?,
            )
            .map_err(python_task_error)?;
            let context_payload = build_python_task_runtime_context(
                py,
                current_depth,
                max_depth,
                current_attempt,
                dry_run,
                connection,
            )
            .map_err(python_task_error)?;

            instance
                .call_method1(method_name, (task_payload, host_payload, context_payload))
                .map(Bound::unbind)
                .map_err(python_task_error)
        })?;
        let result = resolve_python_maybe_awaitable_async(result)
            .await
            .map_err(python_task_error)?;
        Python::attach(|py| {
            python_result_to_host_task_result(result.bind(py).clone()).map_err(python_task_error)
        })
    }
}

#[pyclass(name = "TaskDefinition", skip_from_py_object)]
#[derive(Clone)]
pub struct PyTaskDefinition {
    spec: Option<PythonTaskSpec>,
    inner: TaskDefinition,
}

#[pyclass(name = "Tasks", skip_from_py_object)]
#[derive(Clone, Default)]
pub struct PyTasks {
    specs: Vec<PythonTaskSpec>,
}

#[pyclass(name = "TaskConnectionResolver", skip_from_py_object)]
#[derive(Clone)]
pub struct PyTaskConnectionResolver {
    pub(crate) inner: Option<Arc<dyn TaskConnectionResolver>>,
}

#[pyclass(name = "TaskRunOptions", skip_from_py_object)]
#[derive(Clone)]
/// Python wrapper for runtime task execution options.
pub struct PyTaskRunOptions {
    pub(crate) max_depth: Option<usize>,
    pub(crate) dry_run: bool,
}

#[pymethods]
impl PyTaskDefinition {
    #[staticmethod]
    fn from_python_class(py_task_class: Bound<'_, PyAny>) -> PyResult<Self> {
        let spec = extract_python_task_spec(py_task_class)?;
        Ok(Self {
            inner: task_definition_from_spec(&spec),
            spec: Some(spec),
        })
    }

    #[getter]
    fn name(&self) -> String {
        self.inner.name().to_string()
    }

    #[getter]
    fn connection_plugin_name(&self) -> Option<String> {
        self.inner.connection_plugin_name().map(str::to_owned)
    }

    #[getter]
    fn sub_tasks(&self) -> Vec<Self> {
        if let Some(spec) = self.spec.as_ref() {
            return spec
                .sub_tasks
                .iter()
                .cloned()
                .map(|spec| Self {
                    inner: task_definition_from_spec(&spec),
                    spec: Some(spec),
                })
                .collect();
        }

        self.inner
            .as_task()
            .sub_tasks()
            .into_iter()
            .map(|task| Self {
                inner: TaskDefinition::new(RuntimeTaskWrapper { inner: task }),
                spec: None,
            })
            .collect()
    }

    #[getter]
    fn retry(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        retry_config_to_py_object(py, self.inner.retry_config())
    }

    #[getter]
    fn supports_dry_run(&self) -> bool {
        self.inner.supports_dry_run()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        match self.spec.as_ref() {
            Some(spec) => json_value_to_py(py, &python_task_spec_to_json(spec)),
            None => json_value_to_py(py, &task_definition_to_json(&self.inner)),
        }
    }

    #[pyo3(signature = (host, connection_resolver=None, run_options=None))]
    fn run_on_host(
        &self,
        host: Bound<'_, PyAny>,
        connection_resolver: Option<PyRef<'_, PyTaskConnectionResolver>>,
        run_options: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyTaskResults> {
        let py = host.py();
        let host = python_host_to_rust_host(host)?;
        let mut hosts = Hosts::new();
        let host_id = host.hostname().unwrap_or("host").to_string();
        hosts.add_host(host_id, host);
        let resolver = connection_resolver.and_then(|resolver| resolver.inner.clone());
        let run_options = resolve_task_run_options(run_options.as_ref(), 0)?;
        let inner = py
            .detach(|| run_task_definition_on_hosts(&self.inner, &hosts, resolver, run_options))
            .map_err(|err| PyValueError::new_err(format!("python task execution failed: {err}")))?;
        Ok(PyTaskResults { inner })
    }

    #[pyo3(signature = (hosts, connection_resolver=None, run_options=None))]
    fn run_on_hosts(
        &self,
        hosts: Bound<'_, PyAny>,
        connection_resolver: Option<PyRef<'_, PyTaskConnectionResolver>>,
        run_options: Option<Bound<'_, PyAny>>,
    ) -> PyResult<PyTaskResults> {
        let py = hosts.py();
        let hosts = python_hosts_to_rust_hosts(hosts)?;
        let resolver = connection_resolver.and_then(|resolver| resolver.inner.clone());
        let run_options = resolve_task_run_options(run_options.as_ref(), 0)?;
        let inner = py
            .detach(|| run_task_definition_on_hosts(&self.inner, &hosts, resolver, run_options))
            .map_err(|err| PyValueError::new_err(format!("python task execution failed: {err}")))?;
        Ok(PyTaskResults { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskDefinition(name={:?}, connection_plugin_name={:?}, sub_tasks={})",
            self.name(),
            self.connection_plugin_name(),
            self.sub_tasks().len()
        )
    }
}

#[pymethods]
impl PyTaskRunOptions {
    /// Create runtime task execution options.
    #[new]
    #[pyo3(signature = (max_depth=None, dry_run=false))]
    fn new(max_depth: Option<usize>, dry_run: bool) -> Self {
        Self { max_depth, dry_run }
    }

    /// Return the maximum nested sub-task depth override, if configured.
    #[getter]
    fn max_depth(&self) -> Option<usize> {
        self.max_depth
    }

    /// Return whether dry-run execution is requested.
    #[getter]
    fn dry_run(&self) -> bool {
        self.dry_run
    }

    /// Return a copy with a different maximum nested sub-task depth.
    fn with_max_depth(&self, max_depth: usize) -> Self {
        Self {
            max_depth: Some(max_depth),
            dry_run: self.dry_run,
        }
    }

    /// Return a copy with dry-run execution enabled or disabled.
    fn with_dry_run(&self, dry_run: bool) -> Self {
        Self {
            max_depth: self.max_depth,
            dry_run,
        }
    }

    /// Convert the options to a Python dictionary.
    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let payload = PyDict::new(py);
        match self.max_depth {
            Some(max_depth) => payload.set_item("max_depth", max_depth)?,
            None => payload.set_item("max_depth", py.None())?,
        }
        payload.set_item("dry_run", self.dry_run)?;
        Ok(payload.unbind().into())
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskRunOptions(max_depth={:?}, dry_run={})",
            self.max_depth, self.dry_run
        )
    }
}

impl PyTaskDefinition {
    pub(crate) fn from_runtime_definition(inner: TaskDefinition) -> Self {
        Self { spec: None, inner }
    }
}

#[pymethods]
impl PyTasks {
    #[new]
    fn new() -> Self {
        Self::default()
    }

    #[pyo3(signature = (task_class))]
    fn add_task(&mut self, task_class: Bound<'_, PyAny>) -> PyResult<()> {
        self.specs.push(extract_python_task_spec(task_class)?);
        Ok(())
    }

    fn task_definitions(&self) -> Vec<PyTaskDefinition> {
        self.specs
            .iter()
            .cloned()
            .map(|spec| PyTaskDefinition {
                inner: task_definition_from_spec(&spec),
                spec: Some(spec),
            })
            .collect()
    }

    fn to_list(&self) -> Vec<PyTaskDefinition> {
        self.task_definitions()
    }

    fn __len__(&self) -> usize {
        self.specs.len()
    }

    fn __getitem__(&self, index: isize) -> PyResult<PyTaskDefinition> {
        let len = self.specs.len() as isize;
        let index = if index < 0 { len + index } else { index };
        if index < 0 || index >= len {
            return Err(pyo3::exceptions::PyIndexError::new_err(
                "task index out of range",
            ));
        }
        let spec = self.specs[index as usize].clone();
        Ok(PyTaskDefinition {
            inner: task_definition_from_spec(&spec),
            spec: Some(spec),
        })
    }

    fn __repr__(&self) -> String {
        let names = self
            .specs
            .iter()
            .map(|spec| spec.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        format!("Tasks([{names}])")
    }
}

impl PyTasks {
    fn to_runtime_tasks(&self) -> Tasks {
        let mut tasks = Tasks::new();
        for spec in &self.specs {
            tasks.push(task_definition_from_spec(spec));
        }
        tasks
    }
}

#[pymethods]
impl PyTaskConnectionResolver {
    fn __repr__(&self) -> String {
        format!(
            "TaskConnectionResolver(available={})",
            self.inner.as_ref().is_some()
        )
    }
}

#[pyclass(name = "TaskResults", skip_from_py_object)]
#[derive(Clone)]
pub struct PyTaskResults {
    pub(crate) inner: TaskResults,
}

#[pymethods]
impl PyTaskResults {
    #[getter]
    fn task_name(&self) -> String {
        self.inner.task_name().to_string()
    }

    #[getter]
    fn passed_hosts(&self) -> Vec<String> {
        self.inner
            .passed_hosts()
            .into_iter()
            .map(|host| host.to_string())
            .collect()
    }

    #[getter]
    fn failed_hosts(&self) -> Vec<String> {
        self.inner
            .failed_hosts()
            .into_iter()
            .map(|host| host.to_string())
            .collect()
    }

    #[getter]
    fn skipped_hosts(&self) -> Vec<String> {
        self.inner
            .skipped_hosts()
            .into_iter()
            .map(|host| host.to_string())
            .collect()
    }

    fn merge(&mut self, other: PyRef<'_, PyTaskResults>) {
        self.inner.merge(other.inner.clone());
    }

    fn host_summary(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let summary = self.inner.host_summary();
        let value = json!({
            "passed": summary.passed(),
            "failed": summary.failed(),
            "skipped": summary.skipped(),
            "total": summary.total(),
        });
        json_value_to_py(py, &value)
    }

    fn task_summary(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_value_to_py(
            py,
            &task_results_summary_to_json(&self.inner.task_summary()),
        )
    }

    #[pyo3(signature = (*, raw=false))]
    fn to_dict(&self, py: Python<'_>, raw: bool) -> PyResult<Py<PyAny>> {
        let dumped = if raw {
            self.inner.to_raw_json_string()
        } else {
            self.inner.to_json_string()
        }
        .map_err(|err| PyValueError::new_err(format!("failed to serialize task results: {err}")))?;

        let value: Value = serde_json::from_str(&dumped).map_err(|err| {
            PyValueError::new_err(format!("failed to parse task results json: {err}"))
        })?;
        let value = if raw {
            value
        } else {
            normalize_task_results_json(&value)
        };
        json_value_to_py(py, &value)
    }

    #[pyo3(signature = (*, raw=false, pretty=false))]
    fn to_json(&self, raw: bool, pretty: bool) -> PyResult<String> {
        let dumped = match (raw, pretty) {
            (true, true) => self.inner.to_raw_pretty_json_string(),
            (true, false) => self.inner.to_raw_json_string(),
            (false, true) => self.inner.to_pretty_json_string(),
            (false, false) => self.inner.to_json_string(),
        }
        .map_err(|err| PyValueError::new_err(format!("failed to serialize task results: {err}")))?;

        if raw {
            return Ok(dumped);
        }

        let value: Value = serde_json::from_str(&dumped).map_err(|err| {
            PyValueError::new_err(format!("failed to parse task results json: {err}"))
        })?;
        let normalized = normalize_task_results_json(&value);
        if pretty {
            serde_json::to_string_pretty(&normalized).map_err(|err| {
                PyValueError::new_err(format!("failed to serialize task results: {err}"))
            })
        } else {
            serde_json::to_string(&normalized).map_err(|err| {
                PyValueError::new_err(format!("failed to serialize task results: {err}"))
            })
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskResults(task_name={:?}, passed={}, failed={}, skipped={})",
            self.inner.task_name(),
            self.inner.passed_hosts().len(),
            self.inner.failed_hosts().len(),
            self.inner.skipped_hosts().len()
        )
    }
}

pub fn run_task(
    py: Python<'_>,
    runtime: &RuntimeGenja,
    task_class: Bound<'_, PyAny>,
    run_options: Option<Bound<'_, PyAny>>,
) -> PyResult<PyTaskResults> {
    let spec = extract_python_task_spec(task_class)?;
    let task = task_from_spec(&spec);
    let run_options = resolve_task_run_options(
        run_options.as_ref(),
        runtime.settings().runner().max_task_depth(),
    )?;
    let inner = py
        .detach(|| runtime.run_task_with_options(task, run_options))
        .map_err(|err| {
            PyValueError::new_err(format!("failed to run task through Genja runtime: {err}"))
        })?;
    Ok(PyTaskResults { inner })
}

pub fn run_task_async(
    py: Python<'_>,
    runtime: &RuntimeGenja,
    task_class: Bound<'_, PyAny>,
    run_options: Option<Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let spec = extract_python_task_spec(task_class)?;
    let runtime = runtime.clone();
    let task = task_from_spec(&spec);
    let run_options = resolve_task_run_options(
        run_options.as_ref(),
        runtime.settings().runner().max_task_depth(),
    )?;

    future_into_py(py, async move {
        let inner = runtime
            .run_task_with_options_async(task, run_options)
            .await
            .map_err(|err| {
                PyValueError::new_err(format!("failed to run task through Genja runtime: {err}"))
            })?;
        Ok(PyTaskResults { inner })
    })
    .map(Bound::unbind)
}

pub fn run_tasks(
    py: Python<'_>,
    runtime: &RuntimeGenja,
    task_input: Bound<'_, PyAny>,
    run_options: Option<Bound<'_, PyAny>>,
) -> PyResult<Vec<PyTaskResults>> {
    let tasks = task_input
        .extract::<PyRef<'_, PyTasks>>()
        .map_err(|_| PyValueError::new_err("tasks must be a genja.Tasks instance"))?
        .to_runtime_tasks();

    let run_options = resolve_task_run_options(
        run_options.as_ref(),
        runtime.settings().runner().max_task_depth(),
    )?;
    let results = py
        .detach(|| runtime.run_tasks_with_options(tasks, run_options))
        .map_err(|err| {
            PyValueError::new_err(format!("failed to run tasks through Genja runtime: {err}"))
        })?;
    Ok(results
        .into_iter()
        .map(|inner| PyTaskResults { inner })
        .collect())
}

pub fn run_tasks_async(
    py: Python<'_>,
    runtime: &RuntimeGenja,
    task_input: Bound<'_, PyAny>,
    run_options: Option<Bound<'_, PyAny>>,
) -> PyResult<Py<PyAny>> {
    let tasks = task_input
        .extract::<PyRef<'_, PyTasks>>()
        .map_err(|_| PyValueError::new_err("tasks must be a genja.Tasks instance"))?
        .to_runtime_tasks();

    let runtime = runtime.clone();
    let run_options = resolve_task_run_options(
        run_options.as_ref(),
        runtime.settings().runner().max_task_depth(),
    )?;

    future_into_py(py, async move {
        let results = runtime
            .run_tasks_with_options_async(tasks, run_options)
            .await
            .map_err(|err| {
                PyValueError::new_err(format!("failed to run tasks through Genja runtime: {err}"))
            })?;
        Ok(results
            .into_iter()
            .map(|inner| PyTaskResults { inner })
            .collect::<Vec<_>>())
    })
    .map(Bound::unbind)
}

fn resolve_task_run_options(
    run_options: Option<&Bound<'_, PyAny>>,
    default_max_depth: usize,
) -> PyResult<TaskRunOptions> {
    if let Some(run_options) = run_options {
        if run_options.is_none() {
            // Explicit None is equivalent to omitting run_options.
        } else if let Ok(run_options) = run_options.extract::<PyRef<'_, PyTaskRunOptions>>() {
            return Ok(
                TaskRunOptions::new(run_options.max_depth.unwrap_or(default_max_depth))
                    .with_dry_run(run_options.dry_run),
            );
        } else {
            return Err(PyValueError::new_err(
                "run_options must be a genja.TaskRunOptions instance",
            ));
        }
    }

    Ok(TaskRunOptions::new(default_max_depth))
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyHostTaskResult>()?;
    module.add_class::<PyTaskDefinition>()?;
    module.add_class::<PyTasks>()?;
    module.add_class::<PyTaskConnectionResolver>()?;
    module.add_class::<PyTaskRunOptions>()?;
    module.add_class::<PyTaskResults>()?;
    Ok(())
}

pub(crate) fn python_result_to_host_task_result(obj: Bound<'_, PyAny>) -> PyResult<HostTaskResult> {
    let value = normalize_python_json_payload(&obj, "invalid python task result")?;
    host_task_result_from_payload(&value)
}

pub(crate) fn python_result_to_task_results(obj: Bound<'_, PyAny>) -> PyResult<TaskResults> {
    let value = normalize_python_task_results_payload(&obj)?;
    json_to_task_results(&value)
}

fn host_task_result_from_payload(value: &Value) -> PyResult<HostTaskResult> {
    if let Some(outcome) = value.get("outcome") {
        return host_task_result_from_payload(outcome);
    }

    let result_value = if let Some(passed) = value.get("Passed") {
        let mut tagged = passed.clone();
        tagged["status"] = Value::String("passed".to_string());
        tagged
    } else if let Some(failed) = value.get("Failed") {
        let mut tagged = failed.clone();
        tagged["status"] = Value::String("failed".to_string());
        tagged
    } else if let Some(skipped) = value.get("Skipped") {
        let mut tagged = skipped.clone();
        tagged["status"] = Value::String("skipped".to_string());
        tagged
    } else {
        value.clone()
    };

    let status = result_value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("python task result is missing 'status'"))?;

    match status {
        "passed" => Ok(HostTaskResult::passed(json_to_task_success(&result_value)?)),
        "failed" => Ok(HostTaskResult::failed(json_to_task_failure(&result_value)?)),
        "skipped" => Ok(HostTaskResult::skipped_with_detail(json_to_task_skip(
            &result_value,
        ))),
        other => Err(PyValueError::new_err(format!(
            "unsupported python task result status '{other}'"
        ))),
    }
}

fn json_to_task_success(value: &Value) -> PyResult<TaskSuccess> {
    let mut success = TaskSuccess::new();

    if let Some(result) = value.get("result") {
        success = success.with_result(result.clone());
    }
    if let Some(changed) = value.get("changed").and_then(Value::as_bool) {
        success = success.with_changed(changed);
    }
    if let Some(diff) = value.get("diff").and_then(Value::as_str) {
        success = success.with_diff(diff);
    }
    if let Some(summary) = value.get("summary").and_then(Value::as_str) {
        success = success.with_summary(summary);
    }
    if let Some(warnings) = value.get("warnings").and_then(Value::as_array) {
        for warning in warnings {
            if let Some(warning) = warning.as_str() {
                success = success.with_warning(warning);
            }
        }
    }
    if let Some(messages) = value.get("messages").and_then(Value::as_array) {
        for message in messages {
            success = success.with_message(json_to_task_message(message)?);
        }
    }
    if let Some(metadata) = value.get("metadata") {
        success = success.with_metadata(metadata.clone());
    }

    Ok(success)
}

fn json_to_task_failure(value: &Value) -> PyResult<TaskFailure> {
    let message = value
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("failed task result is missing 'message'"))?;
    let mut failure = TaskFailure::capture(message.to_string());

    if let Some(kind) = value.get("kind").and_then(Value::as_str) {
        failure = failure.with_kind(parse_failure_kind(kind)?);
    } else {
        failure = failure.with_kind(TaskFailureKind::External);
    }
    if let Some(retryable) = value.get("retryable").and_then(Value::as_bool) {
        failure = failure.with_retryable(retryable);
    }
    if let Some(details) = value.get("details") {
        failure = failure.with_details(details.clone());
    }
    if let Some(warnings) = value.get("warnings").and_then(Value::as_array) {
        for warning in warnings {
            if let Some(warning) = warning.as_str() {
                failure = failure.with_warning(warning);
            }
        }
    }
    if let Some(messages) = value.get("messages").and_then(Value::as_array) {
        for message in messages {
            failure = failure.with_message(json_to_task_message(message)?);
        }
    }

    Ok(failure)
}

fn json_to_task_skip(value: &Value) -> TaskSkip {
    let mut skip = TaskSkip::new();
    if let Some(reason) = value.get("reason").and_then(Value::as_str) {
        skip = skip.with_reason(reason);
    }
    if let Some(message) = value.get("message").and_then(Value::as_str) {
        skip = skip.with_message(message);
    }
    skip
}

fn json_to_task_message(value: &Value) -> PyResult<TaskMessage> {
    let level = value
        .get("level")
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("task message is missing 'level'"))?;
    let text = value
        .get("text")
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("task message is missing 'text'"))?;

    let mut message = TaskMessage::new(parse_message_level(level)?, text);

    if let Some(code) = value.get("code").and_then(Value::as_str) {
        message = message.with_code(code);
    }
    if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
        let parsed = humantime::parse_rfc3339(timestamp).map_err(|err| {
            PyValueError::new_err(format!(
                "invalid task message timestamp '{timestamp}': {err}"
            ))
        })?;
        message = message.with_timestamp(parsed);
    }

    Ok(message)
}

fn parse_message_level(level: &str) -> PyResult<MessageLevel> {
    match level {
        "info" | "Info" => Ok(MessageLevel::Info),
        "warning" | "warn" | "Warning" | "Warn" => Ok(MessageLevel::Warning),
        "error" | "Error" => Ok(MessageLevel::Error),
        "debug" | "Debug" => Ok(MessageLevel::Debug),
        other => Err(PyValueError::new_err(format!(
            "unsupported task message level '{other}'"
        ))),
    }
}

fn parse_failure_kind(kind: &str) -> PyResult<TaskFailureKind> {
    match kind {
        "connection" | "Connection" => Ok(TaskFailureKind::Connection),
        "authentication" | "Authentication" => Ok(TaskFailureKind::Authentication),
        "validation" | "Validation" => Ok(TaskFailureKind::Validation),
        "timeout" | "Timeout" => Ok(TaskFailureKind::Timeout),
        "command" | "Command" => Ok(TaskFailureKind::Command),
        "unsupported" | "Unsupported" => Ok(TaskFailureKind::Unsupported),
        "internal" | "Internal" => Ok(TaskFailureKind::Internal),
        "external" | "External" => Ok(TaskFailureKind::External),
        other => Err(PyValueError::new_err(format!(
            "unsupported task failure kind '{other}'"
        ))),
    }
}

fn host_task_result_to_json(result: &HostTaskResult) -> Value {
    serde_json::to_value(result).expect("host task result should serialize")
}

fn normalize_task_results_json(value: &Value) -> Value {
    let Some(object) = value.as_object() else {
        return value.clone();
    };

    let mut normalized = object.clone();

    if let Some(hosts) = object.get("hosts").and_then(Value::as_object) {
        let normalized_hosts = hosts
            .iter()
            .map(|(hostname, result)| {
                let normalized_result = host_task_result_from_payload(result).map(|host_result| {
                    let mut normalized_result = host_task_result_to_json(&host_result);
                    if let Some(execution_metadata) = result
                        .get("execution_metadata")
                        .or_else(|| result.get("execution_metadata_raw"))
                    {
                        normalized_result["execution_metadata"] = execution_metadata.clone();
                    }
                    normalized_result
                });
                (
                    hostname.clone(),
                    normalized_result.unwrap_or_else(|_| result.clone()),
                )
            })
            .collect::<serde_json::Map<String, Value>>();
        normalized.insert("hosts".to_string(), Value::Object(normalized_hosts));
    }

    if let Some(sub_tasks) = object.get("sub_tasks").and_then(Value::as_object) {
        let normalized_sub_tasks = sub_tasks
            .iter()
            .map(|(task_name, sub_results)| {
                (task_name.clone(), normalize_task_results_json(sub_results))
            })
            .collect::<serde_json::Map<String, Value>>();
        normalized.insert("sub_tasks".to_string(), Value::Object(normalized_sub_tasks));
    }

    Value::Object(normalized)
}

fn task_results_summary_to_json(summary: &TaskResultsSummary) -> Value {
    let sub_tasks = summary
        .sub_tasks()
        .iter()
        .map(|(task_name, sub_summary)| {
            (
                task_name.to_string(),
                task_results_summary_to_json(sub_summary),
            )
        })
        .collect::<serde_json::Map<String, Value>>();

    json!({
        "task_name": summary.task_name(),
        "hosts": {
            "passed": summary.hosts().passed(),
            "failed": summary.hosts().failed(),
            "skipped": summary.hosts().skipped(),
            "total": summary.hosts().total(),
        },
        "duration_ms": summary.duration_ms(),
        "duration": summary.duration_display(),
        "sub_tasks": Value::Object(sub_tasks),
    })
}

fn task_definition_to_json(task_definition: &TaskDefinition) -> Value {
    task_to_json(task_definition.as_task())
}

fn task_to_json(task: &dyn Task) -> Value {
    json!({
        "name": task.name(),
        "connection_plugin_name": task.connection_plugin_name(),
        "processors": task.processor_names(),
        "retry": retry_config_to_json(task.retry_config()),
        "options": task.options(),
        "supports_dry_run": task.supports_dry_run(),
        "sub_tasks": task.sub_tasks().into_iter().map(|sub_task| task_to_json(sub_task.as_ref())).collect::<Vec<_>>(),
    })
}

fn retry_config_to_json(retry_config: Option<&RetryConfig>) -> Value {
    match retry_config {
        Some(retry_config) => {
            serde_json::to_value(retry_config).expect("retry config should serialize")
        }
        None => Value::Null,
    }
}

fn run_task_definition_on_hosts(
    task_definition: &TaskDefinition,
    hosts: &Hosts,
    connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
    run_options: TaskRunOptions,
) -> Result<TaskResults, ::genja_core::GenjaError> {
    let future = async {
        let started_at = SystemTime::now();
        let mut results = TaskResults::new(task_definition.name()).with_started_at(started_at);
        task_definition.process_task_start(&mut results)?;

        for (host_id, host) in hosts.iter() {
            let mut host_results = TaskResults::new(task_definition.name());
            task_definition
                .start_with_connection_resolver_and_options(
                    host_id.as_str(),
                    host,
                    &mut host_results,
                    connection_resolver.as_deref(),
                    run_options,
                )
                .await?;
            results.merge(host_results);
        }

        let finished_at = SystemTime::now();
        let duration_ns = finished_at
            .duration_since(started_at)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        results = results
            .with_finished_at(finished_at)
            .with_duration_ns(duration_ns);
        task_definition.process_task_finish(&mut results)?;
        Ok(results)
    };

    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        tokio::task::block_in_place(|| handle.block_on(future))
    } else {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| {
                ::genja_core::GenjaError::Message(format!("failed to build async runtime: {err}"))
            })?;
        runtime.block_on(future)
    }
}

fn extract_python_task_spec(py_task_class: Bound<'_, PyAny>) -> PyResult<PythonTaskSpec> {
    let class_dict = py_task_class.getattr("__dict__")?;
    let has_dry_run = class_dict.contains("dry_run")?;
    let has_dry_run_async = class_dict.contains("dry_run_async")?;
    let execution_mode = match (
        class_dict.contains("start")?,
        class_dict.contains("start_async")?,
    ) {
        (true, false) => PythonTaskExecutionMode::Blocking,
        (false, true) => PythonTaskExecutionMode::Async,
        (true, true) | (false, false) => {
            return Err(PyValueError::new_err(
                "python task class must define exactly one of 'start' or 'start_async'",
            ));
        }
    };
    let info_obj = class_dict
        .call_method1("get", ("__genja_task_info__",))?
        .extract::<Option<Py<PyAny>>>()?
        .map(|value| value.bind(py_task_class.py()).clone())
        .ok_or_else(|| PyValueError::new_err("python task class is missing __genja_task_info__"))?;
    let info: Bound<'_, PyDict> = info_obj.cast_into()?;

    let name: String = info
        .get_item("name")?
        .ok_or_else(|| PyValueError::new_err("python task metadata is missing 'name'"))?
        .extract()?;
    if name.trim().is_empty() {
        return Err(PyValueError::new_err(
            "python task metadata field 'name' must not be empty",
        ));
    }
    let connection_plugin_name = if let Some(value) = info.get_item("connection_plugin_name")? {
        if value.is_none() {
            None
        } else {
            let connection_plugin_name: String = value.extract()?;
            if connection_plugin_name.trim().is_empty() {
                return Err(PyValueError::new_err(
                    "python task metadata field 'connection_plugin_name' must not be empty",
                ));
            }
            Some(connection_plugin_name)
        }
    } else {
        None
    };
    let processor_names = if let Some(processors) = info.get_item("processors")? {
        processors.extract::<Vec<String>>()?
    } else {
        Vec::new()
    };
    reject_misplaced_retry_metadata(&info)?;
    let retry_config = extract_retry_config_from_metadata(&info)?;

    let supports_dry_run = if let Some(value) = info.get_item("supports_dry_run")? {
        if value.is_none() {
            false
        } else {
            value.extract::<bool>()?
        }
    } else {
        false
    };
    if supports_dry_run {
        match (execution_mode, has_dry_run, has_dry_run_async) {
            (PythonTaskExecutionMode::Blocking, true, false) => {}
            (PythonTaskExecutionMode::Async, false, true) => {}
            (PythonTaskExecutionMode::Blocking, false, _) => {
                return Err(PyValueError::new_err(
                    "supports_dry_run=True requires 'dry_run' for sync Python tasks",
                ));
            }
            (PythonTaskExecutionMode::Async, _, false) => {
                return Err(PyValueError::new_err(
                    "supports_dry_run=True requires 'dry_run_async' for async Python tasks",
                ));
            }
            (PythonTaskExecutionMode::Blocking, _, true) => {
                return Err(PyValueError::new_err(
                    "supports_dry_run=True requires sync Python tasks to define 'dry_run', not 'dry_run_async'",
                ));
            }
            (PythonTaskExecutionMode::Async, true, _) => {
                return Err(PyValueError::new_err(
                    "supports_dry_run=True requires async Python tasks to define 'dry_run_async', not 'dry_run'",
                ));
            }
        }
    }

    let options = if let Some(options) = info.get_item("options")? {
        if options.is_none() {
            None
        } else {
            Some(py_any_to_json_value(&options)?)
        }
    } else {
        None
    };

    let mut sub_tasks = Vec::new();
    if let Some(py_sub_tasks) = info.get_item("sub_tasks")?
        && !py_sub_tasks.is_none()
    {
        for sub_task in py_sub_tasks.try_iter()? {
            sub_tasks.push(extract_python_task_spec(sub_task?)?);
        }
    }

    Ok(PythonTaskSpec {
        name,
        connection_plugin_name,
        processor_names,
        retry_config,
        options,
        supports_dry_run,
        py_task_class: Arc::new(py_task_class.unbind()),
        execution_mode,
        sub_tasks,
    })
}

fn task_from_spec(spec: &PythonTaskSpec) -> PythonBackedTask {
    PythonBackedTask {
        spec: spec.clone(),
        sub_tasks: spec
            .sub_tasks
            .iter()
            .map(|sub| Arc::new(task_from_spec(sub)) as Arc<dyn Task>)
            .collect(),
    }
}

fn task_definition_from_spec(spec: &PythonTaskSpec) -> TaskDefinition {
    TaskDefinition::new(task_from_spec(spec))
}

fn python_task_spec_to_json(spec: &PythonTaskSpec) -> Value {
    json!({
        "name": spec.name,
        "connection_plugin_name": spec.connection_plugin_name,
        "processors": spec.processor_names,
        "retry": retry_config_to_json(spec.retry_config.as_ref()),
        "options": spec.options,
        "supports_dry_run": spec.supports_dry_run,
        "sub_tasks": spec.sub_tasks.iter().map(python_task_spec_to_json).collect::<Vec<_>>(),
    })
}

fn python_task_spec_to_py_dict<'py>(
    py: Python<'py>,
    spec: &PythonTaskSpec,
) -> PyResult<Bound<'py, PyDict>> {
    let task = PyDict::new(py);
    task.set_item("name", &spec.name)?;
    match &spec.connection_plugin_name {
        Some(connection_plugin_name) => {
            task.set_item("connection_plugin_name", connection_plugin_name)?
        }
        None => task.set_item("connection_plugin_name", py.None())?,
    }
    task.set_item("processors", &spec.processor_names)?;
    task.set_item(
        "retry",
        retry_config_to_py_object(py, spec.retry_config.as_ref())?,
    )?;
    task.set_item("supports_dry_run", spec.supports_dry_run)?;
    if let Some(options) = spec.options.as_ref() {
        task.set_item("options", json_value_to_py(py, options)?)?;
    } else {
        task.set_item("options", py.None())?;
    }
    task.set_item(
        "sub_tasks",
        spec.sub_tasks
            .iter()
            .map(|sub_task| python_task_spec_to_py_dict(py, sub_task))
            .collect::<PyResult<Vec<_>>>()?,
    )?;
    Ok(task)
}

fn retry_config_to_py_object(
    py: Python<'_>,
    retry_config: Option<&RetryConfig>,
) -> PyResult<Py<PyAny>> {
    let value = retry_config_to_json(retry_config);
    json_value_to_py(py, &value)
}

fn reject_misplaced_retry_metadata(info: &Bound<'_, PyDict>) -> PyResult<()> {
    if info.contains("allow_retries")? {
        return Err(PyValueError::new_err(
            "python task metadata field 'allow_retries'; did you mean retry['allow']?",
        ));
    }
    if info.contains("max_task_attempts")? {
        return Err(PyValueError::new_err(
            "python task metadata field 'max_task_attempts'; did you mean retry['max_attempts']?",
        ));
    }
    if info.contains("delay_ms")? {
        return Err(PyValueError::new_err(
            "python task metadata field 'delay_ms'; did you mean retry['delay_ms']?",
        ));
    }
    Ok(())
}

fn extract_retry_config_from_metadata(info: &Bound<'_, PyDict>) -> PyResult<Option<RetryConfig>> {
    let Some(retry) = info.get_item("retry")? else {
        return Ok(None);
    };
    if retry.is_none() {
        return Ok(None);
    }
    let retry = retry.cast::<PyDict>().map_err(|_| {
        PyValueError::new_err("python task metadata field 'retry' must be a dict or None")
    })?;

    let allow = if let Some(value) = retry.get_item("allow")? {
        if value.is_none() {
            None
        } else {
            Some(value.extract::<bool>()?)
        }
    } else {
        None
    };
    let max_attempts = if let Some(value) = retry.get_item("max_attempts")? {
        if value.is_none() {
            None
        } else {
            if value.is_instance_of::<pyo3::types::PyBool>() {
                return Err(PyValueError::new_err(
                    "python task metadata field 'retry.max_attempts' must be an integer",
                ));
            }
            let max_attempts = value.extract::<isize>()?;
            if max_attempts < 1 {
                return Err(PyValueError::new_err(
                    "python task metadata field 'retry.max_attempts' must be at least 1",
                ));
            }
            Some(max_attempts as usize)
        }
    } else {
        None
    };
    let delay_ms = if let Some(value) = retry.get_item("delay_ms")? {
        if value.is_none() {
            None
        } else {
            if value.is_instance_of::<pyo3::types::PyBool>() {
                return Err(PyValueError::new_err(
                    "python task metadata field 'retry.delay_ms' must be an integer",
                ));
            }
            let delay_ms = value.extract::<i128>()?;
            if delay_ms < 0 {
                return Err(PyValueError::new_err(
                    "python task metadata field 'retry.delay_ms' must be at least 0",
                ));
            }
            Some(delay_ms as u64)
        }
    } else {
        None
    };

    if allow.is_some() || max_attempts.is_some() || delay_ms.is_some() {
        Ok(Some(RetryConfig::new(allow, max_attempts, delay_ms)))
    } else {
        Ok(None)
    }
}

fn build_python_task_runtime_context(
    py: Python<'_>,
    current_depth: usize,
    max_depth: Option<usize>,
    current_attempt: usize,
    dry_run: bool,
    connection: Option<Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    let context = PyDict::new(py);
    context.set_item("current_depth", current_depth)?;
    context.set_item("current_attempt", current_attempt)?;
    context.set_item("dry_run", dry_run)?;
    if let Some(max_depth) = max_depth {
        context.set_item("max_depth", max_depth)?;
    } else {
        context.set_item("max_depth", py.None())?;
    }
    match connection {
        Some(connection) => context.set_item("connection", connection.bind(py))?,
        None => context.set_item("connection", py.None())?,
    }
    build_python_task_model(py, "TaskRuntimeContext", context)
}

fn build_python_task_model<'py>(
    py: Python<'py>,
    class_name: &str,
    kwargs: Bound<'py, PyDict>,
) -> PyResult<Py<PyAny>> {
    let task_module = PyModule::import(py, "genja.task")?;
    let class = task_module.getattr(class_name)?;
    Ok(class.call((), Some(&kwargs))?.unbind())
}

pub(crate) fn host_to_py_dict<'py>(py: Python<'py>, host: &Host) -> PyResult<Bound<'py, PyDict>> {
    let payload = PyDict::new(py);
    payload.set_item("hostname", host.hostname())?;
    payload.set_item("port", host.port())?;
    payload.set_item("username", host.username())?;
    payload.set_item("password", host.password())?;
    payload.set_item("platform", host.platform())?;

    if let Some(data) = host.data() {
        payload.set_item("data", json_value_to_py(py, data)?)?;
    } else {
        payload.set_item("data", py.None())?;
    }

    Ok(payload)
}

pub(crate) fn hosts_to_py_dict(py: Python<'_>, hosts: &Hosts) -> PyResult<Py<PyAny>> {
    let payload = PyDict::new(py);
    for (host_id, host) in hosts.iter() {
        payload.set_item(host_id.as_str(), host_to_py_dict(py, host)?)?;
    }
    Ok(payload.into_any().unbind())
}

pub(crate) fn python_host_to_rust_host(obj: Bound<'_, PyAny>) -> PyResult<Host> {
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
        obj
    };

    let json_module = PyModule::import(normalized.py(), "json")?;
    let dumped: String = json_module
        .call_method1("dumps", (normalized,))?
        .extract()?;
    serde_json::from_str(&dumped)
        .map_err(|err| PyValueError::new_err(format!("invalid host payload: {err}")))
}

pub(crate) fn python_hosts_to_rust_hosts(obj: Bound<'_, PyAny>) -> PyResult<Hosts> {
    let dict = obj.cast::<PyDict>().map_err(|_| {
        PyValueError::new_err("hosts must be a dict mapping host id to host payload")
    })?;

    let mut hosts = Hosts::new();
    for (host_id, host_obj) in dict.iter() {
        let host_id: String = host_id.extract()?;
        let host = python_host_to_rust_host(host_obj)?;
        hosts.add_host(host_id, host);
    }
    Ok(hosts)
}

fn python_task_error(err: PyErr) -> TaskError {
    TaskError::new(std::io::Error::other(err.to_string()))
}

struct RuntimeTaskWrapper {
    inner: Arc<dyn Task>,
}

impl TaskInfo for RuntimeTaskWrapper {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn connection_plugin_name(&self) -> Option<&str> {
        self.inner.connection_plugin_name()
    }

    fn get_connection_key(&self, hostname: &str) -> Option<ConnectionKey> {
        self.inner.get_connection_key(hostname)
    }

    fn processor_names(&self) -> Vec<&str> {
        self.inner.processor_names()
    }

    fn options(&self) -> Option<&Value> {
        self.inner.options()
    }

    fn retry_config(&self) -> Option<&RetryConfig> {
        self.inner.retry_config()
    }

    fn supports_dry_run(&self) -> bool {
        self.inner.supports_dry_run()
    }
}

#[async_trait]
impl Task for RuntimeTaskWrapper {
    fn start(
        &self,
        host: &Host,
        context: &::genja_core::task::BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        self.inner.start(host, context)
    }

    async fn start_async(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        self.inner.start_async(host, context).await
    }

    fn dry_run(
        &self,
        host: &Host,
        context: &::genja_core::task::BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        self.inner.dry_run(host, context)
    }

    async fn dry_run_async(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        self.inner.dry_run_async(host, context).await
    }

    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        self.inner.sub_tasks()
    }

    fn execution_mode(&self) -> TaskExecutionMode {
        self.inner.execution_mode()
    }
}

pub(crate) fn py_any_to_json_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    normalize_python_json_payload(obj, "invalid json payload")
}

fn normalize_python_json_payload(obj: &Bound<'_, PyAny>, error_prefix: &str) -> PyResult<Value> {
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
    serde_json::from_str(&dumped)
        .map_err(|err| PyValueError::new_err(format!("{error_prefix}: {err}")))
}

fn normalize_python_task_results_payload(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
    if obj.hasattr("task_name")? && obj.hasattr("host_summary")? && obj.hasattr("task_summary")? {
        let kwargs = PyDict::new(obj.py());
        kwargs.set_item("raw", true)?;
        let raw_payload = obj.call_method("to_dict", (), Some(&kwargs))?;
        return normalize_python_json_payload(&raw_payload, "invalid python task results");
    }

    normalize_python_json_payload(obj, "invalid python task results")
}

pub(crate) fn json_value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    let dumped = serde_json::to_string(value)
        .map_err(|err| PyValueError::new_err(format!("failed to serialize value: {err}")))?;
    let json_module = PyModule::import(py, "json")?;
    Ok(json_module.call_method1("loads", (dumped,))?.unbind())
}

fn json_to_task_results(value: &Value) -> PyResult<TaskResults> {
    let task_name = value
        .get("task_name")
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("task results payload is missing 'task_name'"))?;
    let mut results = TaskResults::new(task_name);

    if let Some(summary) = value.get("summary").and_then(Value::as_str) {
        results = results.with_summary(summary);
    }
    if let Some(started_at) = value.get("started_at").and_then(Value::as_str) {
        let parsed = humantime::parse_rfc3339(started_at).map_err(|err| {
            PyValueError::new_err(format!(
                "invalid task results started_at '{started_at}': {err}"
            ))
        })?;
        results = results.with_started_at(parsed);
    }
    if let Some(finished_at) = value.get("finished_at").and_then(Value::as_str) {
        let parsed = humantime::parse_rfc3339(finished_at).map_err(|err| {
            PyValueError::new_err(format!(
                "invalid task results finished_at '{finished_at}': {err}"
            ))
        })?;
        results = results.with_finished_at(parsed);
    }
    if let Some(duration_ns) = value.get("duration_ns").and_then(Value::as_u64) {
        results = results.with_duration_ns(duration_ns as u128);
    } else if let Some(duration_ms) = value.get("duration_ms").and_then(Value::as_u64) {
        results = results.with_duration_ms(duration_ms as u128);
    }

    if let Some(hosts) = value.get("hosts").and_then(Value::as_object) {
        for (hostname, result) in hosts {
            results.insert_host_result(hostname, host_task_result_from_payload(result)?);
        }
    }

    if let Some(sub_tasks) = value.get("sub_tasks").and_then(Value::as_object) {
        for (task_name, sub_results) in sub_tasks {
            results.insert_sub_task(task_name, json_to_task_results(sub_results)?);
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ::genja_core::task::{TaskFailure, TaskFailureKind};
    use pyo3::types::PyModule;
    use pyo3::types::PyTuple;

    fn init_python() {
        crate::init_embedded_python();
        Python::attach(|py| {
            let sys = PyModule::import(py, "sys").expect("sys module should import");
            let modules = sys.getattr("modules").expect("sys.modules should exist");
            let genja = PyModule::from_code(
                py,
                pyo3::ffi::c_str!("__path__ = []\n"),
                pyo3::ffi::c_str!("genja/__init__.py"),
                pyo3::ffi::c_str!("genja"),
            )
            .expect("genja stub should build");
            let task = PyModule::from_code(
                py,
                pyo3::ffi::c_str!(
                    "class _Model:\n    def __init__(self, **kwargs):\n        self.__dict__.update(kwargs)\n\n    def to_dict(self):\n        return dict(self.__dict__)\n\nclass TaskInfo(_Model):\n    pass\n\nclass Host(_Model):\n    pass\n\nclass TaskRuntimeContext(_Model):\n    def has_connection(self):\n        return self.connection is not None\n"
                ),
                pyo3::ffi::c_str!("genja/task.py"),
                pyo3::ffi::c_str!("genja.task"),
            )
            .expect("task stub should build");
            genja
                .add("task", &task)
                .expect("task module should attach to package");
            modules
                .set_item("genja", &genja)
                .expect("genja stub should register");
            modules
                .set_item("genja.task", &task)
                .expect("task stub should register");
        });
    }

    fn make_task_class<'py>(
        py: Python<'py>,
        name: &str,
        connection_plugin_name: Option<&str>,
        sub_tasks: &[Bound<'py, PyAny>],
        execution_mode: PythonTaskExecutionMode,
    ) -> PyResult<Bound<'py, PyAny>> {
        let builtins = PyModule::import(py, "builtins")?;
        let type_fn = builtins.getattr("type")?;
        let object = builtins.getattr("object")?;
        let bases = PyTuple::new(py, [object])?;

        let info = PyDict::new(py);
        info.set_item("name", name)?;
        match connection_plugin_name {
            Some(connection_plugin_name) => {
                info.set_item("connection_plugin_name", connection_plugin_name)?
            }
            None => info.set_item("connection_plugin_name", py.None())?,
        }
        info.set_item("retry", py.None())?;
        info.set_item("supports_dry_run", false)?;
        info.set_item("sub_tasks", sub_tasks)?;

        let attrs = PyDict::new(py);
        attrs.set_item("__genja_task_info__", info)?;
        match execution_mode {
            PythonTaskExecutionMode::Blocking => {
                attrs.set_item(
                    "start",
                    PyModule::from_code(
                        py,
                        pyo3::ffi::c_str!("def start(self, task, host, context):\n    return {'status': 'passed'}\n"),
                        pyo3::ffi::c_str!("tests/_task_stub.py"),
                        pyo3::ffi::c_str!("tests._task_stub"),
                    )?
                    .getattr("start")?,
                )?;
            }
            PythonTaskExecutionMode::Async => {
                attrs.set_item(
                    "start_async",
                    PyModule::from_code(
                        py,
                        pyo3::ffi::c_str!("async def start_async(self, task, host, context):\n    return {'status': 'passed'}\n"),
                        pyo3::ffi::c_str!("tests/_task_stub.py"),
                        pyo3::ffi::c_str!("tests._task_stub"),
                    )?
                    .getattr("start_async")?,
                )?;
            }
        }

        type_fn.call1((name, bases, attrs))
    }

    #[test]
    fn python_result_to_host_task_result_round_trips_success_dict() {
        init_python();
        Python::attach(|py| {
            let result = PyDict::new(py);
            result.set_item("status", "passed").unwrap();
            result.set_item("changed", true).unwrap();
            result.set_item("summary", "backup complete").unwrap();
            result
                .set_item("warnings", vec!["using fallback path"])
                .unwrap();
            result
                .set_item(
                    "messages",
                    vec![
                        json_value_to_py(
                            py,
                            &json!({
                                "level": "info",
                                "text": "backup complete",
                                "code": "BACKUP_DONE",
                                "timestamp": "2026-04-29T12:00:00Z",
                            }),
                        )
                        .unwrap(),
                    ],
                )
                .unwrap();
            result
                .set_item(
                    "metadata",
                    json_value_to_py(py, &json!({"backup_file": "/tmp/router1.cfg"})).unwrap(),
                )
                .unwrap();

            let host_result = python_result_to_host_task_result(result.into_any())
                .expect("success result should convert");
            let data = host_task_result_to_json(&host_result);

            assert!(host_result.is_passed());
            assert!(data["outcome"]["Passed"].is_object());
            assert_eq!(data["outcome"]["Passed"]["changed"], true);
            assert_eq!(data["outcome"]["Passed"]["summary"], "backup complete");
            assert_eq!(
                data["outcome"]["Passed"]["warnings"],
                json!(["using fallback path"])
            );
            assert_eq!(
                data["outcome"]["Passed"]["messages"][0]["code"],
                "BACKUP_DONE"
            );
            assert_eq!(
                data["outcome"]["Passed"]["metadata"]["backup_file"],
                "/tmp/router1.cfg"
            );
        });
    }

    #[test]
    fn python_result_to_host_task_result_rejects_unknown_status() {
        init_python();
        Python::attach(|py| {
            let result = PyDict::new(py);
            result.set_item("status", "unknown").unwrap();

            let err = python_result_to_host_task_result(result.into_any())
                .expect_err("unknown status should fail");
            assert!(
                err.to_string()
                    .contains("unsupported python task result status 'unknown'")
            );
        });
    }

    #[test]
    fn python_host_to_rust_host_converts_dict_payload() {
        init_python();
        Python::attach(|py| {
            let host = PyDict::new(py);
            host.set_item("hostname", "10.0.0.1").unwrap();
            host.set_item("port", 22).unwrap();
            host.set_item("username", "admin").unwrap();
            host.set_item("password", "secret").unwrap();
            host.set_item("platform", "ios").unwrap();
            host.set_item(
                "data",
                json_value_to_py(py, &json!({"site": "lab"})).unwrap(),
            )
            .unwrap();

            let converted =
                python_host_to_rust_host(host.into_any()).expect("host payload should convert");

            assert_eq!(converted.hostname(), Some("10.0.0.1"));
            assert_eq!(converted.port(), Some(22));
            assert_eq!(converted.username(), Some("admin"));
            assert_eq!(converted.password(), Some("secret"));
            assert_eq!(converted.platform(), Some("ios"));
            assert_eq!(
                converted.data().map(|value| &**value),
                Some(&json!({"site": "lab"}))
            );
        });
    }

    #[test]
    fn extract_python_task_spec_extracts_nested_sub_task_metadata() {
        init_python();
        Python::attach(|py| {
            let verify = make_task_class(
                py,
                "verify_backup",
                Some("ssh"),
                &[],
                PythonTaskExecutionMode::Blocking,
            )
            .expect("sub task class should be created");
            let backup = make_task_class(
                py,
                "backup_config",
                Some("ssh"),
                &[verify],
                PythonTaskExecutionMode::Blocking,
            )
            .expect("parent task class should be created");

            let spec = extract_python_task_spec(backup).expect("task spec should extract");

            assert_eq!(spec.name, "backup_config");
            assert_eq!(spec.connection_plugin_name.as_deref(), Some("ssh"));
            assert_eq!(spec.retry_config, None);
            assert_eq!(spec.options, None);
            assert_eq!(spec.sub_tasks.len(), 1);
            assert_eq!(spec.sub_tasks[0].name, "verify_backup");
            assert_eq!(
                spec.sub_tasks[0].connection_plugin_name.as_deref(),
                Some("ssh")
            );
        });
    }

    #[test]
    fn extract_python_task_spec_extracts_options_payload() {
        init_python();
        Python::attach(|py| {
            let task = make_task_class(
                py,
                "backup_config",
                Some("ssh"),
                &[],
                PythonTaskExecutionMode::Blocking,
            )
            .expect("task class should be created");
            task.getattr("__genja_task_info__")
                .expect("task metadata should exist")
                .cast::<PyDict>()
                .expect("task metadata should be a dict")
                .set_item(
                    "options",
                    json_value_to_py(
                        py,
                        &json!({"backup_path": "/tmp/configs", "compress": true}),
                    )
                    .unwrap(),
                )
                .unwrap();

            let spec = extract_python_task_spec(task).expect("task spec should extract");

            assert_eq!(
                spec.options,
                Some(json!({"backup_path": "/tmp/configs", "compress": true}))
            );
        });
    }

    #[test]
    fn extract_python_task_spec_extracts_retry_overrides() {
        init_python();
        Python::attach(|py| {
            let task = make_task_class(
                py,
                "backup_config",
                Some("ssh"),
                &[],
                PythonTaskExecutionMode::Blocking,
            )
            .expect("task class should be created");
            task.getattr("__genja_task_info__")
                .expect("task metadata should exist")
                .cast::<PyDict>()
                .expect("task metadata should be a dict")
                .set_item("retry", {
                    let retry = PyDict::new(py);
                    retry.set_item("allow", true).unwrap();
                    retry.set_item("max_attempts", 3).unwrap();
                    retry.set_item("delay_ms", 500).unwrap();
                    retry
                })
                .unwrap();

            let spec = extract_python_task_spec(task).expect("task spec should extract");

            let retry_config = spec.retry_config.expect("retry config should be extracted");
            assert_eq!(retry_config.allow(), Some(true));
            assert_eq!(retry_config.max_attempts(), Some(3));
            assert_eq!(retry_config.delay_ms(), Some(500));
        });
    }

    #[test]
    fn extract_python_task_spec_rejects_misplaced_retry_fields() {
        init_python();
        Python::attach(|py| {
            let task = make_task_class(
                py,
                "backup_config",
                Some("ssh"),
                &[],
                PythonTaskExecutionMode::Blocking,
            )
            .expect("task class should be created");
            task.getattr("__genja_task_info__")
                .expect("task metadata should exist")
                .cast::<PyDict>()
                .expect("task metadata should be a dict")
                .set_item("delay_ms", 500)
                .unwrap();

            let err = extract_python_task_spec(task)
                .err()
                .expect("misplaced retry field should fail");
            assert!(err.to_string().contains("did you mean retry['delay_ms']?"));
        });
    }

    #[test]
    fn extract_python_task_spec_rejects_empty_connection_plugin_name() {
        init_python();
        Python::attach(|py| {
            let task = make_task_class(
                py,
                "backup_config",
                Some(""),
                &[],
                PythonTaskExecutionMode::Blocking,
            )
            .expect("task class should be created");

            let err = extract_python_task_spec(task)
                .err()
                .expect("empty plugin should fail");
            assert!(
                err.to_string().contains(
                    "python task metadata field 'connection_plugin_name' must not be empty"
                )
            );
        });
    }

    #[test]
    fn extract_python_task_spec_allows_missing_connection_plugin_name() {
        init_python();
        Python::attach(|py| {
            let task = make_task_class(
                py,
                "backup_config",
                None,
                &[],
                PythonTaskExecutionMode::Blocking,
            )
            .expect("task class should be created");

            let spec = extract_python_task_spec(task).expect("task spec should extract");

            assert_eq!(spec.connection_plugin_name, None);
        });
    }

    #[test]
    fn register_adds_task_classes_to_module() {
        init_python();
        Python::attach(|py| {
            let module =
                PyModule::new(py, "test_task_module").expect("test module should be created");

            register(&module).expect("task classes should register");

            assert!(module.getattr("HostTaskResult").is_ok());
            assert!(module.getattr("TaskDefinition").is_ok());
            assert!(module.getattr("TaskResults").is_ok());
        });
    }

    #[test]
    fn task_definition_run_on_host_executes_async_python_body() {
        init_python();
        Python::attach(|py| {
            let task_class = PyModule::import(py, "tests.fixtures.task_definitions")
                .and_then(|module| module.getattr("AsyncRuntimeTask"))
                .expect("fixture task class should import");

            let task_definition = PyTaskDefinition::from_python_class(task_class)
                .expect("task definition should build");
            let host = {
                let payload = PyDict::new(py);
                payload.set_item("hostname", "router1").unwrap();
                payload.set_item("platform", "ios").unwrap();
                payload
            };

            let result = task_definition
                .run_on_host(host.into_any(), None, None)
                .expect("async task should execute");
            assert_eq!(result.passed_hosts(), vec!["router1".to_string()]);
            let host_result = result
                .inner
                .host_result("router1")
                .expect("router1 result should exist");
            assert!(host_result.is_passed());
            let success = host_result.success().expect("host result should be passed");
            assert!(success.changed());
            assert_eq!(success.summary(), Some("async handled router1"));
            assert_eq!(success.metadata().unwrap()["has_connection"], json!(false));
        });
    }

    #[test]
    fn python_result_to_task_results_accepts_py_task_results_raw_shape() {
        init_python();
        Python::attach(|py| {
            let mut results = TaskResults::new("backup");
            results.insert_host_result(
                "router1",
                HostTaskResult::failed(
                    TaskFailure::capture("boom").with_kind(TaskFailureKind::External),
                ),
            );
            let payload = Py::new(
                py,
                PyTaskResults {
                    inner: results.clone(),
                },
            )
            .expect("task results wrapper should build");

            let round_tripped = python_result_to_task_results(payload.bind(py).clone().into_any())
                .expect("py task results should normalize through raw payload");

            let host_result = round_tripped
                .host_result("router1")
                .expect("router1 result should exist");
            assert!(host_result.failure().is_some());
            assert!(matches!(
                host_result.failure().map(|failure| failure.kind()),
                Some(TaskFailureKind::External)
            ));
        });
    }
}
