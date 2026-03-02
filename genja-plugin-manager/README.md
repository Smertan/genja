# Genja Plugin Manager

![Crates.io Version](https://img.shields.io/crates/v/genja-plugin-manager)
![GitHub License](https://img.shields.io/github/license/smertan/genja-plugin-manager)
![GitHub Actions Workflow Status](https://img.shields.io/github/actions/workflow/status/smertan/genja-plugin-manager/ci.yml)

A flexible and easy-to-use plugin management system for Rust applications. It provides a robust foundation for building plugin-based architectures
in Rust applications.

The `Genja Plugin Manager` library allows dynamic loading, registration, and management of plugins at runtime. It supports individual plugins and grouped plugins, making it suitable for various application architectures.

## Features

- Dynamic loading of plugins from shared object files (`.so` *Linux*, `.dll` *Windows*, `.dylib` *MacOS*)
- Support for individual and grouped plugins
- Plugin registration and deregistration
- Execution of plugin functionality
- Metadata-driven plugin configuration

## Installation

The package can either be installed via `cargo add` or the `Cargo.toml` file.

cargo add

```sh
cargo add genja-plugin-manager
```

or

Cargo.toml file

```toml
[dependencies]
genja-plugin-manager = "0.1.0"
```

## Creating Plugins

To create a plugin, implement the `Plugin` trait and export a `create_plugins` function:

```rust
use plugin_manager::Plugin;
use std::any::Any;

#[derive(Debug)]
struct MyPlugin;

impl Plugin for MyPlugin {
    fn name(&self) -> String {
        "my_plugin".to_string()
    }

    fn execute(&self, _context: &dyn Any) -> Result<(), Box<dyn std::error::Error>> {
        println!("Executing MyPlugin");
        Ok(())
    }
}

#[unsafe(no_mangle)]
pub fn create_plugins() -> Vec<Box<dyn Plugin>> {
    vec![Box::new(MyPlugin)]
}
```

## Setting up Cargo.toml for Plugins

When creating a plugin, you need to set up your `Cargo.toml` file correctly:

1. Add the `genja-plugin-manager` as a dependency:

    ```toml
    [dependencies]
    genja-plugin-manager = "0.1.0"
    ```

2. Configure the library to be both a Rust library and a dynamic library:

    ```toml
    [lib]
    name = "your_plugin_name"
    crate-type = ["lib", "cdylib"]
    ```

This configuration allows your plugin to be compiled as both a Rust library
and a dynamic library, which is necessary for the PluginManager to load it at runtime.

## Building the Plugin

To build your plugin for use with the main project:

1. Navigate to your plugin's directory.
2. Run the following command to build the plugin as a dynamic library:

    ```bash
    cargo build --release
    ```

3. The compiled dynamic library will be in the `target/release` directory with a name like
   `libyour_plugin_name.so` (on Linux), `libyour_plugin_name.dylib` (on macOS),
   or `your_plugin_name.dll` (on Windows).

## Differences between Cargo.toml Files

Both the main project using plugins and the individual plugin projects are end users of the plugin_manager.

1. Main Project Cargo.toml:
    - Located in the root of the project that will use plugins.
    - Includes `genja-plugin-manager` as a dependency.
    - Does not need the `crate-type` specification.
    - Does not contain any metadata for plugin configuration.
    - The loaded plugins are dependant on the plugins specified in the `End-User's` project Cargo.toml.

    Example:

    ```toml
    [package]
    name = "main_project"
    version = "0.1.0"
    edition = "2024"

    [dependencies]
    genja-plugin-manager = "0.1.0"
    ```

2. Plugin Project Cargo.toml:

    - Located in a separate project directory for each plugin.
    - Includes `genja-plugin-manager` as a dependency.
    - Specifies `crate-type = ["lib", "cdylib"]` to build as both a Rust library and a dynamic library.
    - Does not contain plugin metadata configuration.

    Example:

    ```toml
    [package]
    name = "my_plugin"
    version = "0.1.0"
    edition = "2024"

    [dependencies]
    genja-plugin-manager = "0.1.0"

    [lib]
    name = "my_plugin"
    crate-type = ["lib", "cdylib"]
    ```

3. End-User Project Cargo.toml:

    - Includes the main project as dependencies.
    - Contains metadata for plugin configuration.

    Example:

    ```toml
    [package]
    name = "my_application"
    version = "0.1.0"
    edition = "2024"

    [dependencies]
    main_project = "0.1.0"

    [package.metadata.plugins]
    my_plugin = "/path/to/libmy_plugin.so"
    ```

The main differences between these Cargo.toml files are:

1. The Main Project Cargo.toml sets up the core project that will use plugins:
    - It includes the plugin_manager as a dependency.
    - It doesn't specify crate-type or contain plugin metadata.
    - The plugins it can load are determined by the End-User's project configuration.

2. The Plugin Project Cargo.toml configures individual plugin projects:
    - It includes the plugin_manager as a dependency.
    - It specifies crate-type as both "lib" and "cdylib" to produce a dynamic library.
    - It doesn't contain any plugin metadata configuration.

3. The End-User Project Cargo.toml configures the application that will use the main project and its plugins:
    - It includes the main project (not the plugin_manager directly) as a dependency.
    - It contains the metadata for plugin configuration, specifying which plugins to load and how to group them.

## Plugin Configuration

Plugins are configured in the `Cargo.toml` file of the end-user project:

```toml
[package.metadata.plugins]
plugin_a = "/path/to/plugin_a.so"

[package.metadata.plugins.group_name]
plugin_b = "/path/to/plugin_b.so"
plugin_c = "/path/to/plugin_c.so"
```

## Usage

Here's a basic example of how to use the `PluginManager`:

```rust
use plugin_manager::PluginManager;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a new PluginManager
    let mut plugin_manager = PluginManager::new();

    // Activate plugins based on metadata in Cargo.toml
    plugin_manager = plugin_manager.activate_plugins()?;
    
    // Execute a specific plugin
    plugin_manager.execute_plugin("plugin_a", &())?;
    
    // Deregister a plugin
    let deregistered = plugin_manager.deregister_plugin("plugin_b");
    print!("Deregistered plugin: {:?}", deregistered);
    
    // Deregister all plugins
    let deregistered = plugin_manager.deregister_all_plugins();
    println!("Deregistered plugins: {:?}", deregistered);
    Ok(())
}
```

## License

This project is licensed under the Apache License, Version 2.0 - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
