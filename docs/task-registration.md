# Task Registration

Task registration lets applications discover, inspect, and construct tasks by
stable catalog identity. It is the first layer of Genja's task catalog model:
CLIs, MCP servers, provider manifests, and future remote catalogs can inspect a
task descriptor before deciding whether to construct and run the task.

Registration is opt-in. Existing Rust tasks that use `#[genja_task(...)]`
without `registration(...)`, and existing Python tasks that use `@task(...)`
without `registration=...`, continue to work as normal runtime tasks.

## When To Use Registration

Use registration when a task should be addressable by a stable catalog identity
instead of by directly constructing the concrete task type in application code.

Registration is useful for:

- listing locally available registered tasks
- looking up task metadata by ID and version
- showing input schemas to a CLI, UI, or MCP client
- constructing a task from JSON input
- exporting descriptor metadata into future provider manifests

Direct construction is still the simplest path when the caller already knows
the concrete task type:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let task = BackupConfig {
        backup_path: "/tmp/configs".to_string(),
        compress: true,
    };
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    task = BackupConfig(
        backup_path="/tmp/configs",
        compress=True,
    )
    ```

Registration is the API-style path when the caller knows a task identity and
has JSON-compatible input:

=== ":fontawesome-brands-rust: Rust"

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

=== ":fontawesome-brands-python: Python"

    ```python
    task = create_registered_task_by_identity(
        "acme.examples.backup_config@1.0.0",
        {
            "backup_path": "/tmp/configs",
            "compress": True,
            "rules": [],
        },
    )
    ```

## Discovery And Stable IDs

Rust tasks annotated with `#[genja_task(...)]` are submitted to the compiled
task registry. Python tasks are submitted to the Python registry when the module
containing the decorated class is imported.

Rust tasks without explicit registration receive a generated ID:

```text
auto:<rust-type-path>
```

Generated IDs are useful for local Rust discovery, but they are derived from
Rust implementation details and should not be treated as stable public
contracts. Renaming a type or moving a module can change the generated ID.
Python task registration is explicit-only; Python classes without
`registration=...` remain executable but do not appear in the Python task
registry.

Registered tasks use an explicit stable ID and a semantic version. The public
identity is:

```text
<task-id>@<task-version>
```

For example:

```text
acme.examples.backup_config@1.0.0
```

Task IDs are namespace-friendly lowercase identifiers made of `.` separated
segments. Versions must be semantic versions.

For Rust, `version` is optional. If it is omitted, the macro uses the provider
crate version from `CARGO_PKG_VERSION`.

For Python, `version` is optional in the API but only works when Genja can map
the decorated class's top-level module to exactly one installed Python
distribution and read that distribution's package metadata version. If the task
lives in a script, loose module, test file, namespace package, or any layout
where that mapping is missing or ambiguous, registration fails with a
validation error. In those cases, provide an explicit semantic version in
`TaskRegistration`. For example, use `version="1.0.0"` for the first stable
version of the task contract.

