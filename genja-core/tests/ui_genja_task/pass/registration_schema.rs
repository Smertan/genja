use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SchemaTask {
    name: String,
    rules: Vec<SchemaRule>,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct SchemaRule {
    action: String,
}

#[genja_task(
    name = "schema_task",
    registration(
        id = "acme.tests.schema_task",
        input_schema = "schemars"
    )
)]
impl SchemaTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary(format!("{}:{}", self.name, self.rules.len())),
        ))
    }
}

fn main() {}
