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
- Use `Refs: #<issue>` for linked work items when an issue exists.

### Timing

- Update `CHANGELOG.md` during the branch, not only at merge time.
- Any user-visible or breaking change should update the changelog.

## Testing

### Python Rust-Backed Tests

- For `genja-core-python`, run Rust-backed tests from the `genja-core-python` directory.
- Use:
  - `pdm run test-rust`
- Prefer repo-documented test commands over ad hoc commands.

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
