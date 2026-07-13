# Settings

Settings describe how a Genja runtime should be built: inventory source,
runner, SSH config validation, logging preferences, and core runtime behavior.
The same JSON or YAML settings files are used from Rust and Python.

Example files:

- `examples/config.example.yaml`
- `examples/config.example.json`

## Load Settings

### From A Settings File

Supported settings file extensions are `.json`, `.yaml`, and `.yml`.
Settings files are strict: unknown top-level sections and unknown fields inside
typed settings sections fail loading instead of being ignored. Correct
misspelled keys or remove unused keys when `Settings::from_file(...)` reports an
unknown field. Plugin-specific free-form values remain supported in explicit
option maps such as `runner.options` and `inventory.transform_function_options`.

For example, this YAML contains a misspelled runner key:

```yaml
runner:
  worker_counts: 10
```

The same mistake in JSON also fails:

```json
{
  "runner": {
    "worker_counts": 10
  }
}
```

Genja reports the section, unknown field, expected fields, and a suggestion when
there is a close match:

```text
unknown settings field
  section: `runner`
  field: `worker_counts`
  expected fields: `plugin`, `options`, `worker_count`, `max_task_depth`, `max_connection_attempts`, `retry`
  suggestion: did you mean `worker_count`?
```

Change the key to `worker_count`, remove unused keys, or move plugin-specific
values into a supported free-form object such as `runner.options`.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::Settings;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let settings = Settings::from_file("settings.yaml")?;

        println!("Runner plugin: {}", settings.runner().plugin());
        println!("Log level: {}", settings.logging().level());

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    settings = genja_lib.Settings.from_file("settings.yaml")

    print(f"Runner plugin: {settings.runner.plugin}")
    print(f"Log level: {settings.logging.level}")
    ```

### From Programmatic Settings

Code can also construct settings directly. Omitted fields keep the same defaults
used by file-loaded settings. Construction itself does not perform filesystem
validation, but runtime creation validates supplied settings before building the
runtime.

Use `Genja::from_settings(...)` or `Genja.from_settings(...)` when the runtime
should load inventory from `settings.inventory`:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja_core::Settings;
    use genja_core::settings::{InventoryConfig, OptionsConfig, RunnerConfig};

    let settings = Settings::builder()
        .inventory(
            InventoryConfig::builder()
                .options(
                    OptionsConfig::builder()
                        .hosts_file("./inventory/hosts.yaml")
                        .build(),
                )
                .build(),
        )
        .runner(RunnerConfig::builder().plugin("serial").build())
        .build();

    let runtime = Genja::from_settings(settings)?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja

    settings = genja.Settings(
        inventory=genja.InventoryConfig(
            options=genja.OptionsConfig(
                hosts_file="./inventory/hosts.yaml",
            ),
        ),
        runner=genja.RunnerConfig(plugin="serial"),
    )

    runtime = genja.Genja.from_settings(settings)
    ```

### With Explicit Inventory

When hosts or a full inventory are supplied explicitly, `from_hosts(...)` and
`from_inventory(...)` continue to use that explicit inventory instead of loading
from `settings.inventory`:

```python
import genja

settings = genja.Settings(
    runner=genja.RunnerConfig(
        plugin="serial",
        retry=genja.RunnerRetryConfig(
            allow=True,
            max_attempts=3,
            delay_ms=250,
        ),
    ),
)

runtime = genja.Genja.from_hosts(hosts, settings=settings)
```

### Validation Only

To validate programmatic settings explicitly before runtime construction, call:

```python
settings.validate()
```

## Complete Example

```yaml
core:
  # Parsed and exposed, but task failures are currently recorded in results
  # regardless of this value.
  raise_on_error: false

inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./inventory/hosts.yaml
    groups_file: ./inventory/groups.yaml
    defaults_file: ./inventory/defaults.yaml
  transform_function: normalize_inventory
  transform_function_options:
    hostname_suffix: ".lab"

ssh:
  config_file: /home/user/.ssh/config

runner:
  plugin: threaded
  options: {}
  worker_count: 10
  max_task_depth: 10
  max_connection_attempts: 3
  retry:
    allow: false
    max_attempts: 1
    delay_ms: 0

logging:
  enabled: true
  level: info
  log_file: ./genja.log
  to_console: false
  file_size: 10485760
  max_file_count: 10
```

