use genja_core::genja_task;
use serde_json::Value;

struct BadTask;

fn create_bad_task(_input: Value) -> Result<(), genja_core::task::TaskRegistrationError> {
    Ok(())
}

#[genja_task(
    name = "bad_task",
    registration(
        id = "acme.tests.bad_custom_return_task",
        factory = custom(create_bad_task)
    )
)]
impl BadTask {
    async fn start_async(
        &self,
        _host: &genja_core::inventory::Host,
        _context: &genja_core::task::TaskRuntimeContext,
    ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
        Ok(genja_core::task::HostTaskResult::passed(
            genja_core::task::TaskSuccess::new(),
        ))
    }
}

fn main() {}
