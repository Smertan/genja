# License

Genja is licensed under `MPL-2.0`.

The repository root `LICENSE` file contains the Mozilla Public License version
2.0. Each publishable Rust crate also includes a local `LICENSE` copy so crate
archives carry the license text directly.

## Packages

The following Rust crates declare `MPL-2.0` in their Cargo package metadata:

- `genja`
- `genja-core`
- `genja-core-derive`
- `genja-core-python`
- `genja-plugin-manager`

The Python package `genja-py` also declares `MPL-2.0`.

## What MPL Means

MPL-2.0 is a weak copyleft license that applies at the source-file level. In
broad terms, modifications to MPL-covered files should remain available under
MPL-2.0 when distributed, while separate files in a larger work can use a
different license.

This page is only a project guide, not legal advice. Review the license text and
your own distribution or hosting model before adopting Genja in a commercial,
hosted, or closed-source product.

## Plugins

Plugins are separate packages loaded by Genja at runtime. Their license should
be documented in the plugin package metadata and repository.

Plugin authors should review MPL-2.0 and their own distribution model when
building plugins or larger systems that combine with Genja.
