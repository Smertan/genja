use genja_core::genja_task;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BadTask {
    rules: Vec<BadRule>,
}

#[derive(serde::Deserialize)]
struct BadRule {
    action: String,
}

#[genja_task(
    name = "bad_task",
    registration(
        id = "acme.tests.bad_nested_schema_task",
        input_schema = "schemars"
    )
)]
impl BadTask {
    async fn start_async(
        &self,
        _host: &genja_core::inventory::Host,
        _context: &genja_core::task::TaskRuntimeContext,
    ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
        Ok(genja_core::task::HostTaskResult::passed(
            genja_core::task::TaskSuccess::new().with_summary(self.rules.len().to_string()),
        ))
    }
}

fn main() {}
