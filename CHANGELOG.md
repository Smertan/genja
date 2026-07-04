# Changelog

All notable changes to this workspace should be documented in this file.

## Unreleased

### Changed

- **Breaking:** Redesigned `HostTaskResult` from an enum into a structured object with `outcome` and `execution_metadata`. Rust consumers should migrate direct enum variant matching to compatibility accessors or the new structured fields. Refs: #63
- **Breaking:** Removed duplicated host timing fields from `outcome.Passed` and `outcome.Failed` in human JSON serialization. Consumers should read host timing from `execution_metadata.started_at`, `execution_metadata.finished_at`, and `execution_metadata.duration` instead. Refs: #63
- **Breaking:** Removed execution timing fields and accessors from `TaskSuccess` and `TaskFailure`. Rust consumers should read per-host timing from `HostTaskResult.execution_metadata()` and aggregate task timing from `TaskResults`. Refs: #63
- Python `HostTaskResult.to_dict()` now returns the new structured shape. Refs: #63
- `HostTaskResult.status` remains available as a convenience accessor. Refs: #63
- Added grouped retry metadata with `RetryConfig` and the public fields `retry.allow`, `retry.max_attempts`, and `retry.delay_ms` for task-level retry overrides. Refs: #67
- Added nested runner retry settings under `runner.retry.allow`, `runner.retry.max_attempts`, and `runner.retry.delay_ms`. Refs: #67
- Added Rust task macro retry metadata with `#[genja_task(retry(allow = ..., max_attempts = ..., delay_ms = ...))]`. Refs: #67
- Added Python task retry metadata with `retry=RetryConfig(allow=..., max_attempts=..., delay_ms=...)`. Refs: #67
- Added runner-level retry defaults in shared settings, plus task-level overrides in Rust and Python task metadata. Refs: #63
- Built-in task execution now applies retry policy from runner settings and task metadata. Retries only occur for failures explicitly marked `retryable`, and host execution metadata now records attempts, whether a retry occurred, and whether retries were exhausted. Refs: #63
- The workspace Rust toolchain is now pinned to `1.88.0` to align local diagnostics and trybuild snapshots with CI. Refs: #63
