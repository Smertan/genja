# Changelog

All notable changes to this workspace should be documented in this file.

## Unreleased

### Changed

- **Breaking:** Redesigned `HostTaskResult` from an enum into a structured object with `outcome` and `execution_metadata`. Rust consumers should migrate direct enum variant matching to compatibility accessors or the new structured fields.
- Python `HostTaskResult.to_dict()` now returns the new structured shape.
- `HostTaskResult.status` remains available as a convenience accessor.
- Added runner-level retry defaults through `allow_retries` and `max_task_attempts` in shared settings, plus task-level overrides in Rust and Python task metadata.

Refs: #63
