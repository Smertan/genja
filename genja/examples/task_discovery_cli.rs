//! Runnable fixture for the CLI guide's descriptor output examples.

use genja::genja_core::inventory::Host;
use genja::genja_core::task::{HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess};
use genja::genja_task;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BackupConfig {
    backup_path: String,
    compress: bool,
}

#[genja_task(
    name = "backup_config",
    connection_plugin_name = "ssh",
    retry(allow = true, max_attempts = 3, delay_ms = 250),
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
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_summary(
            format!(
                "Documentation fixture only: backup_path={} compress={}",
                self.backup_path, self.compress
            ),
        )))
    }
}

fn main() -> std::process::ExitCode {
    genja::cli::run_main()
}
