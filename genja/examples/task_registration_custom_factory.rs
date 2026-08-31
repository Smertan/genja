use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, TaskError, TaskInfo, TaskRegistrationError, TaskRuntimeContext, TaskSuccess,
    create_compiled_task_by_identity, get_compiled_task_descriptor_by_identity,
};
use genja::genja_task;
use serde_json::Value;

struct ConfigureAcl {
    acl_name: String,
    secret_token: String,
}

fn create_configure_acl(input: Value) -> Result<ConfigureAcl, TaskRegistrationError> {
    let acl_name = input.get("acl").and_then(Value::as_str).ok_or_else(|| {
        TaskRegistrationError::InvalidInput {
            id: "acme.examples.configure_acl".to_string(),
            version: "1.0.0".to_string(),
            message: "`acl` is required".to_string(),
        }
    })?;
    let token_obfuscated = input
        .get("token_obfuscated")
        .and_then(Value::as_str)
        .ok_or_else(|| TaskRegistrationError::InvalidInput {
            id: "acme.examples.configure_acl".to_string(),
            version: "1.0.0".to_string(),
            message: "`token_obfuscated` is required".to_string(),
        })?;

    Ok(ConfigureAcl {
        acl_name: acl_name.to_string(),
        secret_token: token_obfuscated.chars().rev().collect(),
    })
}

#[genja_task(
    name = "configure_acl",
    connection_plugin_name = "ssh",
    registration(
        id = "acme.examples.configure_acl",
        version = "1.0.0",
        description = "Configures an ACL after preparing custom input",
        factory = custom(create_configure_acl)
    )
)]
impl ConfigureAcl {
    async fn start_async(
        &self,
        host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new().with_summary(
            format!(
                "configured {} on {} using a {} byte token",
                self.acl_name,
                host.hostname().unwrap_or("host"),
                self.secret_token.len()
            ),
        )))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let identity = "acme.examples.configure_acl@1.0.0";

    let descriptor = get_compiled_task_descriptor_by_identity(identity)?;
    println!("Descriptor:");
    println!("{}", serde_json::to_string_pretty(&descriptor)?);

    let task = create_compiled_task_by_identity(
        identity,
        serde_json::json!({
            "acl": "edge-inbound",
            "token_obfuscated": "terces"
        }),
    )?;

    println!("\nCreated task: {}", task.name());

    let error = match create_compiled_task_by_identity(identity, serde_json::json!({})) {
        Ok(_) => panic!("custom factory should reject missing fields"),
        Err(error) => error,
    };
    println!("\nRejected invalid input: {error}");

    Ok(())
}
