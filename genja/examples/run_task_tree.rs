use std::sync::Arc;

use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, Task, TaskError, TaskRuntimeContext, TaskSuccess,
};
use genja::{Genja, TaskDerive, async_trait};
use serde_json::json;

#[derive(TaskDerive)]
struct DeployConfig {
    name: &'static str,
    #[task(subtask)]
    validate_config: Arc<dyn Task>,
    #[task(subtask)]
    collect_logs: Arc<dyn Task>,
}

#[async_trait]
impl Task for DeployConfig {
    async fn start(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
            json!({
                "task": self.name,
                "host": host.hostname(),
                "depth": context.current_depth(),
                "deployed": true
            }),
        )))
    }
}

#[derive(TaskDerive)]
struct ValidateConfig {
    name: &'static str,
}

#[async_trait]
impl Task for ValidateConfig {
    async fn start(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
            json!({
                "task": self.name,
                "host": host.hostname(),
                "depth": context.current_depth(),
                "valid": true
            }),
        )))
    }
}

#[derive(TaskDerive)]
struct CollectLogs {
    name: &'static str,
}

#[async_trait]
impl Task for CollectLogs {
    async fn start(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
            json!({
                "task": self.name,
                "host": host.hostname(),
                "depth": context.current_depth(),
                "logs_collected": true
            }),
        )))
    }
}

fn main() -> Result<(), genja::GenjaError> {
    let genja = Genja::from_settings_file("genja/examples/settings.yaml")?;

    let task = DeployConfig {
        name: "deploy_config",
        validate_config: Arc::new(ValidateConfig {
            name: "validate_config",
        }),
        collect_logs: Arc::new(CollectLogs {
            name: "collect_logs",
        }),
    };

    let results = genja.run_task(task, 1)?;

    let output = results.to_pretty_json_string().map_err(|err| {
        genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
    })?;
    println!("{output}");

    Ok(())
}
