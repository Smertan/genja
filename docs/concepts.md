# Concepts

Genja runs task logic against an inventory of hosts. The runtime loads hosts,
selects which hosts should receive work, executes tasks with a runner, and
returns structured results.

## Runtime

The runtime is the main entry point for execution. It owns the active settings,
loaded plugins, loaded inventory, selected hosts, and runner configuration.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;

    let genja = Genja::from_settings_file("settings.yaml")?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    genja = genja_lib.Genja.from_settings_file("settings.yaml")
    ```

Most applications build one runtime, optionally filter it, then run one or more
tasks. Filtering returns another runtime value with the same inventory and
plugins but a narrower selected host list.

## Hosts

A host represents one target in the inventory. Hosts can contain connection
details, platform information, group membership, and arbitrary `data` used for
selection or task logic.

```yaml
router1:
  hostname: 10.0.0.1
  platform: ios
  groups:
    - core
  data:
    site:
      name: core
```

Host names come from the inventory map key. In this example, the host ID is
`router1`.

Host fields can be authored directly on the host or inherited from defaults and
groups. When Genja resolves a host for execution, values are applied in this
order:

1. inventory defaults
2. groups, in the order listed on the host
3. the host itself

Later values override earlier scalar values. Object-shaped `data` values are
merged, with later keys taking precedence.

## Inventory

Inventory is the complete set of hosts, groups, and defaults available to the
runtime. Inventory can be loaded from files or supplied directly by code.

The built-in file inventory plugin reads JSON or YAML files. A typical settings
file points the inventory plugin at a hosts file:

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml
```

Groups and defaults let you avoid repeating common values across hosts.

The file inventory plugin accepts separate hosts, groups, and defaults files.
Custom inventory plugins can return the same model from code when inventory
comes from an API, database, generated source, or another system.

## Selection

Selection narrows the loaded inventory to the hosts that should receive work.
Filtering does not remove hosts from the underlying inventory; it changes the
active host selection used by task execution.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let core_site = genja.filter_by_key_value("data.site.name", "^core$")?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    core_site = genja.filter_by_key_value("data.site.name", "^core$")
    ```

Filters serialize each host to JSON and can match nested keys with dot paths.
`filter_by_key(...)` checks that a key exists. `filter_by_key_value(...)`
matches the selected value with a regular expression.

## Tasks

A task is the unit of work Genja runs for each selected host. Task code receives
the current host and a runtime context, then returns a host-level result.

Tasks can also define sub-tasks. Sub-tasks let you model a small execution tree,
such as deploy, validate, then collect logs.

A task result is host-scoped. The parent task can pass for one host and fail or
skip for another. Sub-task results are stored below the task that declared them,
so the final result has the same shape as the task tree.

## Runners

A runner controls how tasks are executed across selected hosts. Genja includes
built-in runners:

- `serial`: runs work one host at a time.
- `threaded`: runs work concurrently across hosts.

Runner behavior is configured through settings:

```yaml
runner:
  plugin: threaded
  worker_count: 10
  max_task_depth: 10
```

`max_task_depth` controls how far sub-tasks are followed. Use `0` to execute
only the root task, `1` to include direct children, and higher values for deeper
task trees.

## Plugins

Plugins extend runtime behavior. Genja uses plugins for inventory loading,
runners, connection handling, processors, and transforms.

The plugin manager registers plugins and lets the runtime resolve them by name.
The settings file selects plugin names for parts of the runtime, such as
inventory and runner plugins.

Connection plugins are selected per task. When a task declares a connection
plugin, the runtime resolves the host's connection parameters and exposes the
opened connection through `TaskRuntimeContext`.

## Results

Task execution returns structured results. Results include:

- task name
- per-host status
- per-host summary
- metadata
- messages
- nested sub-task results

Use results for reporting, logging, automation decisions, or test assertions.

The normalized JSON form is meant for consumers that want stable field names.
Raw result output preserves the internal Rust enum-like shape and is usually
more useful for debugging.

## Settings

Settings describe how the runtime should be built: inventory source, runner,
SSH config, core behavior, and logging preferences.

Settings are shared across Rust and Python. The file format is the same, while
the language APIs differ slightly. Rust uses accessor methods; Python uses
properties.

See [Settings](settings.md) for the full schema.
