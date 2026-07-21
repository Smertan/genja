# Plugins

Plugins extend Genja at runtime. They provide inventory sources, runners,
connection handlers, processors, and inventory transforms.

The plugin manager is the registry Genja uses to resolve plugin names from
settings and task metadata. Built-in plugins are registered automatically; custom
plugins are registered or loaded before the runtime is built.

## Plugin Identity

Every plugin has a name and a group. The name is how settings, tasks, or runtime
code select the plugin. The group tells Genja which plugin interface the object
implements.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use std::sync::Arc;
    use genja::genja_core::task::TaskProcessor;
    use genja_plugin_manager::plugin_types::{Plugin, PluginProcessor, Plugins};

    #[derive(Clone)]
    struct AuditPlugin;

    impl Plugin for AuditPlugin {
        fn name(&self) -> String {
            "audit".to_string()
        }
    }

    impl TaskProcessor for AuditPlugin {}

    impl PluginProcessor for AuditPlugin {
        fn processor(&self) -> Arc<dyn TaskProcessor> {
            Arc::new(self.clone())
        }
    }

    let plugin = Plugins::Processor(Box::new(AuditPlugin));
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.processor import ProcessorPluginBase


    class AuditPlugin(ProcessorPluginBase):
        name = "audit"
    ```

## Plugin Groups

Genja supports these plugin groups:

| Group | Purpose |
| --- | --- |
| `InventoryPlugin` | Loads hosts, groups, and defaults into inventory. |
| `RunnerPlugin` | Controls how tasks execute across selected hosts. |
| `ConnectionPlugin` | Creates and manages per-host connection sessions. |
| `ProcessorPlugin` | Runs lifecycle hooks around task execution and results. |
| `TransformFunctionPlugin` | Normalizes or enriches inventory values on access. |

Python base classes provide the correct group name automatically.

## Built-In Plugins

Every Genja runtime starts with these built-in plugins:

| Name | Group | Purpose |
| --- | --- | --- |
| `FileInventoryPlugin` | `InventoryPlugin` | Loads host, group, and default files. |
| `threaded` | `RunnerPlugin` | Runs host work concurrently with bounded async workers. |
| `serial` | `RunnerPlugin` | Runs host work one host at a time. |

Connection plugins, processors, and transforms are registered by user code or
loaded dynamically.

## Plugin Type Matrix

Use the specialized guide for full implementation details:

| Plugin type | Selected by | Rust trait | Python base class |
| --- | --- | --- | --- |
| Inventory | `inventory.plugin` | `PluginInventory` or `AsyncPluginInventory` | `InventoryPluginBase` |
| Runner | `runner.plugin` or `with_runner(...)` | `PluginRunner` | `RunnerPluginBase` |
| Connection | task `connection_plugin_name` | `PluginConnection` | `ConnectionPluginBase` |
| Processor | task `processors` | `PluginProcessor` and `TaskProcessor` | `ProcessorPluginBase` |
| Transform | `inventory.transform_function` | `PluginTransformFunction` | `TransformFunctionPluginBase` |

## Register Plugins

Register plugins before building the runtime.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja_core::inventory::{Hosts, Inventory};
    use genja_plugin_manager::plugin_types::Plugins;
    use genja_plugin_manager::PluginManager;

    let inventory = Inventory::builder().hosts(Hosts::new()).build();
    let mut plugins = PluginManager::new();
    plugins.register_plugin(Plugins::Processor(Box::new(AuditPlugin)));

    let genja = Genja::builder(inventory)
        .with_plugin_manager(plugins)
        .build()?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    plugins = genja_lib.PluginManager()
    plugins.register_plugin(AuditPlugin())

    genja = genja_lib.Genja.from_hosts(hosts, plugin_manager=plugins)
    ```

Python plugins should inherit from the matching base class. The base class
provides the locked `group` property and uses abstract methods for required
plugin behavior.

Plugin names are unique within a plugin manager. Registering a second Rust
plugin with the same name panics. Python registration returns an error for
invalid plugin identity and should be treated as setup-time validation.

In Python, `PluginManager` is consumed when it is passed into
`Genja.builder(...)`, `Genja.from_hosts(...)`, `Genja.from_settings(...)`, or
`Genja.from_settings_async(...)`, or `Genja.from_settings_file(...)`. Do not
reuse that manager afterward; create a new manager for another runtime.

