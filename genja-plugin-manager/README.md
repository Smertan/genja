# Genja Plugin Manager

![Crates.io Version](https://img.shields.io/crates/v/genja-plugin-manager)
![GitHub License](https://img.shields.io/github/license/smertan/genja-plugin-manager)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/smertan/genja-plugin-manager/ci.yml)

A plugin management library for Rust applications that need to load Genja-compatible plugins from shared libraries at runtime.

## What Changed

The recommended integration flow is now:

1. build your plugin crates as `cdylib`
2. declare the built plugin library paths in the end-user app's `Cargo.toml`
3. call `genja_plugin_manager::build_support::copy_plugins_from_manifest()` from the end-user app's `build.rs`
4. load plugins at runtime from a `plugins` directory beside the executable

Runtime loading no longer needs to read the end-user app manifest directly.

## Features

- Dynamic loading of plugins from shared library files
  - Linux: `.so`
  - macOS: `.dylib`
  - Windows: `.dll`
- Support for individual and grouped plugin metadata entries
- Runtime scanning of a plugin directory
- Type-safe plugin lookup by plugin kind
- Build-script helper for copying plugin artifacts into the runtime plugin directory

## Installation

Runtime dependency:

```toml
[dependencies]
genja-plugin-manager = "0.1.0"
```

If your application uses manifest-driven plugin copying in `build.rs`, add it as a build dependency too:

```toml
[build-dependencies]
genja-plugin-manager = "0.1.0"
```

## Creating a Plugin

Implement `Plugin` plus one of the typed plugin traits and export `create_plugins`.

```rust
use genja_core::inventory::Hosts;
use genja_core::settings::RunnerConfig;
use genja_core::task::{TaskDefinition, TaskResults, Tasks};
use genja_plugin_manager::plugin_types::{Plugin, PluginRunner, Plugins};

#[derive(Debug)]
struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> String {
        "my_plugin".to_string()
    }
}

impl PluginRunner for MyPlugin {
    fn run(
        &self,
        _task: &TaskDefinition,
        _hosts: &Hosts,
        _runner_config: &RunnerConfig,
        _max_depth: usize,
    ) -> Result<TaskResults, genja_core::GenjaError> {
        Ok(TaskResults::new("my_plugin"))
    }

    fn run_tasks(
        &self,
        _tasks: &Tasks,
        _hosts: &Hosts,
        _runner_config: &RunnerConfig,
        _max_depth: usize,
    ) -> Result<Vec<TaskResults>, genja_core::GenjaError> {
        Ok(Vec::new())
    }
}

#[unsafe(no_mangle)]
pub fn create_plugins() -> Vec<Plugins> {
    vec![Plugins::Runner(Box::new(MyPlugin))]
}
```

Plugin crate setup:

```toml
[package]
name = "my_plugin"
version = "0.1.0"
edition = "2024"

[dependencies]
genja-plugin-manager = "0.1.0"
genja-core = "0.1.0"

[lib]
name = "my_plugin"
crate-type = ["lib", "cdylib"]
```

Build the plugin:

```bash
cargo build --release
```

## End-User Application Setup

The end-user application is the source of truth for plugin artifacts.

Example `Cargo.toml`:

```toml
[package]
name = "use_genja"
version = "0.1.0"
edition = "2024"

[dependencies]
genja = "0.1.0"
genja-plugin-manager = "0.1.0"

[build-dependencies]
genja-plugin-manager = "0.1.0"

[package.metadata.plugins]
hostname_ip_transform = "../target/{PROFILE}/libhostname_ip_transform.so"

[package.metadata.plugins.inventory]
host_loader = "../target/{PROFILE}/libhost_loader.so"
```

Notes:

- metadata paths are resolved relative to the consuming app's `Cargo.toml`
- `{PROFILE}` is replaced by `debug` or `release` by the build helper
- grouped entries are supported and flattened into the runtime plugin directory

Example `build.rs`:

```rust
fn main() {
    genja_plugin_manager::build_support::copy_plugins_from_manifest().unwrap();
}
```

What this does:

- reads `[package.metadata.plugins]` from the end-user app manifest
- copies the referenced plugin libraries into `target/{PROFILE}/plugins`
- leaves runtime loading to the normal plugin directory scan

## Runtime Loading

At runtime, load plugins from a directory instead of reading `Cargo.toml`.

```rust
use genja_plugin_manager::PluginManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let plugin_manager = PluginManager::new()
        .load_plugins_from_directory("target/debug/plugins")?;

    if let Some(runner) = plugin_manager.get_runner_plugin("my_plugin") {
        let _ = runner;
    }

    Ok(())
}
```

In a real application, prefer resolving the plugin directory relative to the executable location, for example `current_exe().parent().join("plugins")`.

## Metadata Format

Both individual and grouped entries are supported.

```toml
[package.metadata.plugins]
plugin_a = "../target/{PROFILE}/libplugin_a.so"

[package.metadata.plugins.inventory]
inventory_a = "../target/{PROFILE}/libinventory_a.so"

[package.metadata.plugins.runner]
threaded_ext = "../target/{PROFILE}/libthreaded_ext.so"
```

## Workspace Notes

If the end-user app is part of a Cargo workspace, paths in `[package.metadata.plugins]` are still resolved relative to that crate's own `Cargo.toml`, not the workspace root.

That usually means plugin artifact paths look like:

```toml
[package.metadata.plugins]
plugin_a = "../target/{PROFILE}/libplugin_a.so"
```

instead of:

```toml
[package.metadata.plugins]
plugin_a = "target/{PROFILE}/libplugin_a.so"
```

depending on your workspace layout.

## API Summary

Common entry points:

- `PluginManager::load_plugin(...)`
- `PluginManager::load_plugins_from_directory(...)`
- `PluginManager::get_runner_plugin(...)`
- `PluginManager::get_inventory_plugin(...)`
- `PluginManager::get_processor_plugin(...)`
- `genja_plugin_manager::build_support::copy_plugins_from_manifest()`

## License

This project is licensed under the Apache License, Version 2.0. See `LICENSE`.

## Contributing

Contributions are welcome. Submit a pull request with tests for behavior changes.
