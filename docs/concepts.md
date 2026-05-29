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
    import genja

    runtime = genja.Genja.from_settings_file("settings.yaml")
    ```

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
    core_site = runtime.filter_by_key_value("data.site.name", "^core$")
    ```

## Tasks

A task is the unit of work Genja runs for each selected host. Task code receives
the current host and a runtime context, then returns a host-level result.

Tasks can also define sub-tasks. Sub-tasks let you model a small execution tree,
such as deploy, validate, then collect logs.

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

## Plugins

Plugins extend runtime behavior. Genja uses plugins for inventory loading,
runners, connection handling, processors, and transforms.

The plugin manager registers plugins and lets the runtime resolve them by name.
The settings file selects plugin names for parts of the runtime, such as
inventory and runner plugins.

## Results

Task execution returns structured results. Results include:

- task name
- per-host status
- per-host summary
- metadata
- messages
- nested sub-task results

Use results for reporting, logging, automation decisions, or test assertions.

## Settings

Settings describe how the runtime should be built: inventory source, runner,
SSH config, core behavior, and logging preferences.

Settings are shared across Rust and Python. The file format is the same, while
the language APIs differ slightly. Rust uses accessor methods; Python uses
properties.

See [Settings](settings.md) for the full schema.

