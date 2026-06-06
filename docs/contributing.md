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

The workspace includes Rust 2024 edition crates, so contributors should use
Rust and Cargo 1.85.0 or newer.

This is the contributor baseline for the current workspace. Individual crates do
not currently declare a formal `rust-version` MSRV in their package metadata.

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

If a change touches shared runtime behavior, task execution, plugin loading, or
public APIs, prefer running both the focused package tests and the Rust
workspace test command above.

## Python Checks

The Python bindings live in `genja-core-python`. Run commands from that
directory.

Build the extension into the active Python environment and run the Python tests:

```bash
pdm run test
```

Run the Rust tests that exercise the PyO3 crate:

```bash
pdm run test-rust
```

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

## Release Notes

User-facing compatibility information belongs in
[Versions And Compatibility](version-compatibility.md). Maintainer-only publish
steps should stay out of the user guides unless they are needed by contributors
building release artifacts.
