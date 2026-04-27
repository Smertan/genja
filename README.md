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

## Filtering Hosts

`Genja` keeps the inventory immutable and returns a new instance with a reduced
host selection when filtering. Use `filter_by_key` when a key only needs to
exist, and `filter_by_key_value` when the value must match a regex.

Plain keys are searched recursively across host fields and nested `data`.
Dot paths can be used to target a specific path.

```rust
use genja::Genja;
use genja_core::inventory::{BaseBuilderHost, Data, Host, Hosts, Inventory};
use serde_json::json;

let mut hosts = Hosts::new();
hosts.add_host(
    "router1",
    Host::builder()
        .hostname("10.0.0.1")
        .data(Data::new(json!({
            "site": {
                "name": "data_center",
                "role": "core"
            }
        })))
        .build(),
);
hosts.add_host(
    "router2",
    Host::builder()
        .hostname("10.0.0.2")
        .data(Data::new(json!({
            "site": {
                "name": "branch",
                "role": "edge"
            }
        })))
        .build(),
);

let inventory = Inventory::builder().hosts(hosts).build();
let genja = Genja::from_inventory(inventory);

let with_site = genja.filter_by_key("site")?;
assert_eq!(with_site.host_ids().len(), 2);

let data_center = genja.filter_by_key_value("data.site.name", "^data_center$")?;
assert_eq!(data_center.host_ids().len(), 1);
assert_eq!(data_center.host_ids()[0].as_str(), "router1");
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Running Tasks

Tasks are defined in `genja_core::task` and executed through `Genja::run`.
The recommended pattern is:

1. Define a struct for the task.
2. Derive `Task` to generate `TaskInfo` and `SubTasks`.
3. Implement `genja_core::task::Task` and return a `HostTaskResult` from `start()`.
4. Run the task with `Genja::run(task, max_depth)`.

### Derive Macro

`#[derive(TaskDerive)]` does not implement the full `Task` trait for you.
It generates `TaskInfo` and `SubTasks`, then you provide the execution logic by manually implementing `Task`.

The derive macro maps fields like this:

- `name` is required and becomes `TaskInfo::name()`.
- `plugin_name` is optional and becomes `TaskInfo::plugin_name()`.
- `options` is optional and becomes `TaskInfo::options()`.
- `processor_names` is optional and becomes `TaskInfo::processor_names()`.
- `#[task(processors = ["audit"])]` can be used when processor names are fixed at compile time.
- Fields marked with `#[task(subtask)]` are collected into `SubTasks::sub_tasks()` in declaration order.

That means the usual pattern is:

1. Add `#[derive(TaskDerive)]` to the task struct.
2. Declare `name`, and optionally `plugin_name`, `options`, `processor_names`, and `#[task(subtask)]` fields.
3. Implement `Task::start(&self, host)` manually.

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
- `#[derive(TaskDerive)]` requires a `name` field and generates `TaskInfo` plus `SubTasks`, not `Task::start()`.
- `plugin_name` is optional, but usually needed for real task execution.
- Rich task output lives in `TaskSuccess`, `TaskFailure`, `TaskSkip`, and `TaskResults`.
- The lower-level task API is documented in `genja-core/src/task.rs`.

### Task Processor Plugins

Processor plugins run lifecycle hooks before and after selected tasks and task instances.
Processor names are resolved by `PluginManager`, and invalid names return `GenjaError::PluginNotFound`.
Tasks opt into processors by name:

```rust
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, Task, TaskSuccess};
use genja_core_derive::Task as TaskDerive;

#[derive(TaskDerive)]
#[task(processors = ["audit"])]
struct DeployTask {
    name: &'static str,
    plugin_name: Option<String>,
}

impl Task for DeployTask {
    fn start(&self, _host: &Host) -> HostTaskResult {
        HostTaskResult::passed(TaskSuccess::new())
    }
}
```

A processor plugin returns a `TaskProcessor` implementation:

```rust
use genja_core::task::{TaskProcessor, TaskProcessorContext, TaskResults};
use genja_plugin_manager::plugin_types::{Plugin, PluginProcessor, Plugins};
use std::sync::Arc;

#[derive(Debug)]
struct AuditProcessorPlugin;

impl Plugin for AuditProcessorPlugin {
    fn name(&self) -> String {
        "audit".to_string()
    }
}

impl PluginProcessor for AuditProcessorPlugin {
    fn processor(&self) -> Arc<dyn TaskProcessor> {
        Arc::new(AuditProcessor)
    }
}

struct AuditProcessor;

impl TaskProcessor for AuditProcessor {
    fn on_task_finish(
        &self,
        context: &TaskProcessorContext,
        results: &mut TaskResults,
    ) -> Result<(), genja_core::GenjaError> {
        let _ = (context, results);
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub fn create_plugins() -> Vec<Plugins> {
    vec![Plugins::Processor(Box::new(AuditProcessorPlugin))]
}
```

### Task Execution Rules

- `Genja::run` executes the full task tree once per selected host.
- The parent task runs before any of its sub-tasks.
- The parent host result is recorded before sub-task execution starts.
- Sub-tasks run in the order returned by `sub_tasks()`. With `#[derive(TaskDerive)]`, that is the declaration order of `#[task(subtask)]` fields.
- Sub-task results are stored under `results.sub_task("<name>")` and grouped by sub-task name across hosts.
- Sub-tasks are not automatically skipped when a parent fails or is skipped. If you need that behavior, encode it in the task and return a skipped result explicitly.
- Depth is zero-based. The root task runs at depth `0`, its direct children at depth `1`, and so on.
- Because the limit check is inclusive of the current depth, `max_depth = 0` allows only the root task, while `max_depth = 1` allows one level of sub-tasks.

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