## Precedence

Configuration is loaded in this order:

1. Config file values.
2. Environment variables used by default functions.
3. Hard-coded defaults.

Environment variables are fallback defaults for missing values. They do not
override explicit values in a settings file.

Boolean settings accept real booleans and loose string values such as
`true`, `false`, `1`, `0`, `yes`, `no`, `on`, and `off`.

## Top-Level Sections

These top-level sections are supported:

| Section | Type | Default when omitted |
| --- | --- | --- |
| `core` | object | `CoreConfig::default()` |
| `inventory` | object | `InventoryConfig::default()` |
| `ssh` | object | `SSHConfig::default()` |
| `runner` | object | `RunnerConfig::default()` |
| `logging` | object | `LoggingConfig::default()` |

Top-level sections are optional. Nested fields have the defaults listed below.
Unknown top-level sections are rejected.
Current implementation note: if the `inventory` section is present, include an
`options` object; use `options: {}` when the selected inventory plugin does not
need file paths.

## Core

```yaml
core:
  raise_on_error: false
```

| Field | Type | Default | Env fallback |
| --- | --- | --- | --- |
| `raise_on_error` | bool | `false` | `GENJA_CORE_RAISE_ON_ERROR` |

`raise_on_error` is currently parsed and exposed through settings, including its
environment fallback, but task execution does not branch on this value. Task
entrypoint errors are recorded as failed host results, and `run_task(...)` /
`run_tasks(...)` can still return `Ok(...)` when hosts failed. Outer runtime
errors such as invalid plugins, processor hook errors, runner errors, and config
loading errors still return errors regardless of this field.

## Inventory

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml
    groups_file: ./groups.yaml
    defaults_file: ./defaults.yaml
  transform_function: normalize_inventory
  transform_function_options:
    hostname_suffix: ".lab"
```

| Field | Type | Default | Env fallback |
| --- | --- | --- | --- |
| `plugin` | string | `FileInventoryPlugin` | `GENJA_INVENTORY_PLUGIN` |
| `options` | object | `{}` | none |
| `transform_function` | string or null | `null` | none |
| `transform_function_options` | object or null | `null` | none |

### Inventory Options

The built-in `FileInventoryPlugin` reads these options:

| Field | Type | Default |
| --- | --- | --- |
| `hosts_file` | string or null | `null` |
| `groups_file` | string or null | `null` |
| `defaults_file` | string or null | `null` |

Inventory files must be JSON (`.json`) or YAML (`.yaml`, `.yml`).
Unknown fields inside `inventory.options` are rejected. Put arbitrary
transform-specific values under `inventory.transform_function_options`.

Hosts and groups files are maps keyed by name:

```yaml
router1:
  hostname: 10.0.0.1
  port: 22
  username: admin
  groups:
    - core
  data:
    role: edge
```

Defaults files contain one object:

```yaml
username: admin
platform: linux
data:
  retries: 3
```

Hosts and groups support:

- `hostname` (string or null)
- `port` (number or null)
- `username` (string or null)
- `password` (string or null)
- `platform` (string or null)
- `groups` (list of strings or null)
- `data` (object or null)
- `connection_options` (map of string to object or null)

Defaults support the same fields except `groups`.

See [Inventory](inventory.md) for file examples, group inheritance, filtering,
and connection option precedence.

## SSH

```yaml
ssh:
  config_file: /home/user/.ssh/config
```

| Field | Type | Default | Env fallback |
| --- | --- | --- | --- |
| `config_file` | string or null | `null` | none |

When `config_file` is set, `Settings::from_file(...)` checks that the file
exists, can be opened, and parses as strict OpenSSH-style config. Programmatic
settings can be checked with `Settings::validate()` in Rust, `settings.validate()`
in Python, or `settings.ssh.validate()` in Python for the SSH section only.
Python runtime creation also validates supplied settings before building the
runtime. If the field is omitted or `null`, SSH config validation is skipped.

## Runner

```yaml
runner:
  plugin: threaded
  options: {}
  worker_count: 10
  max_task_depth: 10
  max_connection_attempts: 3
  retry:
    allow: false
    max_attempts: 1
    delay_ms: 0
