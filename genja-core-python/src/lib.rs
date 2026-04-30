use ::genja::Genja as RuntimeGenja;
use ::genja_core::settings::{
    CoreConfig, InventoryConfig, LoggingConfig, OptionsConfig, RunnerConfig, SSHConfig,
};
use ::genja_core::inventory::{ConnectionKey, Host, Hosts, Inventory};
use ::genja_core::Settings;
use ::genja_core::task::{
    HostTaskResult, MessageLevel, SubTasks, Task, TaskDefinition, TaskError, TaskFailure,
    TaskFailureKind, TaskInfo, TaskMessage, TaskResults, TaskResultsSummary, TaskSkip,
    TaskSuccess,
};
use humantime::format_rfc3339;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::SystemTime;

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

#[pyclass(name = "HostTaskResult")]
#[derive(Clone)]
struct PyHostTaskResult {
    inner: HostTaskResult,
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
        match self.inner {
            HostTaskResult::Passed(_) => "passed",
            HostTaskResult::Failed(_) => "failed",
            HostTaskResult::Skipped(_) => "skipped",
        }
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
    plugin_name: String,
    py_task_class: Arc<Py<PyAny>>,
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

    fn plugin_name(&self) -> &str {
        &self.spec.plugin_name
    }

    fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
        ConnectionKey::new(hostname, self.plugin_name())
    }

    fn options(&self) -> Option<&Value> {
        None
    }
}

impl SubTasks for PythonBackedTask {
    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        self.sub_tasks.clone()
    }
}

impl Task for PythonBackedTask {
    fn start(&self, host: &Host) -> Result<HostTaskResult, TaskError> {
        Python::with_gil(|py| {
            let class = self.spec.py_task_class.as_ref().bind(py);
            let instance = class.call0().map_err(python_task_error)?;
            let task_payload =
                build_python_task_model(
                    py,
                    "TaskInfo",
                    python_task_spec_to_py_dict(py, &self.spec).map_err(python_task_error)?,
                )
                .map_err(python_task_error)?;
            let host_payload =
                build_python_task_model(
                    py,
                    "Host",
                    host_to_py_dict(py, host).map_err(python_task_error)?,
                )
                .map_err(python_task_error)?;
            let context_payload = {
                let context = PyDict::new(py);
                context.set_item("current_depth", 0).map_err(python_task_error)?;
                context.set_item("max_depth", py.None()).map_err(python_task_error)?;
                build_python_task_model(py, "TaskContext", context).map_err(python_task_error)?
            };

            let result = instance
                .call_method1("run", (task_payload, host_payload, context_payload))
                .map_err(python_task_error)?;

            python_result_to_host_task_result(result).map_err(python_task_error)
        })
    }
}

#[pyclass(name = "TaskDefinition")]
#[derive(Clone)]
struct PyTaskDefinition {
    inner: TaskDefinition,
    spec: PythonTaskSpec,
}

#[pymethods]
impl PyTaskDefinition {
    #[staticmethod]
    fn from_python_class(py_task_class: Bound<'_, PyAny>) -> PyResult<Self> {
        let spec = extract_python_task_spec(py_task_class)?;
        Ok(Self {
            inner: task_definition_from_spec(&spec),
            spec,
        })
    }

    #[getter]
    fn name(&self) -> String {
        self.spec.name.clone()
    }

    #[getter]
    fn plugin_name(&self) -> String {
        self.spec.plugin_name.clone()
    }

    #[getter]
    fn sub_tasks(&self) -> Vec<Self> {
        self.spec
            .sub_tasks
            .iter()
            .cloned()
            .map(|spec| Self {
                inner: task_definition_from_spec(&spec),
                spec,
            })
            .collect()
    }

    fn to_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        json_value_to_py(py, &python_task_spec_to_json(&self.spec))
    }

    fn run_on_host(&self, host: Bound<'_, PyAny>) -> PyResult<PyHostTaskResult> {
        let host = python_host_to_rust_host(host)?;
        let result = self
            .inner
            .as_task()
            .start(&host)
            .map_err(|err| PyValueError::new_err(format!("python task execution failed: {err}")))?;
        Ok(PyHostTaskResult { inner: result })
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskDefinition(name={:?}, plugin_name={:?}, sub_tasks={})",
            self.spec.name,
            self.spec.plugin_name,
            self.spec.sub_tasks.len()
        )
    }
}

