//! Project-local compiled task discovery through the optional CLI re-export.

#![cfg(feature = "genja-cli")]

use genja::cli::commands::task::{describe_task, list_tasks};
use genja::cli::discovery::rust::CompiledTaskDescriptorSource;
use genja::cli::output::OutputFormat;
use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
    get_compiled_task_descriptor_by_identity,
};
use genja::genja_task;

const IDENTITY: &str = "acme.tests.feature.local_task@1.0.0";

#[derive(Default)]
struct LocalTask;

#[genja_task(
    name = "local_task",
    registration(
        id = "acme.tests.feature.local_task",
        version = "1.0.0",
        description = "Task linked by the consuming project",
        factory = "default"
    )
)]
impl LocalTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

#[test]
fn cli_reexport_lists_the_project_registered_task() {
    let source = CompiledTaskDescriptorSource::new();
    let output = list_tasks(&source, OutputFormat::Json).expect("listing should succeed");
    let actual: serde_json::Value = serde_json::from_str(&output).expect("valid JSON list");
    let descriptor =
        get_compiled_task_descriptor_by_identity(IDENTITY).expect("task should be registered");

    assert_eq!(actual, serde_json::json!([descriptor]));
}

#[test]
fn cli_reexport_describes_the_same_task_as_core() {
    let source = CompiledTaskDescriptorSource::new();
    let output =
        describe_task(&source, IDENTITY, OutputFormat::Json).expect("description should succeed");
    let actual: serde_json::Value = serde_json::from_str(&output).expect("valid JSON descriptor");
    let descriptor =
        get_compiled_task_descriptor_by_identity(IDENTITY).expect("task should be registered");

    assert_eq!(actual, serde_json::json!(descriptor));
    assert_eq!(actual["id"], "acme.tests.feature.local_task");
    assert_eq!(actual["version"], "1.0.0");
    assert_eq!(
        actual["description"],
        "Task linked by the consuming project"
    );
    assert_eq!(actual["constructible"], true);
}
