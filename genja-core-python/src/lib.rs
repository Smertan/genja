//! Python bindings for Genja Core.
//!
//! This module provides Python bindings for the Genja infrastructure automation framework,
//! enabling Python applications to leverage Genja's task execution, inventory management,
//! and plugin system capabilities.
//!
//! # Overview
//!
//! The `genja` Python module exposes the following core components:
//!
//! - **Runtime** (`Genja`) - Main entry point for task execution and inventory management
//! - **Plugin Manager** (`PluginManager`) - Dynamic plugin loading and management
//! - **Settings** (`Settings`, `CoreConfig`) - Configuration management with file and environment variable support
//! - **Task System** (`TaskDefinition`, `TaskResults`, `HostTaskResult`) - Task definition and execution results
//!
//! # Architecture
//!
//! This crate uses PyO3 to bridge Rust and Python, providing:
//!
//! 1. **Type Conversion** - Automatic conversion between Rust and Python types
//! 2. **Error Handling** - Rust errors are converted to Python exceptions
//! 3. **Memory Safety** - Rust's ownership model ensures safe Python/Rust interop
//! 4. **Performance** - Native Rust performance with Python convenience
//!
//! # Module Structure
//!
//! The Python module is organized into submodules that mirror the Rust crate structure:
//!
//! ```text
//! genja/
//! ├── Genja              (runtime execution)
//! ├── PluginManager      (plugin management)
//! ├── Settings           (configuration)
//! ├── CoreConfig         (core settings)
//! ├── TaskDefinition     (task definitions)
//! ├── TaskResults        (execution results)
//! └── HostTaskResult     (per-host results)
//! ```
//!
//! # Python Usage Examples
//!
//! ## Basic Task Execution
//!
//! ```python
//! from genja import Genja, Settings
//!
//! # Load configuration
//! settings = Settings.from_file("config.yaml")
//!
//! # Create Genja instance with inventory
//! genja = Genja.builder(inventory).with_settings(settings).build()
//!
//! # Execute task
//! results = genja.run_task(TaskClass, max_depth=10)
//! ```
//!
//! ## Plugin Management
//!
//! ```python
//! from genja import PluginManager
//!
//! # Create plugin manager
//! manager = PluginManager.new()
//!
//! # Load plugins from directory
//! manager.with_path("/path/to/plugins")
//!
//! # Get specific plugin
//! plugin = manager.get_plugin("my_plugin")
//! ```
//!
//! ## Configuration Management
//!
//! ```python
//! from genja import Settings, CoreConfig
//!
//! # Load from file
//! settings = Settings.from_file("config.yaml")
//!
//! # Access configuration
//! core_config = settings.core()
//! if core_config.raise_on_error():
//!     print("Errors will cause immediate termination")
//! ```
//!
//! # Python Stub Files
//!
//! This module includes `.pyi` stub files for type checking and IDE support.
//! These stubs provide type hints for all exposed classes and functions, enabling:
//!
//! - Static type checking with mypy
//! - IDE autocomplete and documentation
//! - Better development experience
//!
//! # Thread Safety
//!
//! The Python bindings are designed to work with Python's Global Interpreter Lock (GIL):
//!
//! - All Rust operations that may block release the GIL when appropriate
//! - Async operations are properly bridged to Python's asyncio
//! - Thread-safe operations are marked as such in the Python API
//!
//! # Error Handling
//!
//! Rust errors are automatically converted to Python exceptions:
//!
//! ```python
//! try:
//!     settings = Settings.from_file("missing.yaml")
//! except Exception as e:
//!     print(f"Configuration error: {e}")
//! ```
//!
//! # Logging
//!
//! Rust-side runtime logs are forwarded into Python's standard `logging` system
//! when the extension module is loaded. Configure Python logging handlers and
//! levels before running Genja tasks if you want to capture or display those
//! records.
//!
//! # Performance Considerations
//!
//! - **Zero-copy where possible** - Data is shared between Rust and Python when safe
//! - **Minimal conversions** - Type conversions are optimized for common cases
//! - **Native execution** - Core logic runs at native Rust speed
//! - **GIL management** - Long-running operations release the GIL to allow concurrency
//!
//! # Development
//!
//! ## Building
//!
//! ```bash
//! # Build the Python extension
//! cargo build --release
//!
//! # Install in development mode
//! pip install -e .
//! ```
//!
//! ## Testing
//!
//! The Rust tests embed Python through PyO3 and depend on packages installed in
//! the PDM-managed virtualenv, including modules such as `pydantic`. Running
//! plain `cargo test` bypasses that environment and can produce false failures
//! from missing Python packages or fixture imports.
//!
//! ```bash
//! # Run Rust tests with the PDM-managed virtualenv
//! pdm run test-rust
//!
//! # Run Python tests
//! pdm run test
//! ```
//!
//! # See Also
//!
//! - `genja-core` - Core Rust implementation
//! - [`genja-plugin-manager`](../genja_plugin_manager/index.html) - Plugin system
//! - [PyO3 Documentation](https://pyo3.rs/) - Python/Rust bindings framework