The examples below set `version` explicitly in both languages:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    #[genja_task(
        name = "backup_config",
        registration(
            id = "acme.examples.backup_config",
            version = "1.0.0"
        )
    )]
    impl BackupConfig {
        // task methods
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    @task(
        name="backup_config",
        registration=TaskRegistration(
            id="acme.examples.backup_config",
            version="1.0.0",
        ),
    )
    class BackupConfig:
        # task methods
        ...
    ```

Duplicate `id + version` registrations return a deterministic registry error
when the compiled registry is built.

## Default JSON Construction

The normal factory maps JSON-compatible input into the task constructor.

=== ":fontawesome-brands-rust: Rust"

    The default factory for registered Rust tasks is serde deserialization.
    When `factory` is omitted, Genja treats the JSON input passed to the
    registry as the task struct's construction input. The JSON object must
    match the task fields.

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

=== ":fontawesome-brands-python: Python"

    Python uses `TaskFactory.KWARGS` for the same shape: Genja calls
    `TaskClass(**input)`.

    ```python
    from genja.task import (
        TaskFactory,
        TaskRegistration,
        TaskSuccessResult,
        task,
    )


    @task(
        name="backup_config",
        connection_plugin_name="ssh",
        registration=TaskRegistration(
            id="acme.examples.backup_config",
            version="1.0.0",
            description="Backs up selected paths from a network device",
            factory=TaskFactory.KWARGS,
        ),
    )
    class BackupConfig:
        def __init__(self, backup_path: str, compress: bool) -> None:
            self.backup_path = backup_path
            self.compress = compress

        async def start_async(self, task, host, context):
            return TaskSuccessResult(
                summary=(
                    f"backing up to {self.backup_path} "
                    f"compress={self.compress}"
                )
            )
    ```

    ```python
    from genja.task import create_registered_task_by_identity

    task_definition = create_registered_task_by_identity(
        "acme.examples.backup_config@1.0.0",
        {
            "backup_path": "/tmp/configs",
            "compress": True,
        },
    )
    ```

For Rust serde construction, the input:

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

For Python `TaskFactory.KWARGS`, the same JSON-compatible mapping is expanded
into keyword arguments with `TaskClass(**input)`.

This is the normal path when the public JSON input has the same shape as the
task constructor. Rust task types must implement
`serde::de::DeserializeOwned`; nested input types must be deserializable too.
If the JSON does not match the Rust struct fields, construction fails with
`TaskRegistrationError::InvalidInput`. Python constructor errors are reported
as task input-validation errors with the task identity included.

## Input Schemas

Add input schema metadata when callers should be able to inspect the expected
JSON-compatible input shape.

=== ":fontawesome-brands-rust: Rust"

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
            input_schema = "schemars"
        )
    )]
    impl BackupConfig {
        // task methods
    }
    ```

    `input_schema = "schemars"` requires the task type and any nested input
    types to implement `schemars::JsonSchema`.

    ```rust
    let descriptor =
        get_compiled_task_descriptor_by_identity("acme.examples.backup_config@1.0.0")?;

    if let Some(schema) = descriptor.input_schema {
        println!("{}", serde_json::to_string_pretty(&schema)?);
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import ExplicitInputSchema, TaskRegistration

    registration = TaskRegistration(
        id="acme.examples.backup_config",
        version="1.0.0",
        input_schema=ExplicitInputSchema(
            value={
                "type": "object",
                "required": ["backup_path", "compress"],
                "properties": {
                    "backup_path": {"type": "string"},
                    "compress": {"type": "boolean"},
                },
            },
        ),
    )
    ```

    Python can also derive schema from Pydantic:

    ```python
    from pydantic import BaseModel
    from genja.task import PydanticInputSchema, TaskRegistration


    class BackupConfigInput(BaseModel):
        backup_path: str
        compress: bool


    registration = TaskRegistration(
        id="acme.examples.backup_config",
        version="1.0.0",
        input_schema=PydanticInputSchema(model=BackupConfigInput),
    )
    ```

    ```python
    descriptor = get_registered_task_descriptor_by_identity(
        "acme.examples.backup_config@1.0.0"
    )
    print(descriptor.to_dict()["input_schema"])
    ```

Run the complete Rust schema example from the repository root:

```bash
cargo run -p genja --example task_registration
```

## Listing And Looking Up Tasks

Use list APIs when you need a local task list:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::task::list_compiled_tasks;

    for descriptor in list_compiled_tasks()? {
        println!("{}@{} {}", descriptor.id, descriptor.version, descriptor.name);
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import list_registered_tasks

    for descriptor in list_registered_tasks():
        print(descriptor.identity, descriptor.name)
    ```

Use identity-based lookup when a CLI, API, or MCP tool receives a single task
identity string:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::task::get_compiled_task_descriptor_by_identity;

    let descriptor =
        get_compiled_task_descriptor_by_identity("acme.examples.backup_config@1.0.0")?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import get_registered_task_descriptor_by_identity

    descriptor = get_registered_task_descriptor_by_identity(
        "acme.examples.backup_config@1.0.0"
    )
    ```

Lookup by ID without a version succeeds only when exactly one version exists.
If multiple versions are registered for the same ID, Genja returns an ambiguous
version error so callers do not accidentally run the wrong contract.

## Rust Construction APIs

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

## Rust Construction From JSON

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

## Rust Declarative Task Specs

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
task identity, factory strategy, input schema, or registration metadata. In
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

Use the default factory for registered tasks that do not accept construction
input.

=== ":fontawesome-brands-rust: Rust"

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

    The task type must implement `Default`. Callers must pass `null` or `{}`
    as construction input.

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import TaskFactory, TaskRegistration, task


    @task(
        name="collect_facts",
        registration=TaskRegistration(
            id="acme.examples.collect_facts",
            version="1.0.0",
            factory=TaskFactory.DEFAULT,
        ),
    )
    class CollectFacts:
        async def start_async(self, task, host, context):
            ...
    ```

    Genja calls `CollectFacts()`. Callers may omit input or pass an empty
    mapping; non-empty input is rejected.

## Custom Factory

Use a custom factory when the public JSON-compatible input should differ from
the task constructor or when construction requires input preparation.

=== ":fontawesome-brands-rust: Rust"

    Use `factory = custom(path)` to point at a Rust factory function.

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

=== ":fontawesome-brands-python: Python"

    Use `CustomTaskFactory(callable=...)` to point at a Python factory
    callable.

    ```python
    from genja.task import CustomTaskFactory, TaskRegistration, task


    def create_configure_acl(input):
        acl_name = input.get("acl")
        token_obfuscated = input.get("token_obfuscated")
        if not isinstance(acl_name, str):
            raise ValueError("`acl` is required")
        if not isinstance(token_obfuscated, str):
            raise ValueError("`token_obfuscated` is required")
        return ConfigureAcl(
            acl_name=acl_name,
            secret_token="".join(reversed(token_obfuscated)),
        )


    @task(
        name="configure_acl",
        registration=TaskRegistration(
            id="acme.examples.configure_acl",
            version="1.0.0",
            factory=CustomTaskFactory(callable=create_configure_acl),
        ),
    )
    class ConfigureAcl:
        def __init__(self, acl_name: str, secret_token: str) -> None:
            self.acl_name = acl_name
            self.secret_token = secret_token

        async def start_async(self, task, host, context):
            ...
    ```

    The callable receives the parsed input mapping and must return an instance
    of the registered task class.

Custom factories should return sanitized errors. Identify the affected task and
field, but do not include raw input values or decoded secret material in error
messages. Genja wraps factory failures with the registered task identity.

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

## Python Import Boundary

Python task registration uses the same descriptor JSON contract as Rust task
registration. Registration is opt-in: existing Python `@task(...)` classes
without `registration=...` continue to work with
`TaskDefinition.from_python_class(...)` and normal runtime execution.

`registration=...` is handled by the Python `@task(...)` decorator itself. The
decorator still attaches normal execution metadata to the class. When
`registration` is also supplied, the decorator validates the registration
metadata, builds the language-neutral descriptor, records the selected factory,
and adds the class to the in-process Python task registry.

That decorator work happens only when Python executes the class definition. In
normal Python programs, that means when the module containing the decorated
class is imported.

```python
# acme/tasks/network.py
from genja.task import TaskRegistration, task


@task(
    name="configure_acl",
    registration=TaskRegistration(
        id="acme.network.configure_acl",
        version="1.0.0",
    ),
)
class ConfigureAcl:
    async def start_async(self, task, host, context):
        ...
```

The `ConfigureAcl` descriptor appears after the module is imported:

```python
import acme.tasks.network

from genja.task import list_registered_tasks

descriptors = list_registered_tasks()
```

Genja does not scan Python files looking for decorators. A task module that
exists on disk but has not been imported has not executed its class definition,
so its decorator has not run and its descriptor is not in the registry yet.
Provider manifests, entry points, or task-directory discovery can import known
task modules automatically in future features; this API only lists Python tasks
that are already imported in the current process.

## Python Design Constraints

Python registration is automatic after import, not automatic from files on disk.
This follows from how Python creates classes and applies decorators.

Keep these constraints in mind when designing Python providers, CLIs, or task
catalog tooling:

- The Python registry is in-process. Each process that wants to list, inspect,
  or construct Python registered tasks must import the task modules first.
- Importing a package only executes that package's `__init__.py`. It does not
  automatically import every module below the package unless `__init__.py`
  imports those modules.
- A Rust or PyO3 implementation of the Python decorator would not remove the
  import requirement. Rust `inventory` can collect compiled Rust registration
  records, but Python task classes are created dynamically when Python executes
  their modules.
- Python `version` inference only works when the decorated class's top-level
  module maps to exactly one installed Python distribution with readable
  package metadata. Scripts, loose modules, tests, and ambiguous package
  layouts should set `version` explicitly.
- Python task bodies still execute as Python code. Rust and Tokio can
  coordinate the surrounding Genja runtime, but `start(...)` and
  `start_async(...)` run through the Python runtime.
- `input_schema` is descriptor metadata. It describes JSON-compatible
  construction input for CLIs, UIs, MCP tools, and catalogs; it is not a
  separate task invocation format.

Future provider manifests, package entry points, or explicit CLI flags can
solve discovery by declaring which Python modules to import before reading the
registry.

## Python Factories

Python registration supports three construction strategies:

| Factory | Behavior |
| --- | --- |
| `TaskFactory.KWARGS` | Calls `TaskClass(**input)`. Input must be a JSON-compatible mapping. |
| `TaskFactory.DEFAULT` | Calls `TaskClass()` and rejects non-empty input. |
| `CustomTaskFactory(callable=...)` | Calls the custom callable with the input mapping. The callable must return an instance of the registered task class. |

Factory and input errors identify the affected task identity, such as
`acme.network.configure_acl@2.0.0`.

## Python Input Schemas

`TaskRegistration(input_schema=...)` describes the parsed JSON-compatible input
shape. It is not the input payload itself. Input may come from JSON, YAML, or
another source, but once parsed it is described with JSON Schema and serialized
in descriptors as `input_schema`.

Python input schema options are:

| Input schema | Behavior |
| --- | --- |
| omitted | Descriptor `input_schema` is `null`. |
| `ExplicitInputSchema(value={...})` | Uses the supplied JSON Schema mapping. |
| `PydanticInputSchema(model=Model)` | Calls `Model.model_json_schema()` and stores the generated schema. |

For custom factories or early registration adoption, omitting input schema is
valid. Public provider tasks should include input schema when practical so
CLIs, UIs, and catalog tooling can present expected input fields.

## Related Examples

- `genja/examples/task_registration.rs` shows the baseline registration,
  descriptor listing, schema, and direct construction path.
- `genja/examples/task_registration_custom_factory.rs` shows a custom factory
  for prepared JSON input and sanitized validation errors.
- `genja/examples/task_registration_spec.rs` shows YAML/JSON task spec parsing,
  construction, and retry/session verification overrides.
- `genja/examples/python/task_registration.py` shows Python imported-module
  registration, descriptor listing, and construction by `<id>@<version>`.

## Related Guides

- [Tasks](tasks.md)
- [Examples](examples.md)
- [API Surface](api-surface.md)