Inspect registered plugins during setup:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    for (name, group) in plugins.get_all_plugin_names_and_groups() {
        println!("{name}: {group}");
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    for name, group in plugins.plugin_names_and_groups():
        print(f"{name}: {group}")
    ```

## Python Async Hooks

Python connection, runner, task, and transform hooks may be written as `def` or
`async def`. Genja resolves awaitable return values before handing them back to
the Rust runtime.

Inventory plugins use strict constructor matching: use `def load(...)` with
`Genja.from_settings(...)` for synchronous inventory loading, or
`async def load(...)` with `await Genja.from_settings_async(...)` or
`await Genja.from_settings_file_async(...)` for asynchronous inventory loading.

```python
from genja.inventory import InventoryPluginBase


class ApiInventory(InventoryPluginBase):
    name = "api_inventory"

    async def load(self, settings, plugins):
        return {
            "router1": {
                "hostname": "10.0.0.1",
                "platform": "ios",
            }
        }
```

Processor hooks are sync-only. They mirror the Rust `TaskProcessor` trait, so
implement `on_task_start`, `on_task_finish`, `on_instance_start`, and
`on_instance_finish` with normal `def` methods.

## Select Plugins

Settings select runtime plugins by name. The selected plugin must already be
registered.

```yaml
inventory:
  plugin: FileInventoryPlugin

runner:
  plugin: threaded
```

Tasks can also select plugin names in metadata. Processor names attach lifecycle
hooks to a task, and connection plugin names tell the task runtime which
connection type to resolve.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_task;

    struct BackupConfig;

    #[genja_task(
        name = "backup_config",
        connection_plugin_name = "ssh",
        processors = ["audit"],
    )]
    impl BackupConfig {
        async fn start_async(
            &self,
            _host: &genja::genja_core::inventory::Host,
            _context: &genja::genja_core::task::TaskRuntimeContext,
        ) -> Result<genja::genja_core::task::HostTaskResult, genja::genja_core::task::TaskError> {
            Ok(genja::genja_core::task::HostTaskResult::passed(
                genja::genja_core::task::TaskSuccess::new(),
            ))
        }
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import task


    @task(
        name="backup_config",
        connection_plugin_name="ssh",
        processors=["audit"],
    )
    class BackupConfig:
        ...
    ```

Inventory transforms are selected from the inventory section:

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml
  transform_function: normalize_inventory
```

## Load Plugins

Rust-authored plugins are loaded from compiled shared libraries.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja_plugin_manager::PluginManager;

    let plugins = PluginManager::new()
        .load_plugins_from_directory("./plugins")?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    plugins = genja_lib.PluginManager()
    plugins.load_rust_plugins_from_directory("./plugins")
    ```

Rust dynamic plugin libraries must export a `create_plugins` function that
returns `Vec<Plugins>`. The plugin manager keeps loaded libraries alive for as
long as the manager exists, so build the runtime with the same manager that
loaded the libraries.

End-user Rust applications can also declare plugin artifacts in
`Cargo.toml` metadata and copy them into `target/{PROFILE}/plugins` from
`build.rs` with `genja_plugin_manager::build_support::copy_plugins_from_manifest()`.
The `genja` runtime loads dynamic plugins from a `plugins` directory beside the
running executable.

Python-authored plugins can be registered directly or loaded from
`pyproject.toml` plugin entries.

```toml
[tool.genja.plugins.processor]
audit = "my_package.plugins:AuditProcessor"

[tool.genja.plugins.inventory]
api_inventory = "my_package.plugins:ApiInventory"

[tool.genja.plugins.runner]
canary = "my_package.plugins:CanaryRunner"
```

```python
plugins = genja_lib.PluginManager()
plugins.load_python_plugins_from_pyproject()
```

The manifest key must match the plugin object's `name` value. Use an explicit
path when the manifest is not the current directory's `pyproject.toml`:

```python
plugins.load_python_plugins_from_pyproject("packages/automation/pyproject.toml")
```

## Plugin Manager In The Runtime

The runtime uses the plugin manager at specific points:

- while loading inventory, it resolves the configured inventory plugin and any
  configured transform plugin
- while running tasks, it resolves the configured runner plugin
- while running a task instance, it resolves processor names and connection
  plugin names declared by that task

Inventory plugins receive access to registered plugins during load. Python
inventory plugins receive a read-only registry snapshot with plugin names and
groups, which is useful for validation and discovery without mutating the active
registry.

## Common Failures

- unknown plugin name: the plugin was not registered or loaded before runtime
  construction or task execution
- wrong plugin type: a name exists but belongs to a different plugin group
- duplicate plugin name: another plugin with the same name is already
  registered
- dynamic load failure: the shared library is missing, has the wrong extension,
  or does not export `create_plugins`
- Python pyproject mismatch: the manifest key does not match the plugin's
  declared `name`

## Detailed Guides

Plugin-specific behavior is documented in the relevant guide:

- Inventory plugins: [Inventory](../inventory.md)
- Transform plugins: [Transforms](../transforms.md)
- Task processors: [Processors](../processors.md)
- Runners: [Runners](../runners.md)
- Connection plugins: [Connections](../connection.md)
