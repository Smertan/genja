# Contributing

This page is for contributors working on Genja itself. If you only need to add
Genja to an application, start with [Installation](installation.md) and
[Quickstart](quickstart.md).

## Development Setup

Genja is a Rust workspace with Python bindings for the core runtime. A full
development environment needs:

- Rust and Cargo
- Python 3.10 or newer
- PDM for the Python binding development environment
- maturin for building the Python extension module
- Zensical for building the documentation site

## Tool Versions

The workspace pins Rust and Cargo to 1.85.0 in `rust-toolchain.toml`. Use that
toolchain locally so compiler diagnostics, formatting, and trybuild snapshots
match CI.

Individual crates do not currently declare a formal `rust-version` MSRV in
their package metadata.

Python binding development requires Python 3.10 or newer, as declared by the
`genja-py` package.

Clone the repository and check the Rust workspace first:

```bash
cargo check --workspace
```

For Python binding work, install the Python development dependencies from the
binding package:

```bash
cd genja-core-python
pdm install
```

## Rust Checks

Run the Rust workspace tests before sending changes that affect Rust behavior.
Exclude `genja-core-python` from the workspace test run; its PyO3 tests depend
on the Python environment and should be run with the PDM commands below.

```bash
cargo test --workspace --exclude genja-core-python
```

For focused changes, run the package that owns the behavior:

```bash
cargo test -p genja-core
cargo test -p genja
cargo test -p genja-core-derive
cargo test -p genja-plugin-manager
```

Use Cargo formatting before submitting Rust changes:

```bash
cargo fmt --all -- --check
```

This CI command is read-only. It fails when formatting changes are needed; fix
those locally with:

```bash
cargo fmt --all
```

Run Clippy with warnings treated as errors:

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

CI does not auto-fix formatting or Clippy warnings. If you use
`cargo clippy --fix`, review the generated changes and commit the ones you want
to keep.

If a change touches shared runtime behavior, task execution, plugin loading, or
public APIs, prefer running both the focused package tests and the Rust
workspace test command above.

### Trybuild UI Fixtures

The derive macro compile-fail tests under `genja-core/tests/ui_genja_task/`
use trybuild `.stderr` snapshot files.

These snapshots are sensitive to Rust compiler diagnostic formatting. In
practice, drift is usually caused by `rustc` version changes rather than by the
feature change itself.

When a trybuild test fails only because the expected and actual diagnostics have
drifted, refresh the fixtures with:

```bash
TRYBUILD=overwrite cargo test -p genja-core --test derive_compile
```

Do not hand-edit `.stderr` fixtures unless you are making a small targeted
correction. Prefer regenerating them from compiler output.

Refresh trybuild snapshots only when:

- macro diagnostics intentionally changed
- the Rust toolchain changed and the emitted diagnostics legitimately drifted

When adding new compile-fail fixtures, minimize unrelated warnings in the test
case where possible so snapshot files are less brittle across toolchain
updates.

## Python Checks

The Python bindings live in `genja-core-python`. Run commands from that
directory.

Install the Python development dependencies before running checks:

```bash
pdm install -d
```

Build the extension into the active Python environment and run the Python tests:

```bash
pdm run test
```

Run the Rust tests that exercise the PyO3 crate:

```bash
pdm run test-rust
```

Use `pdm run test-rust` instead of invoking `cargo test -p genja-core-python`
directly. The wrapper runs Cargo through the project Python environment, sets
the correct `PYO3_PYTHON` interpreter, and ensures the PyO3-backed tests can
access the installed Python packages.

Avoid launching multiple `genja-core-python` Rust-backed test commands
concurrently from separate processes. Each run is already single-threaded
internally, and concurrent invocations may hang or fail to return output
reliably in this workspace.

Run type checks for the Python examples and tests:

```bash
pdm run typecheck
```

Run Python lint and formatting checks:

```bash
pdm run lint
```

When debugging PyO3 test failures, the project also provides:

```bash
pdm run test-rust-debug
```

## Changelog And Compatibility

