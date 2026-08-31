use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, TaskError, TaskInfo, TaskRuntimeContext, TaskSuccess,
    create_compiled_task_by_identity, get_compiled_task_descriptor_by_identity,
    list_compiled_tasks,
};
use genja::genja_task;
use serde_json::json;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BackupRule {
    path: String,
    recursive: bool,
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BackupConfig {
    backup_path: String,
    compress: bool,
    rules: Vec<BackupRule>,
}

#[genja_task(
    name = "backup_config",
    connection_plugin_name = "ssh",
    registration(
        id = "acme.examples.backup_config",
        version = "1.0.0",
        description = "Backs up selected paths from a network device",
        input_schema = "schemars"
    )
)]
impl BackupConfig {
    async fn start_async(
        &self,
        host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        let first_rule = self
            .rules
            .first()
            .map(|rule| format!("{} recursive={}", rule.path, rule.recursive))
            .unwrap_or_else(|| "no paths".to_string());

        Ok(HostTaskResult::passed(TaskSuccess::new().with_summary(
            format!(
                "backing up {} to {} compress={} first_rule={}",
                host.hostname().unwrap_or("host"),
                self.backup_path,
                self.compress,
                first_rule
            ),
        )))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let identity = "acme.examples.backup_config@1.0.0";

    println!("Registered task identities:");
    for descriptor in list_compiled_tasks()? {
        println!(
            "- {}@{} ({})",
            descriptor.id, descriptor.version, descriptor.name
        );
    }

    let descriptor = get_compiled_task_descriptor_by_identity(identity)?;
    println!("\nDescriptor:");
    println!("{}", serde_json::to_string_pretty(&descriptor)?);

    if let Some(schema) = &descriptor.input_schema {
        println!("\nInput schema:");
        println!("{}", serde_json::to_string_pretty(schema)?);
    }

    let task = create_compiled_task_by_identity(
        identity,
        json!({
            "backup_path": "/tmp/configs",
            "compress": true,
            "rules": [
                {
                    "path": "/etc/network",
                    "recursive": true
                }
            ]
        }),
    )?;

    println!("\nCreated task: {}", task.name());

    Ok(())
}
