use genja_core::genja_task;
use serde_json::Value;

#[derive(Default)]
struct DefaultFactoryTask;

#[genja_task(
    name = "default_factory_task",
    registration(
        id = "acme.tests.default_factory_task",
        factory = "default"
    )
)]
impl DefaultFactoryTask {
    async fn start_async(
        &self,
        _host: &genja_core::inventory::Host,
        _context: &genja_core::task::TaskRuntimeContext,
    ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
        Ok(genja_core::task::HostTaskResult::passed(
            genja_core::task::TaskSuccess::new(),
        ))
    }
}

struct CustomFactoryTask {
    value: String,
}

fn create_custom_factory_task(
    input: Value,
) -> Result<CustomFactoryTask, genja_core::task::TaskRegistrationError> {
    let value = input
        .get("value")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    Ok(CustomFactoryTask { value })
}

#[genja_task(
    name = "custom_factory_task",
    registration(
        id = "acme.tests.custom_factory_task",
        factory = custom(create_custom_factory_task)
    )
)]
impl CustomFactoryTask {
    async fn start_async(
        &self,
        _host: &genja_core::inventory::Host,
        _context: &genja_core::task::TaskRuntimeContext,
    ) -> Result<genja_core::task::HostTaskResult, genja_core::task::TaskError> {
        Ok(genja_core::task::HostTaskResult::passed(
            genja_core::task::TaskSuccess::new().with_summary(self.value.clone()),
        ))
    }
}

fn main() {}
