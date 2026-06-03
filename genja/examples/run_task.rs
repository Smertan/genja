use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, Task, TaskError, TaskRuntimeContext, TaskSuccess,
};
use genja::{Genja, genja_task};
use serde_json::json;

struct CollectFacts;

#[genja_task(name = "collect_facts")]
impl CollectFacts {
    async fn start_async(
        &self,
        host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
            json!({
                "hostname": host.hostname(),
                "platform": host.platform(),
                "facts_collected": true
            }),
        )))
    }
}

fn main() -> Result<(), genja::GenjaError> {
    let genja = Genja::from_settings_file("genja/examples/settings.yaml")?;

    let results = genja.run_task(
        CollectFacts,
        1,
    )?;

    let output = results.to_pretty_json_string().map_err(|err| {
        genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
    })?;
    println!("{output}");

    Ok(())
}