This repository uses a single top-level `CHANGELOG.md`.
This repository follows Keep a Changelog style for `CHANGELOG.md`.

- Add user-visible changes under `Unreleased`.
- Use release-note headings such as:
  - `Added`
  - `Changed`
  - `Fixed`
  - `Removed`
- Mark breaking entries inline with `**Breaking:**` and include migration guidance in the same bullet.
- Update the changelog during the branch, not only at merge time.
- For linked work items, use `Refs: #<issue>` when an issue exists.

## Documentation

Documentation pages live in `docs/` and the navigation is configured in
`zensical.toml`.

After changing documentation, build the site:

```bash
zensical build
```

To preview the site locally:

```bash
zensical serve
```

When adding a new guide, update both the navigation in `zensical.toml` and the
link list in [Home](index.md) when the page should be discoverable from the
front page.

## Examples

Rust examples live under `genja/examples`. Run them with Cargo:

```bash
cargo run -p genja --example basic_runtime
cargo run -p genja --example filter_hosts
cargo run -p genja --example run_task
cargo run -p genja --example run_task_tree
cargo run -p genja --example async_inventory_plugin
```

Python examples live under `genja/examples/python`. Build the local Python
extension before running them:

```bash
cd genja-core-python
maturin develop
cd ..
python genja/examples/python/basic_runtime.py
```

If you add or change an example, update [Examples](examples.md) so users know
what the example demonstrates.

## Plugin Changes

Plugin manager changes often need dynamic loading coverage. The workspace
includes test plugin crates under `genja-plugin-manager/tests/`.

For plugin loading, registration, ABI, or manager behavior, run:

```bash
cargo test -p genja-plugin-manager
```

For runtime behavior that crosses from plugins into task execution, also run the
relevant `genja-core` or `genja` tests.

## Contribution Flow

Keep changes focused. A good contribution usually includes:

- The implementation change
- Tests for changed behavior
- Documentation updates for user-facing behavior
- Example updates when the change affects common workflows

Before opening a pull request, run the smallest focused checks that prove the
change and the broader checks for any shared runtime or public API changes.

GitHub Actions runs CI for pull requests and for pushes to `main`, `develop`,
`feature/**`, and issue-style `*-genja-*` branches. CI runs formatting checks,
Clippy with `-D warnings`, Rust tests excluding `genja-core-python`, and the
Python binding lint, typecheck, Python test, and PyO3 Rust test commands. A
separate cross-platform compatibility workflow runs on pull requests into
`main` and checks Linux, macOS, and Windows across all supported Python versions.

Release publishing is separate from CI. Rust crates are published only from
`rs-vX.Y.Z` tags on commits reachable from `main`, and the release workflow
publishes the crates in dependency order after validating crate versions and
internal dependency metadata. Python package releases use matching `py-vX.Y.Z`
tags, validate `genja-core-python/pyproject.toml`, build wheels plus a source
distribution, install and test each built wheel, and publish to PyPI with
trusted publishing.

## Release Flow

Use `develop` as the integration branch and `main` as the release branch. Release
preparation should happen on a release branch cut from `develop`:

```text
feature/* -> PR -> develop
develop -> release/x.y.z
release/x.y.z -> PR -> main
main -> fast-forward develop
```

While the release pull request is open, avoid merging new work into `develop` so
`develop` can be fast-forwarded to the released `main` commit after publication.
If `develop` has moved ahead, do not force it; either finish the release sync
first or use a normal merge from `main` into `develop`.

After the release pull request is merged, tag the exact resulting `main` commit.
Rust and Python release tags may point at the same commit:

```bash
git checkout main
git pull origin main
git tag rs-vX.Y.Z
git tag py-vX.Y.Z
git push origin rs-vX.Y.Z py-vX.Y.Z
```

Fast-forward `develop` to the released commit before resuming feature merges:

```bash
git checkout develop
git pull origin develop
git merge --ff-only main
git push origin develop
```

## Release Notes

User-facing compatibility information belongs in
[Versions And Compatibility](version-compatibility.md). Maintainer-only publish
steps should stay out of the user guides unless they are needed by contributors
building release artifacts.