#[pyclass(name = "TaskResults")]
#[derive(Clone)]
struct PyTaskResults {
    inner: TaskResults,
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
        json_value_to_py(py, &task_results_summary_to_json(&self.inner.task_summary()))
    }

    #[pyo3(signature = (*, raw=false))]
    fn to_dict(&self, py: Python<'_>, raw: bool) -> PyResult<Py<PyAny>> {
        let dumped = if raw {
            self.inner.to_raw_json_string()
        } else {
            self.inner.to_json_string()
        }
        .map_err(|err| PyValueError::new_err(format!("failed to serialize task results: {err}")))?;

        let value: Value = serde_json::from_str(&dumped)
            .map_err(|err| PyValueError::new_err(format!("failed to parse task results json: {err}")))?;
        json_value_to_py(py, &value)
    }

    #[pyo3(signature = (*, raw=false, pretty=false))]
    fn to_json(&self, raw: bool, pretty: bool) -> PyResult<String> {
        match (raw, pretty) {
            (true, true) => self.inner.to_raw_pretty_json_string(),
            (true, false) => self.inner.to_raw_json_string(),
            (false, true) => self.inner.to_pretty_json_string(),
            (false, false) => self.inner.to_json_string(),
        }
        .map_err(|err| PyValueError::new_err(format!("failed to serialize task results: {err}")))
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

#[pyclass(name = "Genja")]
#[derive(Clone)]
struct PyGenja {
    inner: RuntimeGenja,
}

#[pymethods]
impl PyGenja {
    #[staticmethod]
    #[pyo3(signature = (hosts, settings=None))]
    fn from_hosts(hosts: Bound<'_, PyAny>, settings: Option<PyRef<'_, PySettings>>) -> PyResult<Self> {
        let inventory = python_hosts_to_inventory(hosts)?;
        let inner = if let Some(settings) = settings {
            RuntimeGenja::builder(inventory)
                .with_settings(settings.inner.clone())
                .build()
                .map_err(|err| PyValueError::new_err(format!("failed to build Genja runtime: {err}")))?
        } else {
            RuntimeGenja::from_inventory(inventory)
        };
        Ok(Self { inner })
    }

    #[staticmethod]
    fn from_settings_file(path: &str) -> PyResult<Self> {
        let inner = RuntimeGenja::from_settings_file(path).map_err(|err| {
            PyValueError::new_err(format!(
                "failed to build Genja runtime from settings file {path}: {err}"
            ))
        })?;
        Ok(Self { inner })
    }

    fn with_runner(&self, runner: &str) -> PyResult<Self> {
        let inner = self
            .inner
            .with_runner(runner)
            .map_err(|err| PyValueError::new_err(format!("failed to select runner {runner}: {err}")))?;
        Ok(Self { inner })
    }

    #[pyo3(signature = (task_class, max_depth=None))]
    fn run_task(
        &self,
        task_class: Bound<'_, PyAny>,
        max_depth: Option<usize>,
    ) -> PyResult<PyTaskResults> {
        let spec = extract_python_task_spec(task_class)?;
        let task = task_from_spec(&spec);
        let max_depth =
            max_depth.unwrap_or_else(|| self.inner.settings().runner().max_task_depth());
        let inner = self.inner.run(task, max_depth).map_err(|err| {
            PyValueError::new_err(format!("failed to run task through Genja runtime: {err}"))
        })?;
        Ok(PyTaskResults { inner })
    }

    fn __repr__(&self) -> String {
        format!(
            "Genja(plugins_loaded={}, inventory_loaded={})",
            self.inner.plugins_loaded(),
            self.inner.inventory_loaded()
        )
    }
}

fn python_result_to_host_task_result(obj: Bound<'_, PyAny>) -> PyResult<HostTaskResult> {
    let normalized = if obj.hasattr("model_dump")? {
        obj.call_method("model_dump", (), Some(&PyDict::from_sequence(&[("mode", "json")].into_pyobject(obj.py())?)?))?
    } else if obj.hasattr("to_dict")? {
        obj.call_method0("to_dict")?
    } else {
        obj
    };

    let json_module = PyModule::import(normalized.py(), "json")?;
    let dumped: String = json_module.call_method1("dumps", (normalized,))?.extract()?;
    let value: Value = serde_json::from_str(&dumped)
        .map_err(|err| PyValueError::new_err(format!("invalid python task result: {err}")))?;

    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| PyValueError::new_err("python task result is missing 'status'"))?;

    match status {
        "passed" => Ok(HostTaskResult::passed(json_to_task_success(&value)?)),
        "failed" => Ok(HostTaskResult::failed(json_to_task_failure(&value)?)),
        "skipped" => Ok(HostTaskResult::Skipped(json_to_task_skip(&value))),
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
            PyValueError::new_err(format!("invalid task message timestamp '{timestamp}': {err}"))
        })?;
        message = message.with_timestamp(parsed);
    }

    Ok(message)
}

