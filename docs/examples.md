# Examples

The repository includes small Rust and Python examples under `genja/examples`.
They use the shared inventory files in `genja/examples/inventory`.

**The commands on this page require a local checkout of the Genja GitHub
repository.** Adding `genja` with `cargo add` does not make its example
executables available through `cargo run --example` in your own project.
Clone the repository, then run the commands from its root:

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
cargo run -p genja --example task_registration
cargo run -p genja --example task_registration_custom_factory
cargo run -p genja --example task_registration_spec
```

| Example | Demonstrates |
| --- | --- |
| `basic_runtime.rs` | Loading a runtime from `settings.yaml` and printing host IDs. |
| `filter_hosts.rs` | Filtering selected hosts with `filter_by_key_value(...)`. |
| `run_task.rs` | Defining a task with `#[genja_task]`, running it, and printing JSON results. |
| `idempotent_task.rs` | Defining a task-authored convergence check that skips already-converged hosts. |
| `run_task_tree.rs` | Defining sub-tasks and running a task tree with depth control. |
| `async_inventory_plugin.rs` | Implementing an async Rust inventory plugin and building a runtime from generated inventory. |
| `task_registration.rs` | Registering a Rust task, listing compiled descriptors, printing schema JSON, and constructing by `<id>@<version>`. |
| `task_registration_custom_factory.rs` | Registering a Rust task with a custom factory for prepared JSON input and sanitized validation errors. |
| `task_registration_spec.rs` | Constructing a registered Rust task from YAML and JSON task spec strings, including retry and session verification overrides. |
| `task_browser.rs` | Exploring CLI task listing and descriptor inspection, or navigating, searching, and inspecting two sample tasks in the optional TUI. |

Use the Rust examples when you want to see the public `genja` crate, the
`#[genja_task]` macro, and Rust plugin traits in context.

### CLI And TUI Task Browser

Run the commands below inside the repository checkout described above.
If you are using Genja as a dependency in your own project, follow the
[project-local CLI binary pattern](cli.md#project-local-cli-binary-rust-and-mixed-projects) instead.

The `task_browser` example links two sample registrations into its own
executable and delegates command handling to `genja::cli::run_main()`.
The sample tasks do not perform backups or connect to hosts, and are not
registered in the normal `genja` binary or libraries.

Run the CLI commands with the `genja-cli` feature:

```bash
cargo run -p genja --features genja-cli --example task_browser -- task list
cargo run -p genja --features genja-cli --example task_browser -- task describe acme.examples.backup_config@1.0.0
```

`backup_config` is blocking and constructible, with a JSON input schema.
`collect_facts` is async and descriptor-only, with a generated local ID and no
construction factory. To describe it, copy its ID and version from `task list`
and pass them in `<id>@<version>` form to `task describe`.
Neither command executes a task. Add `--output json` to inspect the full
descriptor data.

For the terminal UI, enable `genja-tui`, which also enables CLI support:

```bash
cargo run -p genja --features genja-tui --example task_browser -- tui
```

Run this in an interactive terminal. The TUI
loads the two descriptors and shows their fields in a table with the selected
row highlighted. Use Up/Down or `k`/`j` to navigate. Press `/` and type `backup`
to show only `backup_config`; press Enter to return to navigation with the query
retained. Escape then clears it. While editing, Ctrl+u clears the query without
leaving search. Press `q` in navigation mode to exit.

Follow the [Rust tab of the search walkthrough](tui.md#try-search) to try
case-insensitive matching, execution-mode and description searches, and no-match
recovery. See the [Terminal UI guide](tui.md) for feature setup and the
project-local binary pattern.

Press Enter on a selected task to open Details, then Tab to inspect its schema.
`backup_config` includes JSON schema metadata; `collect_facts` shows the missing
schema message. Scroll with Up/Down, PageUp/PageDown, or Home/End; Escape returns
to the list while retaining selection and search. Follow the
[Rust tab of the descriptor inspection walkthrough](tui.md#try-descriptor-inspection)
for a complete example.

### Mixed Rust And Python Discovery

The Python examples include a [task module](../genja/examples/python/task_registration_tasks.py)
and a [pyproject.toml](../genja/examples/python/pyproject.toml) that declares it:

```toml
[tool.genja.tasks]
modules = ["task_registration_tasks"]
```

Install `genja-py` in the Python environment used for discovery and select that
environment's interpreter with `--python` if it is not the `python` on `PATH`.
From the repository root, the generic `genja` binary can list the Python tasks
without linking Rust registrations:

```bash
genja --pyproject genja/examples/python/pyproject.toml task list
```

Run the Rust browser example against the same project file to see both
languages in one list:

```bash
cargo run -p genja --features genja-cli --example task_browser -- \
  --pyproject genja/examples/python/pyproject.toml task list
```

The Rust and Python examples both register
`acme.examples.backup_config@1.0.0`. The list retains both implementations.
Use `rust:acme.examples.backup_config@1.0.0` or
`python:acme.examples.backup_config@1.0.0` with `task describe` to select one;
an unqualified lookup is ambiguous. With `genja-tui` enabled, the same project
file opens one searchable TUI list containing both languages:

```bash
cargo run -p genja --features genja-tui --example task_browser -- \
  --pyproject genja/examples/python/pyproject.toml tui
```

See the [installation guide](installation.md#python-task-discovery-from-the-cli)
for the v0.5.0 Python CLI installation. The
[Python tab of the TUI walkthrough](tui.md#try-descriptor-inspection)
shows inspection and search with the same configured task module.

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
python genja/examples/python/task_registration.py
```

| Example | Demonstrates |
| --- | --- |
| `basic_runtime.py` | Loading hosts from JSON with `Genja.from_hosts(...)` and printing host IDs. |
| `filter_hosts.py` | Filtering selected hosts with `filter_by_key_value(...)`. |
| `run_task.py` | Defining a Python task with `@task`, running it, and printing JSON results. |
| `run_task_tree.py` | Defining Python sub-tasks and running a task tree with `max_depth`. |
| `task_registration.py` | Importing a module with registered Python tasks, listing descriptors, and constructing by `<id>@<version>`. |

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
- [Task Registration](task-registration.md)
- [Plugins](plugins/index.md)
