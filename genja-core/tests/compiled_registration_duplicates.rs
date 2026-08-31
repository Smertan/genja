use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{
    HostTaskResult, TaskRegistrationError, TaskRuntimeContext, TaskSuccess, compiled_task_registry,
};

#[derive(serde::Deserialize)]
struct FirstDuplicateTask;

#[genja_task(
    name = "first_duplicate_task",
    registration(id = "acme.tests.compiled_duplicates.same_task", version = "1.0.0")
)]
impl FirstDuplicateTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

#[derive(serde::Deserialize)]
struct SecondDuplicateTask;

#[genja_task(
    name = "second_duplicate_task",
    registration(id = "acme.tests.compiled_duplicates.same_task", version = "1.0.0")
)]
impl SecondDuplicateTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, genja_core::task::TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}

#[test]
fn compiled_registry_rejects_duplicate_macro_registrations() {
    let error = match compiled_task_registry() {
        Ok(_) => panic!("duplicate registrations should fail"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        TaskRegistrationError::DuplicateRegistration {
            id: "acme.tests.compiled_duplicates.same_task".to_string(),
            version: "1.0.0".to_string(),
        }
    );
    assert_eq!(
        error.to_string(),
        "duplicate task registration `acme.tests.compiled_duplicates.same_task@1.0.0`"
    );
}
