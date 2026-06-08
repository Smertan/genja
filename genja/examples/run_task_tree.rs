use std::sync::Arc;

use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, Task, TaskError, TaskRuntimeContext, TaskSuccess,
};
use genja::{Genja, genja_task};
use serde_json::json;

struct DeployConfig {
    validate_config: Arc<dyn Task>,
    collect_logs: Arc<dyn Task>,
}

#[genja_task(name = "deploy_config")]
impl DeployConfig {
    async fn start_async(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
            json!({
                "task": "deploy_config",
                "host": host.hostname(),
                "depth": context.current_depth(),
                "deployed": true
            }),
        )))
    }

    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        vec![
            Arc::clone(&self.validate_config),
            Arc::clone(&self.collect_logs),
        ]
    }
}

struct ValidateConfig;

#[genja_task(name = "validate_config")]
impl ValidateConfig {
    async fn start_async(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
            json!({
                "task": "validate_config",
                "host": host.hostname(),
                "depth": context.current_depth(),
                "valid": true
            }),
        )))
    }
}

struct CollectLogs;

#[genja_task(name = "collect_logs")]
impl CollectLogs {
    async fn start_async(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
            json!({
                "task": "collect_logs",
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
        validate_config: Arc::new(ValidateConfig),
        collect_logs: Arc::new(CollectLogs),
    };

    let results = genja.run_task(task, 1)?;

    let output = results.to_pretty_json_string().map_err(|err| {
        genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
    })?;
    println!("{output}");

    Ok(())
}
