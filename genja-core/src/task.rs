use serde_json::Value;
use log::warn;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::ConnectionKey;
    use std::sync::atomic::{AtomicUsize, Ordering};

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
