use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{
    HostTaskResult, IdempotencyMode, TaskInfo, TaskRuntimeContext, TaskSuccess,
};

struct IdempotentTask;

#[genja_task(
    name = "idempotent",
    idempotency = IdempotencyMode::Check
)]
impl IdempotentTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {
    let task = IdempotentTask;
    assert_eq!(task.idempotency_mode(), IdempotencyMode::Check);
}