fn parse_message_level(level: &str) -> PyResult<MessageLevel> {
    match level {
        "info" => Ok(MessageLevel::Info),
        "warning" | "warn" => Ok(MessageLevel::Warning),
        "error" => Ok(MessageLevel::Error),
        "debug" => Ok(MessageLevel::Debug),
        other => Err(PyValueError::new_err(format!(
            "unsupported task message level '{other}'"
        ))),
    }
}

fn parse_failure_kind(kind: &str) -> PyResult<TaskFailureKind> {
    match kind {
        "connection" => Ok(TaskFailureKind::Connection),
        "authentication" => Ok(TaskFailureKind::Authentication),
        "validation" => Ok(TaskFailureKind::Validation),
        "timeout" => Ok(TaskFailureKind::Timeout),
        "command" => Ok(TaskFailureKind::Command),
        "unsupported" => Ok(TaskFailureKind::Unsupported),
        "internal" => Ok(TaskFailureKind::Internal),
        "external" => Ok(TaskFailureKind::External),
        other => Err(PyValueError::new_err(format!(
            "unsupported task failure kind '{other}'"
        ))),
    }
}

fn host_task_result_to_json(result: &HostTaskResult) -> Value {
    match result {
        HostTaskResult::Passed(success) => json!({
            "status": "passed",
            "result": success.result(),
            "changed": success.changed(),
            "diff": success.diff(),
            "summary": success.summary(),
            "warnings": success.warnings(),
            "messages": success.messages().iter().map(task_message_to_json).collect::<Vec<_>>(),
            "metadata": success.metadata(),
        }),
        HostTaskResult::Failed(failure) => json!({
            "status": "failed",
            "kind": failure_kind_to_str(failure.kind()),
            "message": failure.message(),
            "retryable": failure.retryable(),
            "details": failure.details(),
            "warnings": failure.warnings(),
            "messages": failure.messages().iter().map(task_message_to_json).collect::<Vec<_>>(),
        }),
        HostTaskResult::Skipped(skip) => json!({
            "status": "skipped",
            "reason": skip.reason(),
            "message": skip.message(),
        }),
    }
}

