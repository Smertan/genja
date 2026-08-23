# Rust Task Registration

Task registration lets Rust applications discover and construct tasks that were
compiled into the process. It is the first layer of Genja's task catalog model:
CLIs, MCP servers, provider manifests, and future remote catalogs can inspect a
task descriptor before deciding whether to construct and run the task.

Registration is opt-in. Existing Rust tasks that use `#[genja_task(...)]`
without `registration(...)` continue to work as normal runtime tasks.

## When To Use Registration

Use registration when a task should be addressable by a stable catalog identity
instead of by directly constructing the Rust struct in application code.

Registration is useful for:

- listing tasks compiled into an application
- looking up task metadata by ID and version
- showing input schemas to a CLI, UI, or MCP client
- constructing a task from JSON input
- exporting descriptor metadata into future provider manifests

Direct construction is still the simplest path when the caller already knows
the Rust type:

```rust
let task = BackupConfig {
    backup_path: "/tmp/configs".to_string(),
    compress: true,
};
```

Registration is the API-style path when the caller knows a task identity and
has JSON-compatible input:

```rust
let task = create_compiled_task_by_identity(
    "acme.examples.backup_config@1.0.0",
    serde_json::json!({
        "backup_path": "/tmp/configs",
        "compress": true,
        "rules": []
    }),
)?;
```

## Discovery And Stable IDs

Every Rust task annotated with `#[genja_task(...)]` is submitted to the compiled
task registry.

Tasks without explicit registration receive a generated ID:

```text
auto:<rust-type-path>
```

Generated IDs are useful for local discovery, but they are derived from Rust
implementation details and should not be treated as stable public contracts.
Renaming a type or moving a module can change the generated ID.

Registered tasks use an explicit stable ID:

```rust
#[genja_task(
    name = "backup_config",
    registration(id = "acme.examples.backup_config")
)]
impl BackupConfig {
    // task methods
}
```

The public identity is:

```text
<task-id>@<task-version>
```

For example:

```text
acme.examples.backup_config@1.0.0
```

Task IDs are namespace-friendly lowercase identifiers made of `.` separated
segments. Versions must be semantic versions. If `version` is omitted, the
macro uses the provider crate version from `CARGO_PKG_VERSION`.

Duplicate `id + version` registrations return a deterministic registry error
when the compiled registry is built.

## Default JSON Construction

The default factory for registered Rust tasks is serde deserialization. When
`factory` is omitted, Genja treats the JSON input passed to the registry as the
task struct's construction input. The JSON object must match the task fields.

```rust
use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
};
use genja::genja_task;

#[derive(serde::Deserialize)]
struct BackupConfig {
    backup_path: String,
    compress: bool,
}

#[genja_task(
    name = "backup_config",
    connection_plugin_name = "ssh",
    registration(
        id = "acme.examples.backup_config",
        version = "1.0.0",
        description = "Backs up selected paths from a network device"
    )
)]
impl BackupConfig {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary(format!(
                "backing up to {} compress={}",
                self.backup_path,
                self.compress
            )),
        ))
    }
}
```

With this task registered, a caller can construct it from JSON:

```rust
use genja::genja_core::task::{TaskInfo, create_compiled_task_by_identity};
use serde_json::json;

let task = create_compiled_task_by_identity(
    "acme.examples.backup_config@1.0.0",
    json!({
        "backup_path": "/tmp/configs",
        "compress": true
    }),
)?;

assert_eq!(task.name(), "backup_config");
```

The input:

```json
{
  "backup_path": "/tmp/configs",
  "compress": true
}
```

maps directly to the task struct:

```rust
BackupConfig {
    backup_path: "/tmp/configs".to_string(),
    compress: true,
}
```

The registry returns a `TaskDefinition`, which is Genja's runtime wrapper around
the constructed task. Conceptually, the factory result is:

```rust
TaskDefinition::new(BackupConfig {
    backup_path: "/tmp/configs".to_string(),
    compress: true,
})
```

This is the normal path when the public JSON input has the same shape as the
Rust task struct. The task type must implement
`serde::de::DeserializeOwned`. Nested input types must be deserializable too.
If the JSON does not match the struct fields, construction fails with
`TaskRegistrationError::InvalidInput`.

