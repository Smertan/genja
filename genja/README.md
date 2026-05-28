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
```

## License

Genja is licensed under AGPL-3.0-only.
