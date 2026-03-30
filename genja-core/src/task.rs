use serde_json::Value;
use std::ops::{Deref, DerefMut};
use std::sync::Arc;

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

impl Task for TaskDefinition {
    fn start(&self) -> Result<(), crate::GenjaError> {
        Self::start_recursive(self.inner.as_ref())
    }
}

impl TaskDefinition {
    fn start_recursive(task: &dyn Task) -> Result<(), crate::GenjaError> {
        task.start()?;
        for sub in task.sub_tasks() {
            Self::start_recursive(sub.as_ref())?;
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
