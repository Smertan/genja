use crate::types::{CustomTreeMap, NatString};
use serde::Serialize;
use serde_json::Value;
use log::warn;
use std::any::type_name;
use std::error::Error;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

pub type TaskError = Arc<dyn Error + Send + Sync + 'static>;

#[derive(Debug, Clone, Default, Serialize)]
pub struct TaskResults {
    task_name: String,
    hosts: CustomTreeMap<HostTaskResult>,
    sub_tasks: CustomTreeMap<TaskResults>,
}

impl TaskResults {
    pub fn new(task_name: impl Into<String>) -> Self {
        Self {
            task_name: task_name.into(),
            hosts: CustomTreeMap::new(),
            sub_tasks: CustomTreeMap::new(),
        }
    }

    pub fn task_name(&self) -> &str {
        &self.task_name
    }

    pub fn insert_host_result<K>(&mut self, hostname: K, result: HostTaskResult)
    where
        K: Into<NatString>,
    {
        self.hosts.insert(hostname.into(), result);
    }

    pub fn host_result(&self, hostname: &str) -> Option<&HostTaskResult> {
        self.hosts.get(hostname)
    }

    pub fn host_result_mut(&mut self, hostname: &str) -> Option<&mut HostTaskResult> {
        self.hosts.get_mut(hostname)
    }

    pub fn hosts(&self) -> &CustomTreeMap<HostTaskResult> {
        &self.hosts
    }

    pub fn insert_sub_task<K>(&mut self, task_name: K, results: TaskResults)
    where
        K: Into<NatString>,
    {
        self.sub_tasks.insert(task_name.into(), results);
    }

    pub fn sub_task(&self, task_name: &str) -> Option<&TaskResults> {
        self.sub_tasks.get(task_name)
    }

    pub fn sub_task_mut(&mut self, task_name: &str) -> Option<&mut TaskResults> {
        self.sub_tasks.get_mut(task_name)
    }

    pub fn sub_tasks(&self) -> &CustomTreeMap<TaskResults> {
        &self.sub_tasks
    }

    pub fn passed_hosts(&self) -> Vec<&NatString> {
        self.hosts
            .iter()
            .filter_map(|(host, result)| result.is_passed().then_some(host))
            .collect()
    }

