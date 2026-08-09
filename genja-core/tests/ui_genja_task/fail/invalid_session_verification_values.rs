use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};

struct InvalidSessionVerificationTask;

#[genja_task(
    name = "invalid_session_verification",
    connection_plugin_name = "ssh",
    session_verification(max_attempts = 0)
)]
impl InvalidSessionVerificationTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {}
