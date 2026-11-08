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
misspelled keys or remove unused keys when settings loading reports an
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

Both Rust and Python show the same settings diagnostic for the section, unknown
field, expected fields, and a close-match suggestion:

```text
unknown settings field
  section: `runner`
  field: `worker_counts`
  expected fields: `plugin`, `options`, `worker_count`, `max_task_depth`, `max_connection_attempts`, `task_timeout`, `task_retry`
  suggestion: did you mean `worker_count`?
```

The complete Rust error starts with `failed to deserialize settings from
<path>:`. Python raises a `ValueError` that also starts with `failed to load
settings from <path>:` before the Rust error text.

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

Use the language's `from_settings` constructor when the runtime should load
inventory from `settings.inventory`:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja::genja_core::Settings;
    use genja::genja_core::settings::{InventoryConfig, OptionsConfig, RunnerConfig};

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

Use async runtime construction when the selected inventory plugin is async.
Async construction is strict: the selected plugin must be async-capable; use
the sync constructors for sync-only inventory plugins such as
`FileInventoryPlugin`.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja::genja_core::Settings;
    use genja::genja_core::settings::InventoryConfig;

    async fn build_runtime() -> Result<Genja, Box<dyn std::error::Error>> {
        // Assumes `api_inventory` is available through runtime plugin discovery.
        let settings = Settings::builder()
            .inventory(InventoryConfig::builder().plugin("api_inventory").build())
            .build();

        let runtime = Genja::from_settings_async(settings).await?;
        Ok(runtime)
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja

    class ApiInventoryPlugin(genja.InventoryPluginBase):
        name = "api_inventory"

        async def load(self, settings, plugins):
            return {
                "router1": {
                    "hostname": "10.0.0.1",
                    "platform": "ios",
                },
            }

    plugins = genja.PluginManager()
    plugins.register_plugin(ApiInventoryPlugin())

    settings = genja.Settings(
        inventory=genja.InventoryConfig(plugin="api_inventory"),
    )

    runtime = await genja.Genja.from_settings_async(settings, plugin_manager=plugins)

    # Or load settings from a file with the same strict async inventory contract.
    runtime = await genja.Genja.from_settings_file_async(
        "settings.yaml",
        plugin_manager=plugins,
    )
    ```

### With Explicit Inventory

When an inventory is supplied explicitly, runtime construction uses it instead
of loading from `settings.inventory`:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja::genja_core::inventory::{Hosts, Inventory};

    let inventory = Inventory::builder().hosts(Hosts::new()).build();
    let runtime = Genja::from_inventory(inventory);
    ```

    `from_inventory(...)` starts with default settings. Configure the runtime
    separately if you need other settings.

=== ":fontawesome-brands-python: Python"

    ```python
    import genja

    hosts = {"router1": {"hostname": "10.0.0.1", "platform": "ios"}}

    settings = genja.Settings(
        runner=genja.RunnerConfig(
            plugin="serial",
            task_retry=genja.RunnerTaskRetryConfig(
                allow=True,
                max_attempts=3,
                delay_ms=250,
            ),
        ),
    )

    runtime = genja.Genja.from_hosts(hosts, settings=settings)
    ```

    `from_hosts(...)` and `from_inventory(...)` accept explicit settings.

### Validation Only

To validate programmatic settings explicitly before runtime construction:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    settings.validate()?;
    ```

=== ":fontawesome-brands-python: Python"

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
  task_timeout: null # optional seconds per task and host
  task_retry:
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
| `core` | object | Core defaults listed below |
| `inventory` | object | Inventory defaults listed below |
| `ssh` | object | SSH defaults listed below |
| `runner` | object | Runner defaults listed below |
| `logging` | object | Logging defaults listed below |

Top-level sections are optional. Nested fields have the defaults listed below,
including when only part of a section is configured. Unknown top-level sections
are rejected.

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
entrypoint errors are recorded as failed host results, so inspect host results
even when a task-running call succeeds. Outer runtime errors such as invalid
plugins, processor hook errors, runner errors, and config loading errors still
surface regardless of this field.

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

When `config_file` is set, loading settings from a file checks that the SSH
config exists, can be opened, and parses as strict OpenSSH-style config. If the
field is omitted or `null`, SSH config validation is skipped.

=== ":fontawesome-brands-rust: Rust"

    Call `settings.validate()` to check programmatic settings before building
    the runtime.

=== ":fontawesome-brands-python: Python"

    Call `settings.validate()` for programmatic settings or
    `settings.ssh.validate()` for the SSH section only. Python runtime creation
    also validates supplied settings.

## Runner

```yaml
runner:
  plugin: threaded
  options: {}
  worker_count: 10
  max_task_depth: 10
  max_connection_attempts: 3
  task_timeout: null # optional seconds per task and host
  task_retry:
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
| `task_timeout` | positive seconds or null | `null` | none |
| `task_retry.allow` | boolean | `false` | none |
| `task_retry.max_attempts` | number | `1` | none |
| `task_retry.delay_ms` | number | `0` | none |
| `task_retry.policy` | `no_retry`, `fixed`, or `exponential` | `fixed` when omitted | none |
| `task_retry.factor` | number at least `2` | `2` for exponential | none |
| `task_retry.max_delay_ms` | number from `1` to `86400000` | `86400000` for exponential | none |

Common `plugin` values are `threaded` and `serial`.

`runner.options` is a free-form object for runner plugin-specific settings.
Unknown fields inside typed runner settings are rejected. The old
`runner.retry` key is rejected; rename it to `runner.task_retry`.

`worker_count` is used by runners that support fixed concurrency. The built-in
`threaded` runner uses it as the maximum number of in-flight host executions.
The built-in `serial` runner ignores it.

`max_task_depth` is the default nested sub-task depth used when the runtime API
does not receive an explicit depth. Rust task execution APIs currently require
an explicit depth argument. Python task execution APIs can omit `max_depth` to
use this setting.

`max_connection_attempts` is part of shared runner configuration. Connection
plugins or connection layers are responsible for interpreting retry behavior.

`task_timeout` sets the runner default for one task on one host. A task's
declared timeout takes precedence, followed by the invocation value and then
the runner value. When all three are omitted, no timeout is configured. A child
task can declare its own value; it does not inherit a timeout declared only by
its parent. Deadline enforcement uses one monotonic budget for connection
setup, checks, attempts, retry waits, and verification for each task on each
host. A retry does not reset that budget. Expiry produces a non-retryable host
timeout with structured `configured_limit_seconds`, `elapsed_seconds`, `phase`,
and `attempt` details.
An expiry before the first task attempt reports zero attempts. Blocking work
is observed to finish locally before another task step is started; the timeout
result does not claim that remote work was terminated.
In an ordered task list, later roots get fresh deadlines on every host,
including hosts where an earlier root or subtask timed out.

Timeout values may include fractions of a second, such as `task_timeout: 0.5`.

### Task timeout entry points

Select a language to see its entry points. The precedence is the same for
each task on each host.

=== ":fontawesome-brands-rust: Rust"

    ```mermaid
    flowchart TD
        TASK["#[genja_task(task_timeout = ...)]"] --> HAS_TASK{"Task timeout set?"}
        HAS_TASK -- Yes --> USE_TASK["Use task timeout"]
        HAS_TASK -- No --> OPTIONS["TaskRunOptions::new(...).with_task_timeout(...)"]
        OPTIONS --> HAS_OPTIONS{"Invocation timeout set?"}
        HAS_OPTIONS -- Yes --> USE_OPTIONS["Use invocation timeout"]
        HAS_OPTIONS -- No --> RUNNER["RunnerConfig::builder().task_timeout(...)<br/>or settings.yaml runner.task_timeout"]
        RUNNER --> HAS_RUNNER{"Runner timeout set?"}
        HAS_RUNNER -- Yes --> USE_RUNNER["Use runner timeout"]
        HAS_RUNNER -- No --> NO_DEADLINE["No deadline"]
        USE_TASK --> DEADLINE["New deadline for this task and host"]
        USE_OPTIONS --> DEADLINE
        USE_RUNNER --> DEADLINE
        DEADLINE --> LIFECYCLE["Connection → checks → attempts → retry waits → verification"]
    ```

    Pass `TaskRunOptions` to `run_task_with_options(...)` or
    `run_tasks_with_options(...)`. The simpler methods that accept `max_depth`
    create options without a timeout, leaving the runner value available as a
    fallback. Runner and invocation APIs take a `Duration`; for example,
    `Duration::from_secs_f64(0.5)` sets half a second. The task attribute takes
    a number of seconds, such as `#[genja_task(task_timeout = 0.5)]`.

=== ":fontawesome-brands-python: Python"

    ```mermaid
    flowchart TD
        TASK["@task(task_timeout=...)"] --> HAS_TASK{"Task timeout set?"}
        HAS_TASK -- Yes --> USE_TASK["Use task timeout"]
        HAS_TASK -- No --> OPTIONS["TaskRunOptions(task_timeout=...)"]
        OPTIONS --> HAS_OPTIONS{"Invocation timeout set?"}
        HAS_OPTIONS -- Yes --> USE_OPTIONS["Use invocation timeout"]
        HAS_OPTIONS -- No --> RUNNER["RunnerConfig(task_timeout=...)<br/>or settings.yaml runner.task_timeout"]
        RUNNER --> HAS_RUNNER{"Runner timeout set?"}
        HAS_RUNNER -- Yes --> USE_RUNNER["Use runner timeout"]
        HAS_RUNNER -- No --> NO_DEADLINE["No deadline"]
        USE_TASK --> DEADLINE["New deadline for this task and host"]
        USE_OPTIONS --> DEADLINE
        USE_RUNNER --> DEADLINE
        DEADLINE --> LIFECYCLE["Connection → checks → attempts → retry waits → verification"]
    ```

    Pass `TaskRunOptions` as `run_options=` to `run_task(...)`,
    `run_task_async(...)`, or their task-list equivalents. Timeout arguments
    take seconds, including fractions such as `@task(task_timeout=0.5)` or
    `RunnerConfig(task_timeout=0.5)`.

`task_retry.allow` controls whether retries are allowed by default for task
execution. `task_retry.max_attempts` controls the default total attempt count for a
task, including the first attempt. `task_retry.delay_ms` is the base delay in
milliseconds. With no policy selected, the fixed-delay behavior is unchanged.
`no_retry` stops after one attempt; `exponential` multiplies the base delay by
`factor` after each retry, capped at `max_delay_ms` for each wait. A task can
select its own policy and override the common fields independently. Selecting
a policy never enables retries or overrides a non-retryable failure.

For the full root and subtask selection order, see
[Runner retry precedence](runners.md#task-retry-precedence).

For example, these runner defaults allow six attempts with waits of 1, 3, 9,
27, and 60 seconds when each earlier attempt fails retryably:

```yaml
runner:
  task_retry:
    allow: true
    max_attempts: 6
    delay_ms: 1000
    policy: exponential
    factor: 3
    max_delay_ms: 60000
```

The equivalent programmatic runner defaults are:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::settings::RunnerConfig;
    use genja::genja_core::task::{TaskRetryConfig, TaskRetryPolicyKind};

    let runner = RunnerConfig::builder()
        .task_retry(
            TaskRetryConfig::builder()
                .allow(true)
                .max_attempts(6)
                .delay_ms(1000)
                .policy(TaskRetryPolicyKind::Exponential)
                .factor(3)
                .max_delay_ms(60000)
                .build(),
        )
        .build();
    ```

    `TaskRetryConfig` is also used for task metadata; placing it in
    `RunnerConfig::builder().task_retry(...)` sets runner defaults.

=== ":fontawesome-brands-python: Python"

    ```python
    import genja

    runner = genja.RunnerConfig(
        task_retry=genja.RunnerTaskRetryConfig(
            allow=True,
            max_attempts=6,
            delay_ms=1000,
            policy=genja.TaskRetryPolicyKind.EXPONENTIAL,
            factor=3,
            max_delay_ms=60000,
        )
    )
    ```

    `RunnerTaskRetryConfig` sets runner defaults in Python.

### Migrating task retry names from v0.5.x

The task retry names change in v0.6.0. Replace the old names directly; there
are no compatibility aliases.

In YAML and JSON settings, rename `runner.retry` to `runner.task_retry`.

=== ":fontawesome-brands-rust: Rust"

    | v0.5.x | v0.6.0 |
    | --- | --- |
    | `RetryConfig` | `TaskRetryConfig` |
    | `RetryConfigBuilder` | `TaskRetryConfigBuilder` |
    | `RunnerConfig::retry()` | `RunnerConfig::task_retry()` |
    | `RunnerConfigBuilder::retry(...)` | `RunnerConfigBuilder::task_retry(...)` |

    The task macro `retry(...)` and `TaskSpec.overrides.retry` keep their
    task-scoped names.

=== ":fontawesome-brands-python: Python"

    | v0.5.x | v0.6.0 |
    | --- | --- |
    | Task `RetryConfig` | `TaskRetryConfig` |
    | `RunnerRetryConfig` | `RunnerTaskRetryConfig` |
    | `RunnerConfig(retry=...)` | `RunnerConfig(task_retry=...)` |
    | `RunnerConfig.retry` | `RunnerConfig.task_retry` |

    The task decorator `retry=TaskRetryConfig(...)` keeps its task-scoped name.

Task descriptor and execution-result field names also stay the same.

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
formats, colors, or log rotation automatically.

=== ":fontawesome-brands-rust: Rust"

    The application owns global logger setup. Initialize it before running
    tasks if you need to capture Genja records.

=== ":fontawesome-brands-python: Python"

    Rust-side Genja records flow through Python's standard `logging` system.
    Configure Python handlers and levels before running tasks. Configure
    logging before loading settings if you also need warnings from settings
    loading.

A typical application startup flow is:

1. Load settings from a file or construct them programmatically.
2. Read the `logging` section.
3. Initialize the application's logger.
4. Build and run `Genja`.

Because logging settings are available only after settings load, records
emitted during loading may precede logger initialization. Genja keeps normal
default fallbacks silent for this reason. Invalid config file values fail
settings loading. Invalid environment variable values may fall back to defaults
and emit a warning only if logging has already been initialized.

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
