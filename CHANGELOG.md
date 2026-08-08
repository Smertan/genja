# Changelog

All notable changes to this workspace should be documented in this file.

## Unreleased

Changed packages:

- Rust crates: `genja`, `genja-core`, `genja-core-derive`, `genja-plugin-manager`
- Python package: `genja-py`

### Added

- Added core Rust dry-run task capability, context, trait, and execution metadata APIs. Refs: #80
- Added Rust `#[genja_task(...)]` dry-run metadata with `supports_dry_run = true`. Refs: #80
- Added Rust task idempotency mode metadata with `IdempotencyMode` and `TaskInfo::idempotency_mode()`, defaulting to disabled. Refs: #88
- Added Rust `#[genja_task(...)]` idempotency metadata with `idempotency = IdempotencyMode::...`. Refs: #88
- Added Rust idempotency check result and default task check hooks for blocking and async tasks. Refs: #88
- Added Rust `#[genja_task(...)]` validation and delegation for idempotency check hooks. Refs: #88
- Added Rust runtime pre-check execution for idempotent tasks, preserving dry-run dispatch without automatic idempotency checks. Refs: #88
- Added Rust `CheckAndVerify` post-application convergence verification with validation failures for remaining changes. Refs: #88
- Added Rust idempotency retry convergence results as `PassedWithWarnings` when a later pre-check finds convergence after a retryable failure. Refs: #88
- Added Python task idempotency support with `IdempotencyMode`, `IdempotencyCheckResult`, `@task(..., idempotency=...)`, and blocking or async check hooks. Refs: #88
- Added Rust runtime task execution options with dry-run support through `Genja::run_task_with_options(...)`, `Genja::run_task_with_options_async(...)`, `Genja::run_tasks_with_options(...)`, and `Genja::run_tasks_with_options_async(...)`. Refs: #80
- Added Python dry-run task support with `@genja.task(..., supports_dry_run=True)`, `dry_run` / `dry_run_async` task methods, `TaskRunOptions` via `run_options=...`, and `TaskRuntimeContext.dry_run`. Refs: #80
- Added Python `Genja.filter_hosts(...)` for predicate-based host filtering with Python callables. Refs: #85
- Added Rust `Genja::from_settings_async(...)` plus Python `Genja.from_settings_async(...)` and `Genja.from_settings_file_async(...)` for strict async inventory loading. Refs: #86
- Added Rust `#[genja_task(...)]` session verification metadata with `session_verification(max_attempts = ..., delay_ms = ...)`. Refs: #89
- Added core Rust session verification configuration and host execution metadata APIs for post-change new-session verification. Refs: #89
- Added async-safe Rust connection replacement support in `ConnectionManager` for post-change session verification. Refs: #89
- Added a Rust task connection resolver replacement hook and wired the built-in runtime resolver to recreate inventory-backed connections for post-change session verification. Refs: #89
- Added Rust runtime execution for post-change session verification after passed, changed task results, including bounded replacement attempts and host-scoped connection failures. Refs: #89
- Added Rust `CheckAndVerify` integration so post-change idempotency verification runs through the replacement session when session verification is enabled. Refs: #89
- Added Rust serial and threaded runner coverage for post-change session verification, including host-scoped failure continuation. Refs: #89
- Added Python `SessionVerificationConfig` and `@task(..., session_verification=...)` support for post-change new-session verification. Refs: #89

### Changed

- **Breaking:** Added `PassedWithWarnings` as a new host task outcome for successful results that carry important warnings. Update exhaustive Rust matches on `HostTaskOutcome` and serialized result parsers to handle `PassedWithWarnings` alongside `Passed`, `Failed`, and `Skipped`. Refs: #99
- **Breaking:** Rust runner plugins now receive `TaskRunOptions` instead of a bare `max_depth` in `PluginRunner::run_task(...)` and `PluginRunner::run_tasks(...)`. Update runner plugin implementations to accept `run_options: TaskRunOptions` and read recursion depth with `run_options.max_depth()`. Refs: #80
- **Breaking:** Python runner plugins now receive `run_options: TaskRunOptions` instead of a bare `max_depth` in `run_task(...)` and `run_tasks(...)`. Update runner plugin implementations to accept `run_options` and pass it to `TaskDefinition.run_on_host(...)` / `run_on_hosts(...)`, or read recursion depth with `run_options.max_depth`. Refs: #80
- **Breaking:** Python task results now support `TaskStatus.PASSED_WITH_WARNINGS` and serialize warning-bearing successes as `PassedWithWarnings`. Update result parsers that assume only `Passed`, `Failed`, and `Skipped` outcome keys. Refs: #99
- Improved Python type stubs and editor-facing API documentation for Genja runtime, settings, plugin manager, connection, plugin, and processor APIs. Refs: #94
- **Breaking:** Rust `Genja::from_settings_file_async(...)` now requires the selected inventory plugin to implement `AsyncPluginInventory` and no longer falls back to synchronous inventory plugins. Python `Genja.from_settings(...)` now rejects async Python inventory plugins; use `await Genja.from_settings_async(...)` instead. Use sync constructors for sync inventory plugins such as `FileInventoryPlugin`, and async constructors for async inventory plugins. Refs: #86

### Fixed

- Improved Python dry-run task decorator validation errors so missing dry-run methods identify the task execution mode and required method signature. Refs: #80
- Settings files now allow partial `inventory` sections, with omitted inventory fields falling back to their defaults. Refs: #86

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