    pub fn failed_hosts(&self) -> Vec<&NatString> {
        self.hosts
            .iter()
            .filter_map(|(host, result)| result.is_failed().then_some(host))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum HostTaskResult {
    Passed(TaskSuccess),
    Failed(TaskFailure),
    Skipped,
}

impl HostTaskResult {
    pub fn passed(result: TaskSuccess) -> Self {
        Self::Passed(result)
    }

    pub fn failed(failure: TaskFailure) -> Self {
        Self::Failed(failure)
    }

    pub fn skipped() -> Self {
        Self::Skipped
    }

    pub fn is_passed(&self) -> bool {
        matches!(self, Self::Passed(_))
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    pub fn is_skipped(&self) -> bool {
        matches!(self, Self::Skipped)
    }

    pub fn success(&self) -> Option<&TaskSuccess> {
        match self {
            Self::Passed(success) => Some(success),
            Self::Failed(_) | Self::Skipped => None,
        }
    }

    pub fn failure(&self) -> Option<&TaskFailure> {
        match self {
            Self::Failed(failure) => Some(failure),
            Self::Passed(_) | Self::Skipped => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct TaskSuccess {
    result: Option<Value>,
    changed: bool,
    diff: Option<String>,
}

impl TaskSuccess {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_result(mut self, result: Value) -> Self {
        self.result = Some(result);
        self
    }

    pub fn with_changed(mut self, changed: bool) -> Self {
        self.changed = changed;
        self
    }

    pub fn with_diff(mut self, diff: impl Into<String>) -> Self {
        self.diff = Some(diff.into());
        self
    }

    pub fn result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    pub fn changed(&self) -> bool {
        self.changed
    }

    pub fn diff(&self) -> Option<&str> {
        self.diff.as_deref()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskFailure {
    #[serde(skip)]
    error: TaskError,
    error_type: String,
    message: String,
}

impl TaskFailure {
    pub fn new<E>(error: E) -> Self
    where
        E: Error + Send + Sync + 'static,
    {
        Self {
            error_type: type_name::<E>().to_string(),
            message: error.to_string(),
            error: Arc::new(error),
        }
    }

    pub fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: Error + 'static,
    {
        self.error.downcast_ref::<E>()
    }

    pub fn error(&self) -> &(dyn Error + Send + Sync + 'static) {
        self.error.as_ref()
    }

    pub fn error_type(&self) -> &str {
        &self.error_type
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Task metadata required for execution.
///
/// When using `#[derive(Task)]`, this trait is implemented automatically.
/// You do not need to import `TaskInfo` unless you reference it explicitly.
/// You still must implement `Task` manually to provide `start()`.
pub trait TaskInfo {
    /// Return the task's name.
    fn name(&self) -> &str;

    /// Return the task's plugin name.
    fn plugin_name(&self) -> &str;

    /// Build the task's connection key for a host.
    fn get_connection_key(&self, hostname: &str) -> crate::inventory::ConnectionKey;

    /// Return the task's options payload, if set.
    fn options(&self) -> Option<&Value>;
}

/// Sub-task provider interface.
pub trait SubTasks {
    /// Return any sub-tasks for this task.
    fn sub_tasks(&self) -> Vec<Arc<dyn Task>>;
}

/// Core task interface required for execution.
///
/// # Example
/// ```rust
/// use genja_core::task::Task;
/// use genja_core_derive::Task as TaskDerive;
///
/// #[derive(TaskDerive)]
/// struct MyTask {
///     name: String,
///     plugin_name: Option<String>,
/// }
///
/// impl Task for MyTask {
///     fn start(&self) -> Result<(), genja_core::GenjaError> {
///         Ok(())
///     }
/// }
/// ```
pub trait Task: TaskInfo + SubTasks {
    /// Start executing the task.
    fn start(&self) -> Result<(), crate::GenjaError>;

    // TODO: should have a function to execute the task with args,
    // (host: Host, args, serde_json::value))
    // fn start
    // Based on a per host basis#
}

/// A task wrapper that enforces the task trait flow.
pub struct TaskDefinition {
    inner: Box<dyn Task>,
}

impl TaskDefinition {
    /// Wrap a user-defined task that implements the Task trait.
    pub fn new<T: Task + 'static>(task: T) -> Self {
        Self {
            inner: Box::new(task),
        }
    }

    /// Borrow the inner task as a trait object.
    pub fn as_task(&self) -> &dyn Task {
        self.inner.as_ref()
    }
}

impl TaskDefinition {
    /// Execute this task and all its sub-tasks recursively up to a maximum depth.
    ///
    /// This method starts the task execution by calling the task's `start()` method,
    /// then recursively executes all sub-tasks returned by `sub_tasks()`. The recursion
    /// is limited by the `max_depth` parameter to prevent infinite loops or excessive
    /// nesting.
    ///
    /// # Parameters
    ///
    /// * `max_depth` - The maximum depth of task nesting allowed. A depth of 1 means
    ///   only the root task will execute. A depth of 2 allows the root task plus one
    ///   level of sub-tasks, and so on.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the task and all its sub-tasks execute successfully within
    /// the depth limit. Returns `Err(GenjaError)` if any task fails to execute or if
    /// the maximum depth is exceeded.
    ///
    /// # Errors
    ///
    /// * Returns `GenjaError::Message` if the task nesting exceeds `max_depth`.
    /// * Propagates any errors returned by the task's `start()` method or its sub-tasks.
    pub fn start(&self, max_depth: usize) -> Result<(), crate::GenjaError> {
        Self::start_with_depth(self.inner.as_ref(), 0, max_depth)
    }

    fn start_with_depth(
        task: &dyn Task,
        depth: usize,
        max_depth: usize,
    ) -> Result<(), crate::GenjaError> {
        if depth > max_depth {
            warn!(
                "max task depth exceeded for task '{}' at depth {} with max_depth {}",
                task.name(),
                depth,
                max_depth
            );
            return Err(crate::GenjaError::Message(format!(
                "max task depth exceeded: {}",
                max_depth
            )));
        }

        task.start()?;

        for sub in task.sub_tasks() {
            Self::start_with_depth(sub.as_ref(), depth + 1, max_depth)?;
        }

        Ok(())
    }
}

impl TaskInfo for TaskDefinition {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn plugin_name(&self) -> &str {
        self.inner.plugin_name()
    }

    fn get_connection_key(&self, hostname: &str) -> crate::inventory::ConnectionKey {
        self.inner.get_connection_key(hostname)
    }

    fn options(&self) -> Option<&Value> {
        self.inner.options()
    }
}

impl SubTasks for TaskDefinition {
    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        self.inner.sub_tasks()
    }
}

#[derive(Default)]
pub struct Tasks(Vec<TaskDefinition>);

impl Tasks {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add_task<T: Task + 'static>(&mut self, task: T) {
        self.0.push(TaskDefinition::new(task));
    }
}

impl Deref for Tasks {
    type Target = Vec<TaskDefinition>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tasks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ConnectionKey;
    use serde_json::json;
    use std::fmt;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug)]
    struct TestTaskFailureError;

    impl fmt::Display for TestTaskFailureError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "task failure test error")
        }
    }

    impl Error for TestTaskFailureError {}

    struct TestTask {
        name: &'static str,
        subs: Vec<Arc<dyn Task>>,
        counter: Arc<AtomicUsize>,
    }

    impl TaskInfo for TestTask {
        fn name(&self) -> &str {
            self.name
        }

        fn plugin_name(&self) -> &str {
            "ssh"
        }

        fn get_connection_key(&self, hostname: &str) -> ConnectionKey {
            ConnectionKey::new(hostname, "ssh")
        }

        fn options(&self) -> Option<&Value> {
            None
        }
    }

    impl SubTasks for TestTask {
        fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
            self.subs.clone()
        }
    }

    impl Task for TestTask {
        fn start(&self) -> Result<(), crate::GenjaError> {
            self.counter.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn chain(depth: usize, counter: Arc<AtomicUsize>) -> Arc<dyn Task> {
        if depth == 1 {
            return Arc::new(TestTask {
                name: "leaf",
                subs: Vec::new(),
                counter,
            });
        }

        let child = chain(depth - 1, counter.clone());
        Arc::new(TestTask {
            name: "node",
            subs: vec![child],
            counter,
        })
    }

    #[test]
    fn start_runs_within_max_depth() {
        let counter = Arc::new(AtomicUsize::new(0));
        let root = chain(3, counter.clone());
        let task = TaskDefinition::new(TestTask {
            name: "root",
            subs: vec![root],
            counter: counter.clone(),
        });

        task.start(4).expect("start should succeed");
        assert_eq!(counter.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn start_fails_when_depth_exceeds_limit() {
        let counter = Arc::new(AtomicUsize::new(0));
        let root = chain(5, counter.clone());
        let task = TaskDefinition::new(TestTask {
            name: "root",
            subs: vec![root],
            counter: counter.clone(),
        });

        let err = task.start(4).expect_err("start should fail at depth > 4");
        match err {
            crate::GenjaError::Message(msg) => {
                assert!(msg.contains("max task depth exceeded"));
            }
            _ => panic!("unexpected error variant"),
        }
        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[test]
    fn task_failure_preserves_metadata_and_supports_downcast() {
        let failure = TaskFailure::new(TestTaskFailureError);

        assert_eq!(failure.message(), "task failure test error");
        assert_eq!(failure.error().to_string(), "task failure test error");
        assert!(
            failure
                .error_type()
                .ends_with("task::tests::TestTaskFailureError")
        );
        assert!(failure.downcast_ref::<TestTaskFailureError>().is_some());
    }

    #[test]
    fn task_results_store_recursive_host_and_sub_task_results() {
        let mut root = TaskResults::new("deploy");
        root.insert_host_result(
            "router1",
            HostTaskResult::passed(
                TaskSuccess::new()
                    .with_result(json!({"deployed": true}))
                    .with_changed(true),
            ),
        );
        root.insert_host_result(
            "router2",
            HostTaskResult::failed(TaskFailure::new(TestTaskFailureError)),
        );

        let mut validate = TaskResults::new("validate");
        validate.insert_host_result(
            "router1",
            HostTaskResult::passed(TaskSuccess::new().with_result(json!({"valid": true}))),
        );
        validate.insert_host_result("router2", HostTaskResult::skipped());

        let mut collect_logs = TaskResults::new("collect_logs");
        collect_logs.insert_host_result(
            "router1",
            HostTaskResult::passed(TaskSuccess::new().with_diff("captured logs")),
        );
        collect_logs.insert_host_result("router2", HostTaskResult::skipped());

        validate.insert_sub_task("collect_logs", collect_logs);
        root.insert_sub_task("validate", validate);

        assert_eq!(root.task_name(), "deploy");
        assert_eq!(
            root.passed_hosts()
                .into_iter()
                .map(|host| host.as_str())
                .collect::<Vec<_>>(),
            vec!["router1"]
        );
        assert_eq!(
            root.failed_hosts()
                .into_iter()
                .map(|host| host.as_str())
                .collect::<Vec<_>>(),
            vec!["router2"]
        );

        let validate = root.sub_task("validate").expect("validate sub task should exist");
        assert_eq!(validate.task_name(), "validate");
        assert!(
            validate
                .host_result("router2")
                .expect("router2 validate result should exist")
                .is_skipped()
        );

        let collect_logs = validate
            .sub_task("collect_logs")
            .expect("collect_logs sub task should exist");
        assert_eq!(collect_logs.task_name(), "collect_logs");
        assert_eq!(
            collect_logs
                .host_result("router1")
                .and_then(HostTaskResult::success)
                .and_then(TaskSuccess::diff),
            Some("captured logs")
        );
    }
}
