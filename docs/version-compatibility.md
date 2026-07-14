# Versions And Compatibility

Genja has Rust crates and Python bindings. This page describes the current
package layout, version expectations, and how to read docs against releases.

## Packages

| Package | Ecosystem | Current role |
| --- | --- | --- |
| `genja` | Rust crate | Main Rust runtime crate for applications. |
| `genja-core` | Rust crate | Core inventory, settings, task, result, state, and error types. |
| `genja-core-derive` | Rust crate | Procedural macros such as `#[genja_task]`. |
| `genja-plugin-manager` | Rust crate | Plugin traits, registration, dynamic loading, and build helpers. |
| `genja-py` | Python distribution | Python bindings that expose the `genja` import package. |
| `genja-core-python` | Rust crate | Internal PyO3 extension crate that builds the Python package. |

See [API Surface](api-surface.md) for which package to depend on.

## Current Versioning

The Rust runtime crates in this release currently use:

```text
genja = 0.3.0
genja-core = 0.3.0
genja-core-derive = 0.2.0
genja-plugin-manager = 0.3.0
```

The Python distribution is also currently versioned as:

```text
genja-py = 0.3.0
```

Until a more formal compatibility policy is published, treat matching minor
versions of `genja`, `genja-core`, `genja-plugin-manager`, and `genja-py` as
the supported combination. For example, use `genja-py 0.1.x` with the Rust
runtime crates from the `0.1.x` line.

## Rust Compatibility

Rust applications should usually depend on `genja`:

```toml
[dependencies]
genja = "0.3.0"
```

The `genja` crate depends on and re-exports core pieces from the workspace, so
most application code does not need direct dependencies on `genja-core` or
`genja-core-derive`.

Use direct lower-level dependencies when building advanced integrations or
standalone plugin crates:

```toml
[dependencies]
genja-plugin-manager = "0.3.0"
genja-core = "0.3.0"
```

When depending on multiple Genja crates directly, keep their versions aligned.
Mixing `genja 0.1.x` with a different `genja-core` or `genja-plugin-manager`
line is not a supported configuration.

## Python Compatibility

Install the Python distribution with:

```bash
pip install genja-py
```

Import it as:

```python
import genja
```

The Python package is built from the Rust runtime in this repository. Keep the
Python package version aligned with the Rust crate version line when combining
Python and Rust artifacts in the same project.

## Docs And Releases

The docs in the repository default branch describe the current development
state. For release-specific behavior, use the docs or source for the matching
release tag, such as:

```bash
git checkout rs-v0.3.0
```

When running repository examples for a specific release, check out the matching
tag before running commands from [Examples](examples.md).

## Compatibility Notes

- Task result JSON has normalized and raw forms. Consumers should prefer
  normalized output unless they explicitly need raw enum-shaped data.
- Plugin names and groups are runtime contracts. Renaming a plugin or changing
  its group is a compatibility-impacting change.
- Task macro behavior is part of the Rust task authoring surface. Changes to
  `#[genja_task]` should be treated as API changes.
- Python task and plugin base classes are public authoring APIs. Changes to
  method names or result shapes affect Python users.

## Related Guides

- [Installation](installation.md)
- [API Surface](api-surface.md)
- [Examples](examples.md)
