# Plugins

Plugins extend Genja at runtime. They provide inventory sources, runners,
connection handlers, processors, and inventory transforms.

## Plugin Identity

Every plugin has a name and a group. The name is how settings, tasks, or runtime
code select the plugin. The group tells Genja which plugin interface the object
implements.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja_plugin_manager::plugin_types::Plugin;

    struct AuditPlugin;

    impl Plugin for AuditPlugin {
        fn name(&self) -> String {
            "audit".to_string()
        }

        fn group(&self) -> String {
            "ProcessorPlugin".to_string()
        }
    }
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

## Register Plugins

Register plugins before building the runtime.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja_core::inventory::{Hosts, Inventory};
    use genja_plugin_manager::PluginManager;

    let inventory = Inventory::builder().hosts(Hosts::new()).build();
    let plugins = PluginManager::new();
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

## Python Async Hooks

Python inventory, connection, runner, task, and transform hooks may be written
as `def` or `async def`. Genja resolves awaitable return values before handing
them back to the Rust runtime.

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

Python-authored plugins can be registered directly or loaded from
`pyproject.toml` plugin entries.

```toml
[tool.genja.plugins.processor]
audit = "my_package.plugins:AuditProcessor"
```

```python
plugins = genja_lib.PluginManager()
plugins.load_python_plugins_from_pyproject()
```

## Detailed Guides

Plugin-specific behavior is documented in the relevant guide:

- Inventory plugins: [Inventory](inventory.md)
- Transform plugins: [Transforms](transforms.md)
- Task processors: [Processors](processors.md)
- Runners: [Runners](runners.md)
- Connection plugins: [Connections](connection.md)
