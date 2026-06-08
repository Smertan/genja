# Examples

The repository includes small Rust and Python examples under `genja/examples`.
They use the shared inventory files in `genja/examples/inventory`.

Installed packages do not include the repository example source files. To run
them locally, clone the repository:

```bash
git clone https://github.com/Smertan/genja.git
cd genja
```

## Shared Data

The examples use:

- `genja/examples/settings.yaml`
- `genja/examples/inventory/hosts.yaml`
- `genja/examples/inventory/hosts.json`

The shared hosts inventory contains four hosts across different platforms and
site roles. Examples use this data to demonstrate loading, filtering, and task
execution without requiring real network devices.

## Rust Examples

Run Rust examples from the repository root:

```bash
cargo run -p genja --example basic_runtime
cargo run -p genja --example filter_hosts
cargo run -p genja --example run_task
cargo run -p genja --example run_task_tree
cargo run -p genja --example async_inventory_plugin
```

| Example | Demonstrates |
| --- | --- |
| `basic_runtime.rs` | Loading a runtime from `settings.yaml` and printing host IDs. |
| `filter_hosts.rs` | Filtering selected hosts with `filter_by_key_value(...)`. |
| `run_task.rs` | Defining a task with `#[genja_task]`, running it, and printing JSON results. |
| `run_task_tree.rs` | Defining sub-tasks and running a task tree with depth control. |
| `async_inventory_plugin.rs` | Implementing an async Rust inventory plugin and building a runtime from generated inventory. |

Use the Rust examples when you want to see the public `genja` crate, the
`#[genja_task]` macro, and Rust plugin traits in context.

## Python Examples

Install the Python package from the repository before running Python examples:

```bash
cd genja-core-python
maturin develop
cd ..
```

Run Python examples from the repository root:

```bash
python genja/examples/python/basic_runtime.py
python genja/examples/python/filter_hosts.py
python genja/examples/python/run_task.py
python genja/examples/python/run_task_tree.py
```

| Example | Demonstrates |
| --- | --- |
| `basic_runtime.py` | Loading hosts from JSON with `Genja.from_hosts(...)` and printing host IDs. |
| `filter_hosts.py` | Filtering selected hosts with `filter_by_key_value(...)`. |
| `run_task.py` | Defining a Python task with `@task`, running it, and printing JSON results. |
| `run_task_tree.py` | Defining Python sub-tasks and running a task tree with `max_depth`. |

See `genja/examples/python/README.md` for the local Python setup notes that live
beside the examples.

## Suggested Reading Order

Start with `basic_runtime`, then `filter_hosts`, then `run_task`. Read
`run_task_tree` after the task guide's sub-task section. Read
`async_inventory_plugin` after the inventory and plugin guides.

For fuller explanations, see:

- [Quickstart](quickstart.md)
- [Inventory](inventory.md)
- [Tasks](tasks.md)
- [Plugins](plugins/index.md)
