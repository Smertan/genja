use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{
    BlockingTaskRuntimeContext, HostTaskResult, IdempotencyCheck, IdempotencyMode,
    TaskRuntimeContext, TaskSuccess,
};

struct BlockingAsyncCheckTask;

#[genja_task(
    name = "blocking_async_check",
    idempotency = IdempotencyMode::CheckAndVerify
)]
impl BlockingAsyncCheckTask {
    fn start(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    async fn check_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<IdempotencyCheck, genja_core::task::TaskError> {
        Ok(IdempotencyCheck::Converged {
            summary: None,
            details: None,
        })
    }
}

fn main() {}
