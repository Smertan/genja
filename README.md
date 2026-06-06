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

## Async Inventory Plugins

Rust supports both synchronous and asynchronous inventory plugins.

- Use `PluginInventory` for synchronous loaders.
- Use `AsyncPluginInventory` for asynchronous loaders.

If you are loading inventory through runtime plugin discovery from a settings
file, use `Genja::from_settings_file_async(...)` when the selected inventory
plugin is async-capable. If you are registering inventory plugins in code, load
the inventory through the plugin first and then build `Genja` from the returned
`Inventory`.

```rust
use genja::Genja;
use genja::async_trait;
use genja::genja_core::inventory::{BaseBuilderHost, Host, Hosts, Inventory};
use genja::genja_core::{InventoryLoadError, Settings};
use genja_plugin_manager::plugin_types::{AsyncPluginInventory, Plugin, Plugins};
use genja_plugin_manager::PluginManager;

#[derive(Debug)]
struct ApiInventoryPlugin;

impl Plugin for ApiInventoryPlugin {
    fn name(&self) -> String {
        "api_inventory".to_string()
    }
}

#[async_trait]
impl AsyncPluginInventory for ApiInventoryPlugin {
    async fn load_async(
        &self,
        _settings: &Settings,
        _plugins: &PluginManager,
    ) -> Result<Inventory, InventoryLoadError> {
        let mut hosts = Hosts::new();
        hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
        Ok(Inventory::builder().hosts(hosts).build())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::from_file("config.yaml")?;

    let mut plugins = PluginManager::new();
    plugins.register_plugin(Plugins::AsyncInventory(Box::new(ApiInventoryPlugin)));

    let inventory = plugins
        .get_async_inventory_plugin("api_inventory")
        .ok_or("missing async inventory plugin")?
        .load_async(&settings, &plugins)
        .await?;

    let genja = Genja::builder(inventory)
        .with_settings(settings)
        .with_plugin_manager(plugins)
        .build()?;

    assert_eq!(genja.host_ids().len(), 1);
    Ok(())
}
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

Tasks are defined in `genja_core::task`. A single root task tree is executed
through `Genja::run_task` or `Genja::run_task_async`; an ordered list of root
task trees is executed through `Genja::run_tasks` or `Genja::run_tasks_async`.
The recommended pattern for a single task is:

1. Define a struct for the task.
2. Add `#[genja_task(...)]` to an inherent `impl` block.
3. Define exactly one entrypoint:
   `fn start(...)` for blocking or `async fn start_async(...)` for async.
4. Run the task with `Genja::run_task(task, max_depth)` from sync code, or
   `Genja::run_task_async(task, max_depth).await` from an active Tokio runtime.

```rust
use genja::{Genja, genja_task};
use genja_core::inventory::{BaseBuilderHost, Host, Inventory, Hosts};
use genja_core::task::{HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess};

struct CheckConfigTask;

#[genja_task(name = "check_config", connection_plugin_name = "ssh")]
impl CheckConfigTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new()
                .with_summary("configuration is present")
                .with_changed(false),
        ))
    }
}

let mut hosts = Hosts::new();
hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
let inventory = Inventory::builder().hosts(hosts).build();

let genja = Genja::builder(inventory).build()?;

let results = genja.run_task(CheckConfigTask, 10)?;

assert!(results.host_result("router1").unwrap().is_passed());
# Ok::<(), Box<dyn std::error::Error>>(())
```

Async Rust applications should use the async execution APIs directly:

```rust
use genja::Genja;

#[tokio::main]
async fn main() -> Result<(), genja::GenjaError> {
    let genja = Genja::from_settings_file("settings.yaml")?;
    let results = genja
        .run_task_async(
            CheckConfigTask,
            10,
        )
        .await?;

    assert!(results.host_result("router1").unwrap().is_passed());
    Ok(())
}
```

The sync wrappers `run_task(...)` and `run_tasks(...)` return an error when
called from an active Tokio runtime. Use `run_task_async(...)` and
`run_tasks_async(...)` in async contexts.

Notes:

- `max_depth` limits recursive sub-task execution. A task with no sub-tasks can use a small value like `1`.
- `#[genja_task(...)]` owns static task metadata like name, processors, and connection plugin selection.
- `connection_plugin_name` is optional, but usually needed for real task execution.
- Rich task output lives in `TaskSuccess`, `TaskFailure`, `TaskSkip`, and `TaskResults`.
- The lower-level task API is documented in `genja-core/src/task.rs`.

### Task Processor Plugins

Processor plugins run lifecycle hooks before and after selected tasks and task instances.
Processor names are resolved by `PluginManager`, and invalid names return `GenjaError::PluginNotFound`.
Tasks opt into processors by name:

```rust
use genja::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess};

struct DeployTask;

#[genja_task(name = "deploy", processors = ["audit"])]
impl DeployTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
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

- `Genja::run_task` executes one full task tree once per selected host.
- `Genja::run_tasks` executes an ordered `Tasks` list. Each root task may have its own sub-task tree, and the returned `Vec<TaskResults>` preserves root task order.
- The parent task runs before any of its sub-tasks.
- The parent host result is recorded before sub-task execution starts.
- Sub-tasks run in the order returned by `sub_tasks()`.
- Sub-task results are stored under `results.sub_task("<name>")` and grouped by sub-task name across hosts.
- Sub-tasks are not automatically skipped when a parent fails or is skipped. If you need that behavior, encode it in the task and return a skipped result explicitly.
- Depth is zero-based. The root task runs at depth `0`, its direct children at depth `1`, and so on.
- Because the limit check is inclusive of the current depth, `max_depth = 0` allows only the root task, while `max_depth = 1` allows one level of sub-tasks.

### Sub-Tasks

Sub-tasks are returned from `fn sub_tasks(&self) -> Vec<Arc<dyn Task>>`. They
execute after the parent task and their results are stored under
`TaskResults::sub_task(...)`.

```rust
use std::sync::Arc;

use genja::{Genja, genja_task};
use genja_core::inventory::{BaseBuilderHost, Host, Inventory, Hosts};
use genja_core::task::{HostTaskResult, Task, TaskError, TaskRuntimeContext, TaskSuccess};

struct ValidateTask;

#[genja_task(name = "validate", connection_plugin_name = "ssh")]
impl ValidateTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary("validation passed"),
        ))
    }
}

struct DeployTask;

#[genja_task(name = "deploy", connection_plugin_name = "ssh")]
impl DeployTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary("deployment complete"),
        ))
    }

    fn sub_tasks(&self) -> Vec<Arc<dyn Task>> {
        vec![Arc::new(ValidateTask)]
    }
}

let mut hosts = Hosts::new();
hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
let inventory = Inventory::builder().hosts(hosts).build();
let genja = Genja::builder(inventory).build()?;

let task = DeployTask;

let results = genja.run_task(task, 2)?;

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
