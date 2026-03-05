use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Task;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tasks(Vec<Task>);

impl Tasks {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add_task(&mut self, task: Task) {
        self.0.push(task);
    }
}

impl Deref for Tasks {
    type Target = Vec<Task>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Tasks {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
