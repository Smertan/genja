# Genja

Genja is a plugin-based automation framework for executing tasks across
multiple hosts.

Inspired by tools like Nornir, Ansible, and Salt, Genja brings the benefits of
static typing and true multi-threading to network automation. It provides both
Rust and Python APIs, allowing you to choose the right tool for your use
case—whether you need the performance of native Rust or the flexibility of Python.

![Genja - Network automation, engineered differently](assets/images/genja-overview.png)

## Key Features

- **Performance**: True multi-threading and async execution in both Rust and Python
- **Type Safety**: Compile-time checks and validation through Rust's type system
- **Plugin Architecture**: Extensible design for inventory, transforms, runners, and tasks
- **Dual Language Support**: Native Rust API and Python bindings
- **Network-Focused**: Built with networking automation requirements in mind—reliability, speed, metrics, and secure remote execution

Use the installation guide to add Genja to a Rust or Python project, then use
the quickstart to load inventory and run your first task.

## Getting Started

- [Installation](installation.md)
- [Quickstart](quickstart.md)
- [Settings](settings.md)
- [Concepts](concepts.md)
- [Inventory](inventory.md)
- [Tasks](tasks.md)

## Reference

- [Plugins](plugins/index.md)
- [Transforms](transforms.md)
- [Connections](connection.md)
- [Processors](processors.md)
- [Runners](runners.md)
- [Examples](examples.md)
- [API Surface](api-surface.md)
- [Versions And Compatibility](version-compatibility.md)
- [Logging And Troubleshooting](logging-troubleshooting.md)

## Development

- [Contributing](contributing.md)
- [License](license.md)

## Licensing

Genja is licensed under `AGPL-3.0-only`. See [License](license.md) for package
license details.