use pyo3::prelude::*;
use pyo3::types::PyModule;

mod plugin_manager;
mod runtime;
mod settings;
mod task;

/// Initializes the `genja` Python module by registering all submodules.
///
/// This function serves as the entry point for the PyO3-based Python extension module.
/// It registers all core components of the Genja framework, including plugin management,
/// runtime, settings, and task functionality.
///
/// # Parameters
///
/// * `_py` - A reference to the Python interpreter. This parameter is prefixed with an
///   underscore as it's required by the PyO3 framework but not directly used in this function.
/// * `module` - A bound reference to the Python module being initialized. This is the module
///   object that will contain all registered classes, functions, and submodules.
///
/// # Returns
///
/// Returns `PyResult<()>` which is:
/// * `Ok(())` if all submodules are successfully registered
/// * `Err(PyErr)` if any submodule registration fails
#[pymodule]
fn genja(_py: Python<'_>, module: &Bound<'_, PyModule>) -> PyResult<()> {
    let _ = pyo3_log::try_init();

    plugin_manager::register(module)?;
    runtime::register(module)?;
    settings::register(module)?;
    task::register(module)?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn init_embedded_python() {
    use std::path::PathBuf;
    use std::process::Command;
    use std::sync::Once;

    static INIT: Once = Once::new();
    INIT.call_once(pyo3::Python::initialize);

    let mut search_paths = vec![
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("python")
            .display()
            .to_string(),
    ];

    if let Ok(python) = std::env::var("PYO3_PYTHON")
        && let Ok(output) = Command::new(python)
            .args([
                "-c",
                "import sysconfig; print(sysconfig.get_paths().get('purelib', '')); print(sysconfig.get_paths().get('platlib', ''))",
            ])
            .output()
        && output.status.success()
    {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            let path = line.trim();
            if !path.is_empty() && !search_paths.iter().any(|existing| existing == path) {
                search_paths.push(path.to_string());
                }
        }
    }

    Python::attach(|py| {
        let sys = PyModule::import(py, "sys").expect("sys module should import");
        let path = sys.getattr("path").expect("sys.path should exist");
        for search_path in search_paths.iter().rev() {
            path.call_method1("insert", (0, search_path))
                .expect("python search path should be inserted");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genja_module_registers_public_classes() {
        init_embedded_python();
        Python::attach(|py| {
            let module =
                PyModule::new(py, "test_genja_module").expect("test module should be created");

            genja(py, &module).expect("module initialization should succeed");

            assert!(module.getattr("Genja").is_ok());
            assert!(module.getattr("PluginManager").is_ok());
            assert!(module.getattr("Settings").is_ok());
            assert!(module.getattr("CoreConfig").is_ok());
            assert!(module.getattr("TaskDefinition").is_ok());
            assert!(module.getattr("TaskResults").is_ok());
            assert!(module.getattr("HostTaskResult").is_ok());
        });
    }
}
