# Settings

This document summarizes the Genja configuration schema and links to examples.

Example file:

- `examples/config.example.yaml`
- `examples/config.example.json`

## Precedence

Configuration is loaded in this order:

1. Config file values (JSON/YAML)
2. Environment variables (used only by default functions, not to override explicit config)
3. Hard-coded defaults

## Top-Level Sections

All sections are optional. Missing fields use defaults.

- `core`
- `inventory`
- `ssh`
- `runner`
- `logging`

## Core

- `raise_on_error` (bool)
  - Default: `false`
  - Env fallback: `GENJA_CORE_RAISE_ON_ERROR` (loose bool parsing: `true/false`, `1/0`, `yes/no`, `on/off`)

## Inventory

- `plugin` (string)
  - Default: `FileInventoryPlugin`
  - Env fallback: `GENJA_INVENTORY_PLUGIN`
- `options` (object)
  - `hosts_file` (string | null)
  - `groups_file` (string | null)
  - `defaults_file` (string | null)
- `transform_function` (string | null)
- `transform_function_options` (object | null)

Inventory file formats:

- Files must be JSON (`.json`) or YAML (`.yaml`, `.yml`).
- Hosts and groups files are maps keyed by name.
- Defaults is a single object using the same fields as a group, minus `groups` and `defaults`.

Hosts file example (YAML):

```yaml
web-1:
  hostname: 10.0.0.10
  port: 22
  username: ubuntu
  groups:
    - web
  data:
    role: frontend
```

Groups file example (YAML):

```yaml
web:
  username: ubuntu
  data:
    env: prod
```

Defaults file example (YAML):

```yaml
username: ubuntu
platform: linux
data:
  retries: 3
  timeout_seconds: 30
```

Inventory schema (hosts/groups):

Hosts and groups support the same fields (hosts also require `name` via the map key):

- `hostname` (string | null)
- `port` (number | null)
- `username` (string | null)
- `password` (string | null)
- `platform` (string | null)
- `groups` (list of strings | null)
- `data` (object | null)
- `connection_options` (map of string to object | null)
- `defaults` (object | null)

Defaults supports the same fields as a group, minus `groups` and `defaults`.

## SSH

- `config_file` (string | null)
  - When set, SSH config syntax is validated on load.

## Runner

- `plugin` (string)
  - Default: `threaded`
  - Env fallback: `GENJA_RUNNER_PLUGIN`
  - Common values: `threaded`, `serial`
- `options` (object)
  - Plugin-specific settings.
  - Default: `{}`
- `worker_count` (number | null)
  - Optional explicit worker count for runners that support fixed concurrency.
  - For `threaded`, this is the preferred way to control the number of worker threads.
  - Default: `null`
- `max_task_depth` (integer)
  - Maximum recursion depth for task and sub-task execution.
  - Default: `10`
- `max_connection_attempts` (integer)
  - Maximum number of connection attempts before retries stop.
  - Default: `3`

## Logging

- `enabled` (bool)
  - Default: `true`
- `level` (string)
  - Default: `info`
  - Env fallback: `GENJA_LOGGING_LEVEL`
- `log_file` (string)
  - Default: `./genja.log`
  - Env fallback: `GENJA_LOGGING_LOG_FILE`
- `to_console` (bool)
  - Default: `false`
  - Env fallback: `GENJA_LOGGING_TO_CONSOLE`
- `file_size` (integer)
  - Default: `10485760` (10 MB)
- `max_file_count` (integer)
  - Default: `10`

Genja parses the `logging` section but does not initialize global logging
automatically. Applications own logger setup because Rust logging is
process-wide. A typical application startup flow is:

1. Load settings with `Settings::from_file(...)`.
2. Read `settings.logging()`.
3. Initialize the application's logger.
4. Build and run `Genja`.

Because settings must be loaded before `settings.logging()` is available, logs
emitted during settings loading may occur before a logger is initialized. Genja
keeps normal default fallbacks silent for this reason. Invalid config file values
fail settings loading. Invalid environment variable values may fall back to
defaults and emit a warning only if a logger has already been initialized.

## Troubleshooting Settings Logging

For troubleshooting settings loading, initialize a temporary logger before
calling `Settings::from_file(...)`. Use a fixed level such as `debug` because
the configured logging level is not available until after settings load.

```rust
use genja_core::Settings;
use tracing_subscriber::EnvFilter;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("debug"))
        .init();

    let settings = Settings::from_file("settings.yaml")?;

    println!("Configured log level: {}", settings.logging().level());
    Ok(())
}
```

## References

The source of truth for defaults and deserialization behavior is:

- `genja-core/src/settings.rs`