## Input Schemas

Add `schema = "schemars"` when callers should be able to inspect the expected
JSON input shape.

```rust
#[derive(serde::Deserialize, schemars::JsonSchema)]
struct BackupConfig {
    backup_path: String,
    compress: bool,
}

#[genja_task(
    name = "backup_config",
    registration(
        id = "acme.examples.backup_config",
        schema = "schemars"
    )
)]
impl BackupConfig {
    // task methods
}
```

`schema = "schemars"` requires the task type and any nested input types to
implement `schemars::JsonSchema`.

The generated schema is stored in `TaskDescriptor::input_schema`:

```rust
let descriptor =
    get_compiled_task_descriptor_by_identity("acme.examples.backup_config@1.0.0")?;

if let Some(schema) = descriptor.input_schema {
    println!("{}", serde_json::to_string_pretty(&schema)?);
}
```

Run the complete schema example from the repository root:

```bash
cargo run -p genja --example task_registration
```

## Listing And Looking Up Tasks

Use `list_compiled_tasks()` when you need a local task list:

```rust
use genja::genja_core::task::list_compiled_tasks;

for descriptor in list_compiled_tasks()? {
    println!("{}@{} {}", descriptor.id, descriptor.version, descriptor.name);
}
```

Use identity-based lookup when a CLI, API, or MCP tool receives a single task
identity string:

```rust
use genja::genja_core::task::get_compiled_task_descriptor_by_identity;

let descriptor =
    get_compiled_task_descriptor_by_identity("acme.examples.backup_config@1.0.0")?;
```

Lookup by ID without a version succeeds only when exactly one version exists.
If multiple versions are registered for the same ID, Genja returns an ambiguous
version error so callers do not accidentally run the wrong contract.

## Choosing A Construction API

The compiled registry exposes two construction paths. Use the direct registry
path when application code already has a task identity and JSON value. Use a
task spec when the caller supplies a YAML or JSON text payload that includes
both the task identity and input.

| Use case | API |
| --- | --- |
| List compiled task descriptors | `list_compiled_tasks()` |
| Inspect one descriptor by identity | `get_compiled_task_descriptor_by_identity(...)` |
| Construct from identity plus JSON value | `create_compiled_task_by_identity(...)` |
| Parse a YAML/JSON task spec without constructing | `TaskSpec::parse_auto(...)` |
| Construct from a YAML/JSON task spec string | `create_compiled_task_from_spec_str(...)` |

## Constructing Tasks From JSON

Use `create_compiled_task_by_identity(...)` when the caller supplies JSON input:

```rust
use genja::genja_core::task::{TaskInfo, create_compiled_task_by_identity};
use serde_json::json;

let task = create_compiled_task_by_identity(
    "acme.examples.backup_config@1.0.0",
    json!({
        "backup_path": "/tmp/configs",
        "compress": true
    }),
)?;

println!("created task {}", task.name());
```

Construction returns a `TaskDefinition`. Most task authors do not need to build
or inspect this wrapper directly; it exists so registry, task-list, and runtime
APIs can store and execute different task types through one interface.

## Declarative Task Specs

Use `TaskSpec` when the caller supplies a single task invocation as text, such
as a CLI file, API request, or MCP tool payload. A task spec contains the
registered task identity, the JSON-compatible input passed to that task's
factory, and optional runtime policy overrides.

YAML:

```yaml
task: acme.examples.backup_config@1.0.0
input:
  backup_path: /tmp/configs
  compress: true
  rules:
    - path: /etc/network
      recursive: true
overrides:
  retry:
    allow: true
    max_attempts: 3
    delay_ms: 500
  session_verification:
    max_attempts: 2
    delay_ms: 1000
```

JSON:

```json
{
  "task": "acme.examples.backup_config@1.0.0",
  "input": {
    "backup_path": "/tmp/configs",
    "compress": true,
    "rules": [
      {
        "path": "/etc/network",
        "recursive": true
      }
    ]
  },
  "overrides": {
    "retry": {
      "allow": true,
      "max_attempts": 3,
      "delay_ms": 500
    },
    "session_verification": {
      "max_attempts": 2,
      "delay_ms": 1000
    }
  }
}
```

Parse a spec without constructing the task when you need to inspect or validate
the request first:

