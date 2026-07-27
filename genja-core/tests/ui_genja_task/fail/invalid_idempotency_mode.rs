use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, IdempotencyMode, TaskRuntimeContext, TaskSuccess};

struct BadIdempotencyTask;

#[genja_task(
    name = "bad",
    idempotency = IdempotencyMode::Validate
)]
impl BadIdempotencyTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {}
