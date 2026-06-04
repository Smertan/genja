use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};

struct AsyncTask;

#[genja_task(name = "async_task")]
impl AsyncTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {}