```rust
use genja::genja_core::task::TaskSpec;

let spec = TaskSpec::parse_auto(source)?;
println!("requested task {}", spec.task);
```

Construct directly from a spec string when the caller wants the registered task:

```rust
use genja::genja_core::task::{TaskInfo, create_compiled_task_from_spec_str};

let task = create_compiled_task_from_spec_str(source)?;
println!("created task {}", task.name());
```

`parse_auto(...)` tries JSON first, then YAML. If neither parser accepts the
input, Genja returns an auto-parse error that includes both parser messages. If
the text parses but is not an object with `task`, optional `input`, and optional
`overrides`, Genja returns a task-spec shape error instead of treating it as a
registry failure.

Overrides are narrow per-run runtime policy controls. The first supported
fields are:

| Override | Effect |
| --- | --- |
| `retry` | Overrides the constructed task's retry policy for this run. |
| `session_verification` | Overrides post-change session verification policy for this run. |

Overrides do not rewrite authored task behavior. The spec intentionally does
not support overriding processors, connection plugin names, execution mode,
task identity, factory strategy, schema, or registration metadata. In
particular, processor overrides are rejected because processors can affect
startup hooks, result handling, and side effects selected by the task author.

This is a single-task construction spec. It is not a workflow DSL or task list.
Future work can build those features on top of the same registered task
identity and JSON-compatible input contract.

Run the complete task spec example from the repository root:

```bash
cargo run -p genja --example task_registration_spec
```

## Default Factory

Use `factory = "default"` for registered tasks that do not accept input:

```rust
#[derive(Default)]
struct CollectFacts;

#[genja_task(
    name = "collect_facts",
    registration(
        id = "acme.examples.collect_facts",
        factory = "default"
    )
)]
impl CollectFacts {
    // task methods
}
```

The task type must implement `Default`. Callers must pass `null` or `{}` as
construction input.

## Custom Factory

Use `factory = custom(path)` when the public JSON input should differ from the
task struct or when construction requires input preparation.

```rust
use genja::genja_core::task::TaskRegistrationError;
use serde_json::Value;

struct ConfigureAcl {
    acl_name: String,
    secret_token: String,
}

fn create_configure_acl(input: Value) -> Result<ConfigureAcl, TaskRegistrationError> {
    let acl_name = input
        .get("acl")
        .and_then(Value::as_str)
        .ok_or_else(|| TaskRegistrationError::InvalidInput {
            id: "acme.examples.configure_acl".to_string(),
            version: "1.0.0".to_string(),
            message: "`acl` is required".to_string(),
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
```

Then reference the function from the task macro:

```rust
#[genja_task(
    name = "configure_acl",
    registration(
        id = "acme.examples.configure_acl",
        version = "1.0.0",
        factory = custom(create_configure_acl)
    )
)]
impl ConfigureAcl {
    // task methods
}
```

Custom factories should return sanitized errors. Identify the affected task and
field, but do not include raw input values or decoded secret material in error
messages.

Run the complete custom factory example from the repository root:

```bash
cargo run -p genja --example task_registration_custom_factory
```

## Descriptor JSON

Descriptors serialize to JSON for future provider manifests, remote catalogs,
and MCP tooling:

```json
{
  "id": "acme.examples.backup_config",
  "id_source": "explicit",
  "name": "backup_config",
  "version": "1.0.0",
  "description": "Backs up selected paths from a network device",
  "execution_mode": "async",
  "connection_plugin_name": "ssh",
  "processor_names": [],
  "retry": null,
  "input_schema": null,
  "constructible": true
}
```

The exact descriptor contract is shared by future Rust and Python registration
work. Python task registration is planned separately; today this guide covers
the Rust registration path.

## Related Examples

- `genja/examples/task_registration.rs` shows the baseline registration,
  descriptor listing, schema, and direct construction path.
- `genja/examples/task_registration_custom_factory.rs` shows a custom factory
  for prepared JSON input and sanitized validation errors.
- `genja/examples/task_registration_spec.rs` shows YAML/JSON task spec parsing,
  construction, and retry/session verification overrides.

## Related Guides

- [Tasks](tasks.md)
- [Examples](examples.md)
- [API Surface](api-surface.md)
