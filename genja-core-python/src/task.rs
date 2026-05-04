use ::genja::Genja as RuntimeGenja;
use ::genja_core::inventory::{ConnectionKey, Host};
use ::genja_core::task::{
    HostTaskResult, MessageLevel, SubTasks, Task, TaskDefinition, TaskError, TaskFailure,
    TaskFailureKind, TaskInfo, TaskMessage, TaskResults, TaskResultsSummary, TaskRuntimeContext,
    TaskSkip, TaskSuccess,
};
use humantime::format_rfc3339;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyModule};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::SystemTime;

use crate::plugin_manager::python_connection_from_runtime_connection;

#[pyclass(name = "HostTaskResult")]
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
    connection_plugin_name: Option<String>,
    processor_names: Vec<String>,
    options: Option<Value>,
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
}

impl SubTasks for PythonBackedTask {
    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        self.sub_tasks.clone()
    }
}

impl Task for PythonBackedTask {
    fn start(&self, host: &Host) -> Result<HostTaskResult, TaskError> {
        self.run_python(host, 0, None, None)
    }

    fn start_with_runtime(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        let python_connection = context
            .connection()
            .and_then(|connection| connection.lock().ok())
            .and_then(|guard| python_connection_from_runtime_connection(&*guard));
        self.run_python(
            host,
            context.current_depth(),
            Some(context.max_depth()),
            python_connection,
        )
    }
}

