//! Demonstrates constructing a registered task from YAML and JSON spec strings.

use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, TaskError, TaskInfo, TaskRuntimeContext, TaskSpec, TaskSuccess,
    create_compiled_task_from_spec_str, create_compiled_task_from_spec_str_with_format,
};
use genja::genja_task;

#[derive(serde::Deserialize)]
struct BackupRule {
    path: String,
    recursive: bool,
}

#[derive(serde::Deserialize)]
struct BackupConfig {
    backup_path: String,
    compress: bool,
    rules: Vec<BackupRule>,
}

#[genja_task(
    name = "backup_config_from_spec",
    connection_plugin_name = "ssh",
    registration(
        id = "acme.examples.backup_config_from_spec",
        version = "1.0.0",
        description = "Constructs a backup task from a declarative spec"
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
    let yaml_spec = r#"
task: acme.examples.backup_config_from_spec@1.0.0
input:
  backup_path: /tmp/configs
  compress: true
  rules:
    - path: /etc/network
      recursive: true
overrides:
  retry:
    allow: true
    max_attempts: 2
    delay_ms: 250
  session_verification:
    max_attempts: 2
    delay_ms: 1000
"#;

    let spec = TaskSpec::parse_auto(yaml_spec)?;
    println!("Parsed spec for {}", spec.task);

    let yaml_task = create_compiled_task_from_spec_str(yaml_spec)?;
    println!("Created from YAML spec: {}", yaml_task.name());
    if let Some(retry) = yaml_task.retry_config() {
        println!(
            "Retry override: allow={:?} max_attempts={:?} delay_ms={:?}",
            retry.allow(),
            retry.max_attempts(),
            retry.delay_ms()
        );
    }
    if let Some(session_verification) = yaml_task.session_verification_config() {
        println!(
            "Session verification override: max_attempts={} delay_ms={}",
            session_verification.max_attempts(),
            session_verification.delay_ms()
        );
    }

    let json_spec = r#"
{
  "task": "acme.examples.backup_config_from_spec@1.0.0",
  "input": {
    "backup_path": "/tmp/configs",
    "compress": false,
    "rules": [
      {
        "path": "/etc/hosts",
        "recursive": false
      }
    ]
  }
}
"#;

    let json_task = create_compiled_task_from_spec_str_with_format(
        json_spec,
        genja::genja_core::task::TaskSpecFormat::Json,
    )?;
    println!("Created from JSON spec: {}", json_task.name());

    Ok(())
}
