# genja-core-python

Python bindings for the Genja runtime.

This package exposes the `genja_core` module, which wraps the Rust runtime and
lets Python code:

- build a runtime from hosts, a full inventory, or a settings file
- run Python-authored tasks
- register Python plugins
- inspect raw and transformed inventory data

## Installation

For end users, install the package with `pip`:

```bash
pip install genja-core-python
```

## Quick Start

Create a runtime from a simple host mapping:

```python
import genja_core
from genja_core.task import TaskSuccessResult, task


@task(name="backup_config")
class BackupTask:
    def run(self, task, host, context):
        return TaskSuccessResult(summary=f"backed up {host.hostname}")


genja = genja_core.Genja.from_hosts(
    {
        "router1": {"hostname": "10.0.0.1", "platform": "ios"},
        "router2": {"hostname": "10.0.0.2", "platform": "nxos"},
    }
).with_runner("serial")

results = genja.run_task(BackupTask)
print(results.to_dict())
```

## Full Inventory

Use `genja_core.inventory` when you need groups and defaults:

```python
import genja_core
from genja_core.inventory import Defaults, Group, Host, Inventory

inventory = Inventory(
    hosts={
        "router1": Host(hostname="10.0.0.1", groups=["core"]),
    },
    groups={
        "core": Group(platform="ios", data={"role": "core"}),
    },
    defaults=Defaults(username="admin", port=22),
)

genja = genja_core.Genja.from_inventory(inventory)

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
import genja_core


class MyProcessorPlugin:
    def name(self) -> str:
        return "audit"

    def group(self) -> str:
        return "ProcessorPlugin"

    def on_task_finish(self, context, results):
        return None


plugins = genja_core.PluginManager()
plugins.register_plugin(MyProcessorPlugin())
```

Rust plugins can be loaded from a directory:

```python
plugins = genja_core.PluginManager()
plugins.load_rust_plugins_from_directory("./plugins")
```

## Settings Files

Build a runtime from a settings file:

```python
import genja_core

genja = genja_core.Genja.from_settings_file("config.yaml")
```

If you need Python plugins during settings-file loading, provide a plugin manager:

```python
plugins = genja_core.PluginManager()
genja = genja_core.Genja.from_settings_file("config.yaml", plugin_manager=plugins)
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
cargo test -p genja-core-python
```

Run the Python test suite:

```bash
pdm run test
```

Run Ruff:

```bash
pdm run lint
```