impl PythonBackedTask {
    fn run_python(
        &self,
        host: &Host,
        current_depth: usize,
        max_depth: Option<usize>,
        connection: Option<Py<PyAny>>,
    ) -> Result<HostTaskResult, TaskError> {
        Python::with_gil(|py| {
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
            let context_payload = {
                let context = PyDict::new(py);
                context
                    .set_item("current_depth", current_depth)
                    .map_err(python_task_error)?;
                if let Some(max_depth) = max_depth {
                    context
                        .set_item("max_depth", max_depth)
                        .map_err(python_task_error)?;
                } else {
                    context
                        .set_item("max_depth", py.None())
                        .map_err(python_task_error)?;
                }
                match connection {
                    Some(connection) => context
                        .set_item("connection", connection.bind(py))
                        .map_err(python_task_error)?,
                    None => context
                        .set_item("connection", py.None())
                        .map_err(python_task_error)?,
                }
                build_python_task_model(py, "TaskRuntimeContext", context)
                    .map_err(python_task_error)?
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
pub struct PyTaskDefinition {
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
    fn connection_plugin_name(&self) -> Option<String> {
        self.spec.connection_plugin_name.clone()
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
        let result =
            self.inner.as_task().start(&host).map_err(|err| {
                PyValueError::new_err(format!("python task execution failed: {err}"))
            })?;
        Ok(PyHostTaskResult { inner: result })
    }

    fn __repr__(&self) -> String {
        format!(
            "TaskDefinition(name={:?}, connection_plugin_name={:?}, sub_tasks={})",
            self.spec.name,
            self.spec.connection_plugin_name,
            self.spec.sub_tasks.len()
        )
    }
}

#[pyclass(name = "TaskResults")]
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

pub fn run_task(
    runtime: &RuntimeGenja,
    task_class: Bound<'_, PyAny>,
    max_depth: Option<usize>,
) -> PyResult<PyTaskResults> {
    let spec = extract_python_task_spec(task_class)?;
    let task = task_from_spec(&spec);
    let max_depth = max_depth.unwrap_or_else(|| runtime.settings().runner().max_task_depth());
    let inner = runtime.run(task, max_depth).map_err(|err| {
        PyValueError::new_err(format!("failed to run task through Genja runtime: {err}"))
    })?;
    Ok(PyTaskResults { inner })
}

pub fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyHostTaskResult>()?;
    module.add_class::<PyTaskDefinition>()?;
    module.add_class::<PyTaskResults>()?;
    Ok(())
}

pub(crate) fn python_result_to_host_task_result(obj: Bound<'_, PyAny>) -> PyResult<HostTaskResult> {
    let value = normalize_python_json_payload(&obj, "invalid python task result")?;
    host_task_result_from_payload(&value)
}

pub(crate) fn python_result_to_task_results(obj: Bound<'_, PyAny>) -> PyResult<TaskResults> {
    let value = normalize_python_json_payload(&obj, "invalid python task results")?;
    json_to_task_results(&value)
}

fn host_task_result_from_payload(value: &Value) -> PyResult<HostTaskResult> {
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
        "skipped" => Ok(HostTaskResult::Skipped(json_to_task_skip(&result_value))),
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

pub(crate) fn task_message_to_json(message: &TaskMessage) -> Value {
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
    if let Some(sub_task) = info.get_item("sub_task")? {
        if !sub_task.is_none() {
            sub_tasks.push(extract_python_task_spec(sub_task)?);
        }
    }

    Ok(PythonTaskSpec {
        name,
        connection_plugin_name,
        processor_names,
        options,
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
        "connection_plugin_name": spec.connection_plugin_name,
        "processors": spec.processor_names,
        "options": spec.options,
        "sub_task": spec.sub_tasks.first().map(python_task_spec_to_json),
    })
}

fn python_task_spec_to_py_dict<'py>(
    py: Python<'py>,
    spec: &PythonTaskSpec,
) -> PyResult<Bound<'py, PyDict>> {
    let task = PyDict::new(py);
    task.set_item("name", &spec.name)?;
    match &spec.connection_plugin_name {
        Some(connection_plugin_name) => task.set_item(
            "connection_plugin_name",
            connection_plugin_name,
        )?,
        None => task.set_item("connection_plugin_name", py.None())?,
    }
    task.set_item("processors", &spec.processor_names)?;
    if let Some(options) = spec.options.as_ref() {
        task.set_item("options", json_value_to_py(py, options)?)?;
    } else {
        task.set_item("options", py.None())?;
    }
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

fn python_task_error(err: PyErr) -> TaskError {
    TaskError::new(std::io::Error::other(err.to_string()))
}

fn py_any_to_json_value(obj: &Bound<'_, PyAny>) -> PyResult<Value> {
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

fn json_value_to_py(py: Python<'_>, value: &Value) -> PyResult<Py<PyAny>> {
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
    use pyo3::types::PyTuple;
    use std::sync::Once;

    fn init_python() {
        static INIT: Once = Once::new();
        INIT.call_once(pyo3::prepare_freethreaded_python);
    }

    fn make_task_class<'py>(
        py: Python<'py>,
        name: &str,
        connection_plugin_name: Option<&str>,
        sub_task: Option<&Bound<'py, PyAny>>,
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
        if let Some(sub_task) = sub_task {
            info.set_item("sub_task", sub_task)?;
        } else {
            info.set_item("sub_task", py.None())?;
        }

        let attrs = PyDict::new(py);
        attrs.set_item("__genja_task_info__", info)?;

        type_fn.call1((name, bases, attrs))
    }

    #[test]
    fn python_result_to_host_task_result_round_trips_success_dict() {
        init_python();
        Python::with_gil(|py| {
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
                    vec![json_value_to_py(
                        py,
                        &json!({
                            "level": "info",
                            "text": "backup complete",
                            "code": "BACKUP_DONE",
                            "timestamp": "2026-04-29T12:00:00Z",
                        }),
                    )
                    .unwrap()],
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

            assert!(matches!(host_result, HostTaskResult::Passed(_)));
            assert_eq!(data["status"], "passed");
            assert_eq!(data["changed"], true);
            assert_eq!(data["summary"], "backup complete");
            assert_eq!(data["warnings"], json!(["using fallback path"]));
            assert_eq!(data["messages"][0]["code"], "BACKUP_DONE");
            assert_eq!(data["metadata"]["backup_file"], "/tmp/router1.cfg");
        });
    }

    #[test]
    fn python_result_to_host_task_result_rejects_unknown_status() {
        init_python();
        Python::with_gil(|py| {
            let result = PyDict::new(py);
            result.set_item("status", "unknown").unwrap();

            let err = python_result_to_host_task_result(result.into_any())
                .expect_err("unknown status should fail");
            assert!(err
                .to_string()
                .contains("unsupported python task result status 'unknown'"));
        });
    }

    #[test]
    fn python_host_to_rust_host_converts_dict_payload() {
        init_python();
        Python::with_gil(|py| {
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
        Python::with_gil(|py| {
            let verify = make_task_class(py, "verify_backup", Some("ssh"), None)
                .expect("sub task class should be created");
            let backup = make_task_class(py, "backup_config", Some("ssh"), Some(&verify))
                .expect("parent task class should be created");

            let spec = extract_python_task_spec(backup).expect("task spec should extract");

            assert_eq!(spec.name, "backup_config");
            assert_eq!(spec.connection_plugin_name.as_deref(), Some("ssh"));
            assert_eq!(spec.options, None);
            assert_eq!(spec.sub_tasks.len(), 1);
            assert_eq!(spec.sub_tasks[0].name, "verify_backup");
            assert_eq!(spec.sub_tasks[0].connection_plugin_name.as_deref(), Some("ssh"));
        });
    }

    #[test]
    fn extract_python_task_spec_extracts_options_payload() {
        init_python();
        Python::with_gil(|py| {
            let task = make_task_class(py, "backup_config", Some("ssh"), None)
                .expect("task class should be created");
            task.getattr("__genja_task_info__")
                .expect("task metadata should exist")
                .downcast::<PyDict>()
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
    fn extract_python_task_spec_rejects_empty_connection_plugin_name() {
        init_python();
        Python::with_gil(|py| {
            let task = make_task_class(py, "backup_config", Some(""), None)
                .expect("task class should be created");

            let err = extract_python_task_spec(task)
                .err()
                .expect("empty plugin should fail");
            assert!(err
                .to_string()
                .contains("python task metadata field 'connection_plugin_name' must not be empty"));
        });
    }

    #[test]
    fn extract_python_task_spec_allows_missing_connection_plugin_name() {
        init_python();
        Python::with_gil(|py| {
            let task = make_task_class(py, "backup_config", None, None)
                .expect("task class should be created");

            let spec = extract_python_task_spec(task).expect("task spec should extract");

            assert_eq!(spec.connection_plugin_name, None);
        });
    }

    #[test]
    fn register_adds_task_classes_to_module() {
        init_python();
        Python::with_gil(|py| {
            let module =
                PyModule::new(py, "test_task_module").expect("test module should be created");

            register(&module).expect("task classes should register");

            assert!(module.getattr("HostTaskResult").is_ok());
            assert!(module.getattr("TaskDefinition").is_ok());
            assert!(module.getattr("TaskResults").is_ok());
        });
    }
}
