# Genja

Genja is a plugin-based Rust framework for executing tasks across multiple
hosts. It combines inventory management, host filtering, hierarchical task
execution, dynamic plugin loading, and structured result tracking behind the
`Genja` runtime.

Use this crate as the main entry point for building and running Genja
automation workflows.

## Features

- Build a runtime from explicit inventory or settings files
- Filter hosts by fields or nested data before execution
- Run single tasks or ordered task lists across selected hosts
- Execute hierarchical task trees with bounded recursion depth
- Load Genja-compatible inventory, runner, connection, and processor plugins
- Track per-host task results, sub-task results, summaries, timing, and status

## Installation

```toml
[dependencies]
genja = "0.1.0"
```

The `genja` crate re-exports the common task authoring pieces so most users do
not need to depend on `genja-core`, `genja-core-derive`, or `async-trait`
directly.

## Quick Start

```rust
use genja::Genja;
use genja::genja_core::Settings;
use genja::genja_core::inventory::{
    BaseBuilderHost, // brings Host::builder() into scope
    Host,
    Hosts,
    Inventory,
};

let mut hosts = Hosts::new();
hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());

let inventory = Inventory::builder().hosts(hosts).build();

let genja = Genja::builder(inventory)
    .with_settings(Settings::default())
    .build()?;

assert_eq!(genja.host_ids().len(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Note

`Host::builder()` is provided by the `BaseBuilderHost` trait, so the trait must
be imported anywhere you use the builder helper.

## Async Inventory Loading

Rust inventory plugins can be synchronous (`PluginInventory`) or asynchronous
(`AsyncPluginInventory`).

When the inventory plugin is discovered from the runtime plugin directory and
selected by `settings.yaml`, use `Genja::from_settings_file_async(...)` for the
async path. When you register inventory plugins in code, load the inventory
explicitly through the plugin and then build `Genja` from the returned
`Inventory`.

```rust
use genja::Genja;
use genja::async_trait;
use genja::genja_core::Settings;
use genja::genja_core::inventory::{
    BaseBuilderHost, // brings Host::builder() into scope
    Host,
    Hosts,
    Inventory,
};
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
    ) -> Result<Inventory, genja::genja_core::InventoryLoadError> {
        let mut hosts = Hosts::new();
        hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());
        Ok(Inventory::builder().hosts(hosts).build())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let settings = Settings::from_file("settings.yaml")?;

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

## Running A Task

Task metadata and sub-task relationships are generated with `TaskDerive`; task
behavior is implemented manually with the `Task` trait.

```rust
use genja::Genja;
use genja::TaskDerive;
use genja::async_trait;
use genja::genja_core::inventory::{
    BaseBuilderHost, // brings Host::builder() into scope
    Host,
    Hosts,
    Inventory,
};
use genja::genja_core::task::{HostTaskResult, Task, TaskError, TaskRuntimeContext, TaskSuccess};

#[derive(TaskDerive)]
struct CheckConfig {
    name: &'static str,
}

#[async_trait]
impl Task for CheckConfig {
    async fn start(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(
            TaskSuccess::new().with_summary("configuration checked"),
        ))
    }
}

let mut hosts = Hosts::new();
hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());

let inventory = Inventory::builder().hosts(hosts).build();
let genja = Genja::builder(inventory).build()?;

let results = genja.run_task(CheckConfig { name: "check_config" }, 1)?;

assert!(results.host_result("router1").unwrap().is_passed());
# Ok::<(), Box<dyn std::error::Error>>(())
```

For async Rust applications, use `run_task_async(...)` instead of `run_task(...)`:

```rust
use genja::Genja;

#[tokio::main]
async fn main() -> Result<(), genja::GenjaError> {
    let genja = Genja::from_settings_file("settings.yaml")?;
    let results = genja
        .run_task_async(CheckConfig { name: "check_config" }, 1)
        .await?;

    assert!(results.host_result("router1").unwrap().is_passed());
    Ok(())
}
```

The sync wrappers `run_task(...)` and `run_tasks(...)` return an error when
called from an active Tokio runtime. Use `run_task_async(...)` and
`run_tasks_async(...)` in async contexts.

## Related Crates

- `genja-core`: core inventory, settings, task, connection, and result types
- `genja-core-derive`: derive macros for task metadata and sub-task discovery
- `genja-plugin-manager`: dynamic plugin loading and build support

## Examples

Examples are included with the crate source and repository. Adding `genja` to an
application with `cargo add genja` does not copy these examples into that
application; clone the repository to run them.

- Repository: <https://github.com/Smertan/genja>
- Runnable crate examples: `genja/examples/`
- Shared example inventory: `genja/examples/inventory/hosts.json`
- Shared example settings: `genja/examples/settings.yaml`
- YAML inventory variant: `genja/examples/inventory/hosts.yaml`
- Sample configuration: `examples/`

Run a crate example from a repository checkout:

The command below uses the repository default branch. For release-specific
examples, check out the matching version tag, such as `v0.1.0`.

```bash
git clone https://github.com/Smertan/genja.git
cd genja
cargo run -p genja --example run_task
cargo run -p genja --example async_inventory_plugin
```

## License

Genja is licensed under AGPL-3.0-only.
