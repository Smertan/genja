# Genja

## Quick Start

Use a JSON or YAML config file and load it with `Settings::from_file`:

```rust
use genja_core::Settings;

let settings = Settings::from_file("config.yaml")?;
```

Build a `Genja` instance with inventory + settings:

```rust
use genja::Genja;
use genja_core::Settings;
use genja_core::inventory::{Inventory, Hosts, Host, BaseBuilderHost};

let mut hosts = Hosts::new();
hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
let inventory = Inventory::builder().hosts(hosts).build();

let genja = Genja::builder(inventory)
    .with_settings(Settings::from_file("config.yaml")?)
    .build()?;
```

## Running Tasks

Tasks are defined in `genja_core::task` and executed through `Genja::run`.
The recommended pattern is:

1. Define a struct for the task.
2. Derive `Task` to generate `TaskInfo` and `SubTasks`.
3. Implement `genja_core::task::Task` and return a `HostTaskResult` from `start()`.
4. Run the task with `Genja::run(task, max_depth)`.

```rust
use genja::Genja;
use genja_core::inventory::{BaseBuilderHost, Host, Inventory, Hosts};
use genja_core::task::{HostTaskResult, Task, TaskSuccess};
use genja_core_derive::Task as TaskDerive;

#[derive(TaskDerive)]
struct CheckConfigTask {
    name: String,
    plugin_name: Option<String>,
}

impl Task for CheckConfigTask {
    fn start(&self, _host: &Host) -> HostTaskResult {
        HostTaskResult::passed(
            TaskSuccess::new()
                .with_summary("configuration is present")
                .with_changed(false),
        )
    }
}

let mut hosts = Hosts::new();
hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
let inventory = Inventory::builder().hosts(hosts).build();

let genja = Genja::builder(inventory).build()?;

let results = genja.run(
    CheckConfigTask {
        name: "check_config".to_string(),
        plugin_name: Some("ssh".to_string()),
    },
    10,
)?;

assert!(results.host_result("router1").unwrap().is_passed());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Notes:

- `max_depth` limits recursive sub-task execution. A task with no sub-tasks can use a small value like `1`.
- `#[derive(TaskDerive)]` requires a `name` field. `plugin_name` is optional, but usually needed for real task execution.
- Rich task output lives in `TaskSuccess`, `TaskFailure`, `TaskSkip`, and `TaskResults`.
- The lower-level task API is documented in `genja-core/src/task.rs`.

### Sub-Tasks

Sub-tasks are declared as `Arc<dyn Task>` fields marked with `#[task(subtask)]`.
They execute after the parent task and their results are stored under `TaskResults::sub_task(...)`.

```rust
use std::sync::Arc;

use genja::Genja;
use genja_core::inventory::{BaseBuilderHost, Host, Inventory, Hosts};
use genja_core::task::{HostTaskResult, Task, TaskSuccess};
use genja_core_derive::Task as TaskDerive;

#[derive(TaskDerive)]
struct ValidateTask {
    name: String,
    plugin_name: Option<String>,
}

impl Task for ValidateTask {
    fn start(&self, _host: &Host) -> HostTaskResult {
        HostTaskResult::passed(TaskSuccess::new().with_summary("validation passed"))
    }
}

#[derive(TaskDerive)]
struct DeployTask {
    name: String,
    plugin_name: Option<String>,
    #[task(subtask)]
    validate: Arc<dyn Task>,
}

impl Task for DeployTask {
    fn start(&self, _host: &Host) -> HostTaskResult {
        HostTaskResult::passed(TaskSuccess::new().with_summary("deployment complete"))
    }
}

let mut hosts = Hosts::new();
hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
let inventory = Inventory::builder().hosts(hosts).build();
let genja = Genja::builder(inventory).build()?;

let task = DeployTask {
    name: "deploy".to_string(),
    plugin_name: Some("ssh".to_string()),
    validate: Arc::new(ValidateTask {
        name: "validate".to_string(),
        plugin_name: Some("ssh".to_string()),
    }),
};

let results = genja.run(task, 2)?;

assert!(results.host_result("router1").unwrap().is_passed());
assert!(results.sub_task("validate").is_some());
assert!(
    results
        .sub_task("validate")
        .unwrap()
        .host_result("router1")
        .unwrap()
        .is_passed()
);
# Ok::<(), Box<dyn std::error::Error>>(())
```

Example configuration files:

- `examples/config.example.yaml`
- `examples/config.example.json`

Settings reference:

- `docs/settings.md`

## Configuration Precedence And Error Handling

Configuration is loaded from two sources in this order:

1. Config file values (JSON/YAML).
2. Environment variables (used only by default functions, not to override explicit config).

Behavior rules:

- If a config field is explicitly provided and is invalid, deserialization fails with an error.
- If a config field is missing, a default value is used.
- For defaults that read environment variables, invalid env values trigger a warning and the default fallback is used.
- Environment variables do not override explicitly provided config values.

Current environment variables:

- `GENJA_CORE_RAISE_ON_ERROR` (bool, loose parsing: `true/false`, `1/0`, `yes/no`, `on/off`)
- `GENJA_INVENTORY_PLUGIN` (string)
- `GENJA_RUNNER_PLUGIN` (string)
- `GENJA_LOGGING_LEVEL` (string)
- `GENJA_LOGGING_LOG_FILE` (string path)
- `GENJA_LOGGING_TO_CONSOLE` (bool, loose parsing: `true/false`, `1/0`, `yes/no`, `on/off`)