fn task_results_summary_to_json(summary: &TaskResultsSummary) -> Value {
    let sub_tasks = summary
        .sub_tasks()
        .iter()
        .map(|(task_name, sub_summary)| {
            (task_name.to_string(), task_results_summary_to_json(sub_summary))
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

fn task_message_to_json(message: &TaskMessage) -> Value {
    json!({
        "level": message_level_to_str(message.level()),
        "text": message.text(),
        "code": message.code(),
        "timestamp": message.timestamp().map(format_timestamp),
    })
}

fn message_level_to_str(level: &MessageLevel) -> &'static str {
    match level {
        MessageLevel::Info => "info",
        MessageLevel::Warning => "warning",
        MessageLevel::Error => "error",
        MessageLevel::Debug => "debug",
    }
}

fn failure_kind_to_str(kind: &TaskFailureKind) -> &'static str {
    match kind {
        TaskFailureKind::Connection => "connection",
        TaskFailureKind::Authentication => "authentication",
        TaskFailureKind::Validation => "validation",
        TaskFailureKind::Timeout => "timeout",
        TaskFailureKind::Command => "command",
        TaskFailureKind::Unsupported => "unsupported",
        TaskFailureKind::Internal => "internal",
        TaskFailureKind::External => "external",
    }
}

fn format_timestamp(timestamp: SystemTime) -> String {
    format_rfc3339(timestamp).to_string()
}

fn extract_python_task_spec(py_task_class: Bound<'_, PyAny>) -> PyResult<PythonTaskSpec> {
    let class_dict = py_task_class.getattr("__dict__")?;
    let info_obj = class_dict
        .call_method1("get", ("__genja_task_info__",))?
        .extract::<Option<Py<PyAny>>>()?
        .map(|value| value.bind(py_task_class.py()).clone())
        .ok_or_else(|| PyValueError::new_err("python task class is missing __genja_task_info__"))?;
    let info: Bound<'_, PyDict> = info_obj.downcast_into()?;

    let name: String = info
        .get_item("name")?
        .ok_or_else(|| PyValueError::new_err("python task metadata is missing 'name'"))?
        .extract()?;
    if name.trim().is_empty() {
        return Err(PyValueError::new_err(
            "python task metadata field 'name' must not be empty",
        ));
    }
    let plugin_name: String = info
        .get_item("plugin_name")?
        .ok_or_else(|| PyValueError::new_err("python task metadata is missing 'plugin_name'"))?
        .extract()?;
    if plugin_name.trim().is_empty() {
        return Err(PyValueError::new_err(
            "python task metadata field 'plugin_name' must not be empty",
        ));
    }

    let mut sub_tasks = Vec::new();
    if let Some(sub_task) = info.get_item("sub_task")? {
        if !sub_task.is_none() {
            sub_tasks.push(extract_python_task_spec(sub_task)?);
        }
    }

    Ok(PythonTaskSpec {
        name,
        plugin_name,
        py_task_class: Arc::new(py_task_class.unbind()),
        sub_tasks,
    })
}

fn task_definition_from_spec(spec: &PythonTaskSpec) -> TaskDefinition {
    TaskDefinition::new(task_from_spec(spec))
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

fn python_task_spec_to_json(spec: &PythonTaskSpec) -> Value {
    json!({
        "name": spec.name,
        "plugin_name": spec.plugin_name,
        "sub_task": spec.sub_tasks.first().map(python_task_spec_to_json),
    })
}

fn python_task_spec_to_py_dict<'py>(
    py: Python<'py>,
    spec: &PythonTaskSpec,
) -> PyResult<Bound<'py, PyDict>> {
    let task = PyDict::new(py);
    task.set_item("name", &spec.name)?;
    task.set_item("plugin_name", &spec.plugin_name)?;
    if let Some(sub_task) = spec.sub_tasks.first() {
        task.set_item("sub_task", python_task_spec_to_py_dict(py, sub_task)?)?;
    } else {
        task.set_item("sub_task", py.None())?;
    }
    Ok(task)
}

fn build_python_task_model<'py>(
    py: Python<'py>,
    class_name: &str,
    kwargs: Bound<'py, PyDict>,
) -> PyResult<Py<PyAny>> {
    let task_module = PyModule::import(py, "genja_core.task")?;
    let class = task_module.getattr(class_name)?;
    Ok(class.call((), Some(&kwargs))?.unbind())
}

fn host_to_py_dict<'py>(py: Python<'py>, host: &Host) -> PyResult<Bound<'py, PyDict>> {
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

fn python_host_to_rust_host(obj: Bound<'_, PyAny>) -> PyResult<Host> {
    let normalized = if obj.hasattr("model_dump")? {
        obj.call_method("model_dump", (), Some(&PyDict::from_sequence(&[("mode", "json")].into_pyobject(obj.py())?)?))?
    } else if obj.hasattr("to_dict")? {
        obj.call_method0("to_dict")?
    } else {
        obj
    };

    let json_module = PyModule::import(normalized.py(), "json")?;
    let dumped: String = json_module.call_method1("dumps", (normalized,))?.extract()?;
    serde_json::from_str(&dumped)
        .map_err(|err| PyValueError::new_err(format!("invalid host payload: {err}")))
}

fn python_hosts_to_inventory(obj: Bound<'_, PyAny>) -> PyResult<Inventory> {
    let dict = obj.downcast::<PyDict>().map_err(|_| {
        PyValueError::new_err("hosts must be a dict mapping host id to host payload")
    })?;

    let mut hosts = Hosts::new();
    for (host_id, host_obj) in dict.iter() {
        let host_id: String = host_id.extract()?;
        let host = python_host_to_rust_host(host_obj)?;
        hosts.add_host(host_id, host);
    }

    Ok(Inventory::builder().hosts(hosts).build())
}

fn python_task_error(err: PyErr) -> TaskError {
    TaskError::new(std::io::Error::other(err.to_string()))
}

fn json_value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
    let dumped = serde_json::to_string(value)
        .map_err(|err| PyValueError::new_err(format!("failed to serialize value: {err}")))?;
    let json_module = PyModule::import(py, "json")?;
    Ok(json_module.call_method1("loads", (dumped,))?.unbind())
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
    module.add_class::<PyHostTaskResult>()?;
    module.add_class::<PyTaskDefinition>()?;
    module.add_class::<PyTaskResults>()?;
    module.add_class::<PyGenja>()?;
    Ok(())
}
