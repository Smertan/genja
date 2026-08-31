use genja_core::genja_task;

#[derive(serde::Deserialize)]
struct BadTask;

#[genja_task(
    name = "bad_task",
    registration(
        id = "acme.tests.bad_task",
        factory = "custom"
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
