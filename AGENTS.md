# AGENTS

Repository-specific instructions for AI coding agents working in this workspace.

## Changelog Policy

### Scope

- This repository uses a single top-level `CHANGELOG.md`.
- Do not create per-crate changelogs unless explicitly requested.
- This repository follows Keep a Changelog style for `CHANGELOG.md`.

### Format

- Add entries under `Unreleased`.
- Use user-facing headings such as:
  - `Added`
  - `Changed`
  - `Fixed`
  - `Removed`
- Write changelog entries as release notes, not as commit-message text.
- Mark breaking entries inline with `**Breaking:**` and include migration guidance in the same bullet.

### Content

- Include migration guidance for breaking changes.
- Add `Refs: #<issue>` inline at the end of each changelog bullet when the bullet is associated with a work item.
- Use `Fixes: #<issue>` only when the change fully resolves the issue and should close it.

### Timing

- Update `CHANGELOG.md` during the branch, not only at merge time.
- Any user-visible or breaking change should update the changelog.

## Testing

### Python Rust-Backed Tests

- For `genja-core-python`, run Rust-backed tests from the `genja-core-python` directory.
- Use:
  - `pdm run test-rust`
- Use `pdm run test-rust` instead of invoking `cargo test -p genja-core-python` directly. The PDM wrapper runs Cargo through the project Python environment, sets the correct `PYO3_PYTHON` interpreter, and ensures PyO3-backed tests can access the installed Python packages.
- Avoid launching multiple `genja-core-python` Rust-backed test commands concurrently from separate tool calls. Each run is already single-threaded internally, and concurrent invocations may hang or fail to return output reliably in this workspace.
- Prefer repo-documented test commands over ad hoc commands.

### Trybuild UI Tests

- `genja-core/tests/ui_genja_task/**/*.stderr` are snapshot fixtures for trybuild compile-fail tests.
- These snapshots are sensitive to Rust compiler diagnostic formatting. Drift is usually caused by `rustc` version changes, not by the feature change itself.
- Do not edit trybuild `.stderr` files manually unless needed for a small targeted correction. Prefer refreshing them by rerunning the relevant test with `TRYBUILD=overwrite`.
- Refresh trybuild snapshots only when:
  - macro diagnostics intentionally changed, or
  - the Rust toolchain changed and the emitted diagnostics legitimately drifted.
- If trybuild fixtures changed, mention in the commit or PR that the snapshot refresh was due to diagnostic output drift or intentional diagnostic changes.
- When adding new trybuild compile-fail fixtures, minimize unrelated warnings in the test case so snapshots are less brittle across toolchain updates.

## Commit Conventions

### Format

- Use Conventional Commits.
- Mark breaking changes with `!`.

### Breaking Changes

- For breaking commits, include a `BREAKING CHANGE:` footer in the commit body when appropriate.
- Breaking API, serialization, or result-shape changes must also be documented in `CHANGELOG.md`.

## Documentation

### User-Facing Changes

- Update relevant docs when public behavior, APIs, settings, or result shapes change.
- Do not leave user-visible behavior changes undocumented.

### Code Documentation

- Document newly added or changed public structs, enums, traits, functions, methods, modules, and trait methods.
- Update module-level documentation when a change alters the module's public concepts, vocabulary, behavior, or examples.
- Keep rustdoc and doc examples aligned with the current public API during the same branch as the code change.
- For Python public API changes, update affected module docstrings, class/function docstrings, type annotations, exported symbols, and stub/typecheck fixtures when applicable.
