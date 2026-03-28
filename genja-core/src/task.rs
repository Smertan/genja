use serde_json::Value;
use std::ops::{Deref, DerefMut};

/// Core task interface required for execution.
pub trait Task {
    /// Start executing the task.
    fn start(&self) -> Result<(), crate::GenjaError>;

    /// Return the task's name.
    fn name(&self) -> &str;

    /// Return the task's plugin name, if set.
    fn plugin(&self) -> Option<&str>;

    /// Return the task's options payload, if set.
    fn options(&self) -> Option<&Value>;
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
        self.inner.start()
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn plugin(&self) -> Option<&str> {
        self.inner.plugin()
    }

    fn options(&self) -> Option<&Value> {
        self.inner.options()
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
