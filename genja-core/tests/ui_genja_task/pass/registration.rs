use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};
use serde_json::Value;

#[derive(serde::Deserialize)]
struct RegisteredTask {
    options: Value,
}

#[genja_task(
    name = "registered_task",
    registration(
        id = "acme.tests.registered_task",
        version = "1.2.3",
        description = "Registered task",
        factory = "serde"
    )
)]
impl RegisteredTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn options(&self) -> Option<&Value> {
        Some(&self.options)
    }
}

fn main() {}
