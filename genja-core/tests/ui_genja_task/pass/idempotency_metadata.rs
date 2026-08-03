use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{
    HostTaskResult, IdempotencyCheck, IdempotencyMode, TaskInfo, TaskRuntimeContext, TaskSuccess,
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

    async fn check_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<IdempotencyCheck, genja_core::task::TaskError> {
        Ok(IdempotencyCheck::Converged {
            summary: Some("already converged".to_string()),
            details: None,
        })
    }
}

fn main() {
    let task = IdempotentTask;
    assert_eq!(task.idempotency_mode(), IdempotencyMode::Check);
}
