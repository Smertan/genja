use genja_core::genja_task;

#[derive(serde::Deserialize)]
struct BadTask {
    name: String,
}

#[genja_task(
    name = "bad_task",
    registration(
        id = "acme.tests.bad_schema_task",
        schema = "schemars"
    )
)]
impl BadTask {
    async fn start_async(
        &self,
        _host: &genja_core::inventory::Host,
        _context: &genja_core::task::TaskRuntimeContext,
    ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
        Ok(genja_core::task::HostTaskResult::passed(
            genja_core::task::TaskSuccess::new().with_summary(self.name.clone()),
        ))
    }
}

fn main() {}