```

| Field | Type | Default | Env fallback |
| --- | --- | --- | --- |
| `plugin` | string | `threaded` | `GENJA_RUNNER_PLUGIN` |
| `options` | object | `{}` | none |
| `worker_count` | number or null | `null` | none |
| `max_task_depth` | number | `10` | none |
| `max_connection_attempts` | number | `3` | none |
| `retry.allow` | boolean | `false` | none |
| `retry.max_attempts` | number | `1` | none |
| `retry.delay_ms` | number | `0` | none |

Common `plugin` values are `threaded` and `serial`.

`runner.options` is a free-form object for runner plugin-specific settings.
Unknown fields inside typed runner settings, including `runner.retry`, are
rejected.

`worker_count` is used by runners that support fixed concurrency. The built-in
`threaded` runner uses it as the maximum number of in-flight host executions.
The built-in `serial` runner ignores it.

`max_task_depth` is the default nested sub-task depth used when the runtime API
does not receive an explicit depth. Rust task execution APIs currently require
an explicit depth argument. Python task execution APIs can omit `max_depth` to
use this setting.

`max_connection_attempts` is part of shared runner configuration. Connection
plugins or connection layers are responsible for interpreting retry behavior.

`retry.allow` controls whether retries are allowed by default for task
execution. `retry.max_attempts` controls the default total attempt count for a
task, including the first attempt. `retry.delay_ms` is a fixed in-process delay
before retry attempts. Tasks may override runner retry defaults field by field
through task metadata. Retries are only attempted for failures explicitly marked
as retryable.

See [Runners](runners.md) for execution behavior and ordering details.

## Logging

```yaml
logging:
  enabled: true
  level: info
  log_file: ./genja.log
  to_console: false
  file_size: 10485760
  max_file_count: 10
```

| Field | Type | Default | Env fallback |
| --- | --- | --- | --- |
| `enabled` | bool | `true` | none |
| `level` | string | `info` | `GENJA_LOGGING_LEVEL` |
| `log_file` | string | current directory `genja.log` | `GENJA_LOGGING_LOG_FILE` |
| `to_console` | bool | `false` | `GENJA_LOGGING_TO_CONSOLE` |
| `file_size` | number | `10485760` | none |
| `max_file_count` | number | `10` | none |

Genja parses the `logging` section but does not install output handlers,
formats, colors, or log rotation automatically. Rust applications own global
logger setup. Python applications receive Rust-side Genja records through
Python's standard `logging` system and should configure Python handlers and
levels before running tasks.

A typical application startup flow is:

1. Load settings with `Settings::from_file(...)`.
2. Read the `logging` section.
3. Initialize the application's logger.
4. Build and run `Genja`.

Because settings must be loaded before `settings.logging()` is available, logs
emitted during settings loading may occur before a logger is initialized. Genja
keeps normal default fallbacks silent for this reason. Invalid config file values
fail settings loading. Invalid environment variable values may fall back to
defaults and emit a warning only if logging has already been initialized. In
Python, configure logging before loading settings if you need to capture
warnings emitted during settings loading.

## Environment Variables

| Variable | Used by | Default when unset |
| --- | --- | --- |
| `GENJA_CORE_RAISE_ON_ERROR` | `core.raise_on_error` | `false` |
| `GENJA_INVENTORY_PLUGIN` | `inventory.plugin` | `FileInventoryPlugin` |
| `GENJA_RUNNER_PLUGIN` | `runner.plugin` | `threaded` |
| `GENJA_LOGGING_LEVEL` | `logging.level` | `info` |
| `GENJA_LOGGING_LOG_FILE` | `logging.log_file` | current directory `genja.log` |
| `GENJA_LOGGING_TO_CONSOLE` | `logging.to_console` | `false` |

## Troubleshooting

See [Logging And Troubleshooting](logging-troubleshooting.md) for logger setup,
settings load failures, inventory file issues, plugin load failures, runner
errors, and empty host selections.

## References

The source of truth for defaults and deserialization behavior is:

- `genja-core/src/settings.rs`
- `genja-core/src/settings/`
