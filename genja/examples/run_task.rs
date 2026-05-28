use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, Task, TaskError, TaskRuntimeContext, TaskSuccess,
};
use genja::{Genja, TaskDerive, async_trait};
use serde_json::json;

#[derive(TaskDerive)]
struct CollectFacts {
    name: &'static str,
}

#[async_trait]
impl Task for CollectFacts {
    async fn start(
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
        CollectFacts {
            name: "collect_facts",
        },
        1,
    )?;

    let output = results.to_pretty_json_string().map_err(|err| {
        genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
    })?;
    println!("{output}");

    Ok(())
}
