//! Sample registrations for exploring the CLI and optional terminal task browser.
//!
//! These tasks are linked only into this example binary. Their implementations
//! return demonstration summaries and do not connect to hosts or perform backups.

use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    BlockingTaskRuntimeContext, HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
};
use genja::genja_task;

#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BackupConfig {
    backup_path: String,
}

#[genja_task(
    name = "backup_config",
    registration(
        id = "acme.examples.backup_config",
        version = "1.0.0",
        description = "Demonstration backup task with a JSON input schema",
        input_schema = "schemars"
    )
)]
impl BackupConfig {
    fn start(
        &self,
        _host: &Host,
        _context: &BlockingTaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_summary(
            format!("Example only: backup destination is {}", self.backup_path),
        )))
    }
}

struct CollectFacts;

// Omitting registration(...) gives this task a generated ID and no factory.
#[genja_task(name = "collect_facts")]
impl CollectFacts {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary("Example only: no facts were collected"),
        ))
    }
}

fn main() -> std::process::ExitCode {
    genja::cli::run_main()
}
