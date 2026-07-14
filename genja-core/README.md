# genja-core

`genja-core` contains the shared runtime primitives used by Genja's Rust and
Python-facing crates.

Most applications should depend on the higher-level `genja` crate. Use
`genja-core` directly when you need to build integrations around Genja's core
types, such as inventory models, settings loading, task definitions, task
results, connection state, or processor behavior.

## Features

- Inventory types for hosts, groups, host variables, and connection metadata
- Settings models and file loading for JSON and YAML configuration
- Task traits, task metadata, task runtime context, and structured task results
  with semantic outcomes plus host execution metadata
- Connection state and task connection resolver traits for runtime integrations
- Processor result handling used by task execution
- Re-exported task authoring macros from `genja-core-derive`

## Installation

```toml
[dependencies]
genja-core = "0.3.0"
```

## Example

Load settings from a configuration file:

```rust
use genja_core::Settings;

let settings = Settings::from_file("config.yaml")?;
# Ok::<(), genja_core::SettingsError>(())
```

Settings files are strict: unknown fields in typed settings sections fail
loading instead of being ignored. Use explicit option maps such as
`runner.options` for plugin-specific values.

Build inventory data in code:

```rust
use genja_core::inventory::{BaseBuilderHost, Host, Hosts, Inventory};

let mut hosts = Hosts::new();
hosts.add_host("router1", Host::builder().hostname("10.0.0.1").build());

let inventory = Inventory::builder().hosts(hosts).build();

assert_eq!(inventory.hosts().len(), 1);
```

## Task Authoring

`genja-core` re-exports the `#[genja_task(...)]` macro used to define task
metadata and runtime entrypoints.

```rust
use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{
    HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
};

struct CheckTask;

#[genja_task(name = "check", connection_plugin_name = "ssh")]
impl CheckTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}
```

For complete application examples, see the
[quickstart guide](https://docs.genja.co.uk/quickstart/) and the top-level
`genja` crate.

Per-host task results are represented by `HostTaskResult`, which separates the
semantic task outcome from host-level execution metadata such as timing and
retry attempt state.
