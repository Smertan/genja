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

### Workspace Rust Tests

- When running the workspace Rust test suite, exclude `genja-core-python` because it requires the Python-backed test environment.
- Use:
  - `cargo test --workspace --exclude genja-core-python`

### Lint And Type Checks

- Run Rust formatting and clippy checks from the workspace root.
- Use:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
- Full workspace clippy may include `genja-core-python`; it does not need the `genja-core-python` test exclusion.
- For Python linting and type checks, run from the `genja-core-python` directory.
- Use:
  - `pdm run lint`
  - `pdm run typecheck`
  - `pdm run check-stubs`
- Run `pdm run check-stubs` when changing Rust/PyO3-exposed Python APIs, `.pyi` stubs, Python API docstrings, or top-level Python re-exports.

### Python Rust-Backed Tests

- For `genja-core-python`, run Python and Rust-backed tests from the `genja-core-python` directory.
- Ask the user to run:
  - `pdm run test`
  - `pdm run test-rust`
- Do not run `pdm run test` or `pdm run test-rust` from the agent harness; they can fail or hang there.
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

## Release Versioning

- Rust releases use a unified release train for publishable Rust crates.
- When preparing `rs-vX.Y.Z`, bump all publishable Rust crates to `X.Y.Z` and keep their internal path dependency version requirements aligned:
  - `genja`
  - `genja-core`
  - `genja-core-derive`
  - `genja-plugin-manager`
- Do not leave an unchanged publishable Rust crate on the previous version during a Rust release.
- Python package releases use the matching `py-vX.Y.Z` tag and bump `genja-py` / `genja-core-python` to `X.Y.Z`.

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

### Python Typing And Stubs

- For Rust/PyO3-exposed Python APIs, keep the corresponding `.pyi` stubs aligned with the exported runtime API.
- Public user-facing classes, functions, methods, and properties in `.pyi` stubs should include useful docstrings.
- For pure Python modules, prefer inline type annotations and docstrings in the `.py` source file instead of creating a new `.pyi` file.
- Do not create new `.pyi` files for pure Python modules unless there is a specific reason to separate implementation from typing.
- Keep `genja/__init__.pyi` aligned with public re-exports from `genja/__init__.py`.
- `pdm run check-stubs` is intentionally scoped to API surfaces already brought up to the documentation standard. When improving another stub file, add it to `STUBS_REQUIRING_DOCSTRINGS` in `genja-core-python/scripts/check_python_api_docs.py` during the same change.
- For Rust/PyO3 classes re-exported from `genja/__init__.py`, keep the class shape aligned between `genja.pyi` and `__init__.pyi`. When another duplicated top-level class is brought under the documentation standard, add it to `DUPLICATED_TOP_LEVEL_CLASSES` in `check_python_api_docs.py`.
- When adding Rust/PyO3 doc comments for another binding source file, extend `genja-core-python/scripts/check_python_api_docs.py` so `pdm run check-stubs` prevents regressions for that file or scoped impl block.
