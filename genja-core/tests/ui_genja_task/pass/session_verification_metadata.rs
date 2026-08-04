use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskInfo, TaskRuntimeContext, TaskSuccess};

struct SessionVerificationTask;

#[genja_task(
    name = "session_verification_task",
    connection_plugin_name = "ssh",
    session_verification(
        max_attempts = 3,
        delay_ms = 5000
    )
)]
impl SessionVerificationTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

fn main() {
    let task = SessionVerificationTask;
    let config = task
        .session_verification_config()
        .expect("session verification config should exist");
    assert_eq!(config.max_attempts(), 3);
    assert_eq!(config.delay_ms(), 5000);
}
