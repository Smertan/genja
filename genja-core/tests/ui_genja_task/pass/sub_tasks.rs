use std::sync::Arc;

use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, Task, TaskRuntimeContext, TaskSuccess};

struct ChildTask;

#[genja_task(name = "child")]
impl ChildTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

struct ParentTask {
    children: Vec<Arc<dyn Task>>,
}

#[genja_task(name = "parent")]
impl ParentTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        self.children.clone()
    }
}

fn main() {}
