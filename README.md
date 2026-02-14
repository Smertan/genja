# Genja

## Configuration Precedence And Error Handling

Configuration is loaded from two sources in this order:

1. Config file values (JSON/YAML).
2. Environment variables (used only by default functions, not to override explicit config).

Behavior rules:

- If a config field is explicitly provided and is invalid, deserialization fails with an error.
- If a config field is missing, a default value is used.
- For defaults that read environment variables, invalid env values trigger a warning and the default fallback is used.
- Environment variables do not override explicitly provided config values.

Current environment variables:

- `GENJA_CORE_RAISE_ON_ERROR` (bool, loose parsing: `true/false`, `1/0`, `yes/no`, `on/off`)
- `GENJA_INVENTORY_PLUGIN` (string)
- `GENJA_RUNNER_PLUGIN` (string)
- `GENJA_LOGGING_LEVEL` (string)
- `GENJA_LOGGING_LOG_FILE` (string path)
- `GENJA_LOGGING_TO_CONSOLE` (bool, loose parsing: `true/false`, `1/0`, `yes/no`, `on/off`)
