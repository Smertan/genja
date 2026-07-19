# Changelog

All notable changes to this workspace should be documented in this file.

## Unreleased

### Added

- Added Python `Genja.filter_hosts(...)` for predicate-based host filtering with Python callables. Refs: #85

### Changed

- Improved Python type stubs and editor-facing API documentation for Genja runtime, settings, plugin manager, connection, plugin, and processor APIs. Refs: #94

## 0.3.0 - 2026-07-14

Released packages:

- Rust crates: `genja`, `genja-core`, `genja-plugin-manager`
- Python package: `genja-py`

### Added

- Added Rust `Genja::from_settings(...)` and Python `Genja.from_settings(...)` constructors that validate programmatic settings and load inventory from `settings.inventory`. Refs: #81
- Exposed the current 1-based task attempt through Rust and Python task runtime contexts. Refs: #73
- Forwarded Rust-side runtime logs from the Python extension into Python's standard `logging` system, allowing applications and pytest `caplog` to capture Genja logs. Refs: #74
- Added Python constructors for `Settings` and nested settings config classes so applications can build runtime settings without a settings file. Refs: #68
- Added explicit settings validation with Rust `Settings::validate()` and Python `Settings.validate()` / `SSHConfig.validate()`. Refs: #68

### Changed

- **Breaking:** Settings files now reject unknown top-level sections and unknown fields inside typed settings sections instead of silently ignoring them. Remove unused keys, correct misspelled keys, or move plugin-specific values into explicit option maps such as `runner.options` or `inventory.transform_function_options`. Refs: #76
- Python runtime construction now validates supplied programmatic settings before building a runtime. Refs: #68

### Fixed

- Fixed the Python source distribution so the declared `LICENSE` file is included at the package root. Refs: #72

## 0.2.0 - 2026-07-06

Released packages:

- Rust crates: `genja`, `genja-core`, `genja-core-derive`, `genja-plugin-manager`
- Python package: `genja-py`

### Changed

- Relicensed the workspace and published packages under MPL-2.0.
- **Breaking:** Redesigned `HostTaskResult` from an enum into a structured object with `outcome` and `execution_metadata`. Rust consumers should migrate direct enum variant matching to compatibility accessors or the new structured fields. Refs: #63
- **Breaking:** Removed duplicated host timing fields from `outcome.Passed` and `outcome.Failed` in human JSON serialization. Consumers should read host timing from `execution_metadata.started_at`, `execution_metadata.finished_at`, and `execution_metadata.duration` instead. Refs: #63
- **Breaking:** Removed execution timing fields and accessors from `TaskSuccess` and `TaskFailure`. Rust consumers should read per-host timing from `HostTaskResult.execution_metadata()` and aggregate task timing from `TaskResults`. Refs: #63
- Python `HostTaskResult.to_dict()` now returns the new structured shape. Refs: #63
- `HostTaskResult.status` remains available as a convenience accessor. Refs: #63
- Added grouped retry metadata with `RetryConfig` and the public fields `retry.allow`, `retry.max_attempts`, and `retry.delay_ms` for task-level retry overrides. Refs: #67
- Added nested runner retry settings under `runner.retry.allow`, `runner.retry.max_attempts`, and `runner.retry.delay_ms`. Refs: #67
- Added Rust task macro retry metadata with `#[genja_task(retry(allow = ..., max_attempts = ..., delay_ms = ...))]`. Refs: #67
- Added Python task retry metadata with `retry=RetryConfig(allow=..., max_attempts=..., delay_ms=...)`. Refs: #67
- Task retry execution now applies `retry.delay_ms` as a fixed in-process delay before retry attempts only. Refs: #67
- Added runner-level retry defaults in shared settings, plus task-level overrides in Rust and Python task metadata. Refs: #63
- Built-in task execution now applies retry policy from runner settings and task metadata. Retries only occur for failures explicitly marked `retryable`, and host execution metadata now records attempts, whether a retry occurred, and whether retries were exhausted. Refs: #63
- The workspace Rust toolchain is now pinned to `1.88.0` to align local diagnostics and trybuild snapshots with CI. Refs: #63
