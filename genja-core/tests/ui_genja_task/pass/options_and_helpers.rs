use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskRuntimeContext, TaskSuccess};

struct OptionsTask {
    options: Option<serde_json::Value>,
}

#[genja_task(name = "options_task", processors = ["audit"])]
impl OptionsTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }

    fn options(&self) -> Option<&serde_json::Value> {
        self.options.as_ref()
    }

    fn helper(&self) -> bool {
        self.options.is_some()
    }
}

fn main() {}
