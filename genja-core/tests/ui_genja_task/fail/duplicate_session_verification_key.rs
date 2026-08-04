use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};

struct DuplicateSessionVerificationKeyTask;

#[genja_task(
    name = "duplicate_session_verification_key",
    connection_plugin_name = "ssh",
    session_verification(
        max_attempts = 2,
        max_attempts = 3
    )
)]
impl DuplicateSessionVerificationKeyTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {}
