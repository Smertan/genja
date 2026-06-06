# Logging And Troubleshooting

This guide covers logging setup and the most common runtime failure modes:
settings loading, inventory files, plugin resolution, runner execution, and
empty host selections.

## Logging Model

Genja emits logs through the Rust `log` facade, but it does not initialize a
global logger. Applications own logger setup because logging is process-wide in
Rust and application-specific in Python.

The `logging` settings section is parsed and exposed for applications to use:

```yaml
logging:
  enabled: true
  level: info
  log_file: ./genja.log
  to_console: false
  file_size: 10485760
  max_file_count: 10
```

Genja does not automatically create `log_file`, install console logging, or
apply log rotation. Read these values and initialize the logger that fits your
application.

## Initialize Logging

For normal application startup, load settings first, then initialize logging
from `settings.logging()`.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::Settings;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let settings = Settings::from_file("settings.yaml")?;

        if settings.logging().enabled() {
            env_logger::Builder::from_env(
                env_logger::Env::default()
                    .default_filter_or(settings.logging().level()),
            )
            .init();
        }

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import logging

    import genja as genja_lib

    settings = genja_lib.Settings.from_file("settings.yaml")

    if settings.logging.enabled:
        logging.basicConfig(
            level=getattr(logging, settings.logging.level.upper()),
        )
    ```

Logs emitted while settings are loading can happen before settings-backed
logging is available. When troubleshooting settings loading itself, initialize a
temporary logger before calling `Settings::from_file(...)`.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::Settings;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        env_logger::Builder::from_env(
            env_logger::Env::default().default_filter_or("debug"),
        )
        .init();

        let settings = Settings::from_file("settings.yaml")?;
        println!("configured log level: {}", settings.logging().level());

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import logging

    import genja as genja_lib

    logging.basicConfig(level=logging.DEBUG)
    settings = genja_lib.Settings.from_file("settings.yaml")

    print(f"configured log level: {settings.logging.level}")
    ```

## Settings Load Failures

`Settings::from_file(...)` accepts only `.json`, `.yaml`, and `.yml` files.
Other extensions return an unsupported format error.

Common settings failures:

- unsupported extension: use `.json`, `.yaml`, or `.yml`
- unreadable file: check the path and permissions
- deserialize failure: check field names, field types, and YAML/JSON syntax
- SSH config failure: `ssh.config_file` exists but cannot be opened or parsed

If `ssh.config_file` is set, Genja validates that file during settings load. Set
the field to `null` or omit it when you do not want SSH config validation.

## Inventory File Issues

The built-in `FileInventoryPlugin` reads paths from `inventory.options`.

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml
    groups_file: ./groups.yaml
    defaults_file: ./defaults.yaml
```

Common inventory failures:

- file does not exist or cannot be read
- unsupported inventory extension
- invalid JSON or YAML syntax
- unknown host or group fields
- transform plugin configured but not registered
- transform plugin name points to a plugin with the wrong type

Inventory files must use `.json`, `.yaml`, or `.yml`. Hosts and groups are maps
keyed by name. Defaults is one object.

If inventory loads but a task runs on no hosts, inspect the selected hosts:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    for host_id in genja.host_ids() {
        println!("{host_id}");
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    for host_id in genja.host_ids():
        print(host_id)
    ```

## Plugin Load And Resolution Failures

Settings and task metadata select plugins by name. The plugin must be registered
with the correct plugin type before the runtime needs it.

Common plugin failures:

- `plugin 'name' not found`: the plugin was not registered or loaded
- `plugin 'name' is not an inventory plugin`: the name exists with the wrong type
- `plugin 'name' is not a runner plugin`: `runner.plugin` points to the wrong type
- `transform plugin 'name' not found`: inventory transform plugin is missing
- dynamic load failure: the shared library is missing, invalid, or lacks `create_plugins`
- Python `pyproject.toml` mismatch: manifest key does not match plugin `name`

Inspect registered plugins before building or running:

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

## Runner Errors

Use `run_task_async(...)` or `run_tasks_async(...)` when already inside an async
runtime. Rust sync wrappers return an error if called from an active Tokio
runtime.

Task entrypoint failures are recorded as failed host results. They do not
normally make `run_task(...)` return an outer error.

Outer runner errors usually mean one of these happened:

- configured runner plugin was missing or the wrong type
- processor name resolution failed
- processor hook returned an error
- custom runner returned an error
- threaded worker task failed before producing host results

Check both the outer `Result` and the returned task results. A successful outer
`Ok(TaskResults)` can still contain failed hosts.

## Why Did Nothing Run?

Check these in order:

1. `host_ids()` is empty because inventory loaded no hosts.
2. A filter returned a runtime with no selected hosts.
3. `max_depth` is `0`, so sub-tasks did not run.
4. The task was skipped by task logic.
5. The task selected a connection plugin that failed to resolve or open.
6. You called a sync Rust API inside Tokio and got an outer error before task
   execution started.

Empty host selections are reportable, not fatal. Runners can return a task
result object with no host results.

## Result Checks

After a run, inspect host summaries before assuming the workflow succeeded:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let summary = results.task_summary();
    let hosts = summary.hosts();

    println!(
        "passed={} failed={} skipped={}",
        hosts.passed(),
        hosts.failed(),
        hosts.skipped(),
    );
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    print(results.host_summary())
    print(results.to_json(pretty=True))
    ```

Use [Settings](settings.md) for the field reference, [Inventory](inventory.md)
for inventory structure, [Plugins](plugins.md) for registration and loading, and
[Runners](runners.md) for execution behavior.
