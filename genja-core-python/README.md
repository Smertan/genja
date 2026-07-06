# genja-py

Python bindings for the Genja runtime.

This package exposes the `genja` module, which wraps the Rust runtime and
lets Python code:

- build a runtime from hosts, a full inventory, or a settings file
- run Python-authored tasks
- register Python plugins
- inspect raw and transformed inventory data

## Installation

For end users, install the package with `pip`:

```bash
pip install genja-py
```

The package currently exposes the `genja` Python module:

```python
import genja
```

## Quick Start

Create a runtime from a simple host mapping:

```python
import genja
from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


@task(name="backup_config")
class BackupTask:
    def start(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult:
        connection = context.connection()
        command_output = None
        if connection is not None:
            command_output = connection.execute_command("show running-config")

        return TaskSuccessResult(
            summary=f"backed up {host.hostname}",
            metadata={"show_running_config": command_output},
        )


genja = genja.Genja.from_hosts(
    {
        "router1": {"hostname": "10.0.0.1", "platform": "ios"},
        "router2": {"hostname": "10.0.0.2", "platform": "nxos"},
    }
).with_runner("serial")

results = genja.run_task(BackupTask)
print(results.to_dict())

tasks = genja.Tasks()
tasks.add_task(BackupTask)

all_results = genja.run_tasks(tasks)
print([result.task_name for result in all_results])
```

`results.to_dict()` returns per-host task results with an `outcome` payload and
separate `execution_metadata` for host timing and retry attempt information.

`TaskRuntimeContext` exposes the resolved connection through
`context.connection()` and `context.has_connection()`. Execution depth remains
internal to the runtime.

For async Python applications, use the async entrypoints:

```python
import asyncio
import genja
from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


@task(name="backup_config_async")
class BackupTaskAsync:
    async def start_async(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult:
        connection = context.connection()
        command_output = None
        if connection is not None:
            command_output = await connection.execute_command("show running-config")

        return TaskSuccessResult(
            summary=f"backed up {host.hostname}",
            metadata={"show_running_config": command_output},
        )


async def main() -> None:
    runtime = genja.Genja.from_hosts(
        {
            "router1": {"hostname": "10.0.0.1", "platform": "ios"},
        }
    ).with_runner("serial")

    results = await runtime.run_task_async(BackupTaskAsync)
    print(results.to_dict())


asyncio.run(main())
```

Use `run_task_async(...)` and `run_tasks_async(...)` when composing Genja with
`asyncio.gather(...)` or other async application code. The synchronous
`run_task(...)` and `run_tasks(...)` entrypoints remain available for scripts
and non-async callers.

Python task authoring rules:

- Define `def start(...)` for blocking tasks.
- Define `async def start_async(...)` for async tasks.
- Define exactly one of those methods on a `@task(...)` class.
- Use `sub_tasks=[ChildTask, ...]` to declare child tasks.

## Full Inventory

Use `genja.inventory` when you need groups and defaults:

```python
import genja
from genja.inventory import Defaults, Group, Host, Inventory

inventory = Inventory(
    hosts={
        "router1": Host(hostname="10.0.0.1", groups=["core"]),
    },
    groups={
        "core": Group(platform="ios", data={"role": "core"}),
    },
    defaults=Defaults(username="admin", port=22),
)

genja = genja.Genja.from_inventory(inventory)

print(genja.inventory_full())
print(genja.inventory_raw())
```

## Inventory Accessors

The runtime exposes three inventory views:

- `genja.inventory()`: raw hosts only
- `genja.inventory_full()`: transformed hosts, groups, and defaults
- `genja.inventory_raw()`: raw hosts, groups, and defaults

## Plugins

You can register Python plugins directly:

```python
import genja


class MyProcessorPlugin:
    name = "audit"
    group = "ProcessorPlugin"

    def on_task_finish(self, context, results):
        return None


plugins = genja.PluginManager()
plugins.register_plugin(MyProcessorPlugin())
```

Rust plugins can be loaded from a directory:

```python
plugins = genja.PluginManager()
plugins.load_rust_plugins_from_directory("./plugins")
```

## Settings Files

Build a runtime from a settings file:

```python
import genja

genja = genja.Genja.from_settings_file("config.yaml")
```

If you need Python plugins during settings-file loading, provide a plugin manager:

```python
plugins = genja.PluginManager()
genja = genja.Genja.from_settings_file("config.yaml", plugin_manager=plugins)
```

## Development

The commands below assume a repository checkout and use PDM-managed tooling.

Clone the repository and move into the Python package directory:

```bash
git clone git@github.com:Smertan/genja.git
cd genja/genja-core-python
```

Install the development dependencies:

```bash
pdm install -d
```

Build and install the Rust extension into the project virtual environment:

```bash
pdm run maturin develop
```

Run the Rust-side binding tests:

```bash
pdm run test-rust
```

Use `pdm run test-rust` instead of plain `cargo test`. The Rust tests embed
Python and need access to the PDM-managed virtualenv packages such as
`pydantic`.

Run the Python test suite:

```bash
pdm run test
```

Run Ruff:

```bash
pdm run lint
```
