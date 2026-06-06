# License

Genja is licensed under `AGPL-3.0-only`.

The repository root `LICENSE` file contains the GNU Affero General Public
License version 3. The Rust crates and Python package metadata are expected to
use the same license identifier.

## Packages

The following Rust crates declare `AGPL-3.0-only` in their Cargo package
metadata:

- `genja`
- `genja-core`
- `genja-core-derive`
- `genja-plugin-manager`

The Python package `genja-py` also declares `AGPL-3.0-only`.

## What AGPL Means

AGPL-3.0 is a strong copyleft license. In broad terms, if you modify and deploy
AGPL-covered software for users over a network, the license is designed to give
those users access to the corresponding source code for the modified version.

This page is only a project guide, not legal advice. Review the license text and
your own distribution or hosting model before adopting Genja in a commercial,
hosted, or closed-source product.

## Plugins

Plugins are separate packages loaded by Genja at runtime. Their license should
be documented in the plugin package metadata and repository.

If a plugin links directly against Genja crates, its license obligations may be
affected by the AGPL-licensed crate it depends on. Plugin authors should review
the AGPL terms before distributing plugins or using them in hosted systems.
