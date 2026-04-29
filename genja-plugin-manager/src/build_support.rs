//! Build-time support for copying plugin libraries into the Cargo output directory.
//!
//! This module provides utilities for integrating plugin libraries into Rust applications
//! at build time. It is designed to be used from `build.rs` scripts to automatically copy
//! plugin dynamic libraries from their build locations into the application's output directory,
//! making them available for runtime loading.
//!
//! # Overview
//!
//! The plugin system requires dynamic libraries to be available at runtime in a known location.
//! This module solves that problem by:
//!
//! 1. Reading plugin declarations from `[package.metadata.plugins]` in `Cargo.toml`
//! 2. Resolving plugin library paths (supporting both absolute and relative paths)
//! 3. Copying plugin libraries to `target/{PROFILE}/plugins/`
//! 4. Emitting `cargo:rerun-if-changed` directives for proper incremental builds
//!
//! # Plugin Declaration Format
//!
//! Plugins are declared in the consuming application's `Cargo.toml` under
//! `[package.metadata.plugins]`. The format supports both individual plugins and
//! grouped plugins:
//!
//! ```toml
//! [package.metadata.plugins]
//! # Individual plugin
//! ssh_plugin = "target/{PROFILE}/libssh_plugin.so"
//!
//! # Grouped plugins
//! [package.metadata.plugins.connection]
//! ssh = "../plugins/ssh/target/{PROFILE}/libssh.so"
//! telnet = "../plugins/telnet/target/{PROFILE}/libtelnet.so"
//!
//! # Absolute paths are also supported
//! [package.metadata.plugins.system]
//! audit = "/opt/genja/plugins/libaudit.so"
//! ```
//!
//! # Path Resolution
//!
//! Plugin paths can be specified in three ways:
//!
//! 1. **Relative paths**: Resolved relative to the manifest directory (where `Cargo.toml` lives)
//! 2. **Absolute paths**: Used as-is without modification
//! 3. **Profile placeholders**: The `{PROFILE}` placeholder is replaced with the current build
//!    profile ("debug" or "release")
//!
//! # Build Integration
//!
//! To use this module in your application, add a `build.rs` file to your project root:
//!
//! ```no_run
//! // build.rs
//! fn main() {
//!     genja_plugin_manager::build_support::copy_plugins_from_manifest()
//!         .expect("Failed to copy plugins");
//! }
//! ```
//!
//! Then declare your plugins in `Cargo.toml`:
//!
//! ```toml
//! [package.metadata.plugins]
//! my_plugin = "target/{PROFILE}/libmy_plugin.so"
//! ```
//!
//! # Runtime Loading
//!
//! After build-time copying, plugins can be loaded at runtime using the plugin manager:
//!
//! ```no_run
//! use genja_plugin_manager::PluginManager;
//!
//! let mut manager = PluginManager::new();
//! manager.load_plugins_from_directory("target/debug/plugins")?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Cross-Platform Considerations
//!
//! The helper reads only `[package.metadata.plugins]`, so the plugin path you
//! declare there must use the correct filename for the OS building the
//! application:
//!
//! Linux:
//!
//! ```toml
//! [package.metadata.plugins]
//! my_plugin = "target/{PROFILE}/libmy_plugin.so"
//! ```
//!
//! macOS:
//!
//! ```toml
//! [package.metadata.plugins]
//! my_plugin = "target/{PROFILE}/libmy_plugin.dylib"
//! ```
//!
//! Windows:
//!
//! ```toml
//! [package.metadata.plugins]
//! my_plugin = "target/{PROFILE}/my_plugin.dll"
//! ```
//!
//! # Error Handling
//!
//! All functions in this module return `io::Result<()>`. Common error scenarios include:
//!
//! - Missing environment variables (`CARGO_MANIFEST_DIR`, `OUT_DIR`, `PROFILE`)
//! - Invalid `Cargo.toml` syntax or structure
//! - Missing plugin source files
//! - Permission errors when creating directories or copying files
//! - Invalid plugin path configurations (e.g., paths with no filename)
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! // build.rs
//! fn main() {
//!     genja_plugin_manager::build_support::copy_plugins_from_manifest()
//!         .expect("Failed to copy plugins");
//! }
//! ```
//!
//! ## With Error Handling
//!
//! ```no_run
//! // build.rs
//! fn main() {
//!     if let Err(e) = genja_plugin_manager::build_support::copy_plugins_from_manifest() {
//!         eprintln!("Warning: Failed to copy plugins: {}", e);
//!         eprintln!("Plugins may not be available at runtime");
//!     }
//! }
//! ```
//!
//! ## Multiple Plugin Groups
//!
//! ```toml
//! [package.metadata.plugins.connection]
//! ssh = "target/{PROFILE}/libssh.so"
//! telnet = "target/{PROFILE}/libtelnet.so"
//!
//! [package.metadata.plugins.inventory]
//! file = "target/{PROFILE}/libfile_inventory.so"
//! database = "target/{PROFILE}/libdb_inventory.so"
//!
//! [package.metadata.plugins.runner]
//! threaded = "target/{PROFILE}/libthreaded_runner.so"
//! serial = "target/{PROFILE}/libserial_runner.so"
//! ```
//!
//! # Implementation Notes
//!
//! - The module uses `cargo:rerun-if-changed` directives to ensure plugins are recopied
//!   when source files change
//! - Plugin directory structure is created automatically if it doesn't exist
//! - Existing plugin files in the destination are overwritten without warning
//! - The module processes nested plugin groups recursively
//! - Profile resolution happens before path resolution, allowing profile-specific paths

use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Copy plugin libraries declared in `[package.metadata.plugins]` from the
/// calling application's `Cargo.toml` into `target/{PROFILE}/plugins`.
///
/// This helper is intended to be called from an end-user application's
/// `build.rs`, where `CARGO_MANIFEST_DIR`, `OUT_DIR`, and `PROFILE` all refer
/// to the consuming application rather than a dependency crate.
///
/// # Behavior
///
/// The function reads the `Cargo.toml` manifest from the directory specified by
/// the `CARGO_MANIFEST_DIR` environment variable and looks for plugin entries under
/// `[package.metadata.plugins]`. If found, it copies the specified plugin libraries
/// to a `plugins` subdirectory within the Cargo profile output directory
/// (e.g., `target/debug/plugins` or `target/release/plugins`).
///
/// Plugin paths can contain the `{PROFILE}` placeholder, which will be replaced
/// with the current build profile (e.g., "debug" or "release").
///
/// # Returns
///
/// Returns `Ok(())` if the operation succeeds or if no plugins are declared.
/// Returns an `Err` containing an `io::Error` if:
/// - Required environment variables (`CARGO_MANIFEST_DIR`, `OUT_DIR`, `PROFILE`) are not set
/// - The manifest file cannot be read or parsed
/// - The profile output directory cannot be resolved
/// - Plugin files cannot be copied to the destination
///
/// # Errors
///
/// This function will return an error if:
/// - The `CARGO_MANIFEST_DIR`, `OUT_DIR`, or `PROFILE` environment variables are not set
/// - The `Cargo.toml` file cannot be read or contains invalid TOML
/// - The Cargo profile output directory structure is unexpected
/// - The destination `plugins` directory cannot be created
/// - Any plugin file cannot be copied to the destination
///
/// # Examples
///
/// ```no_run
/// // In your build.rs:
/// fn main() {
///     genja_plugin_manager::build_support::copy_plugins_from_manifest()
///         .expect("Failed to copy plugins");
/// }
/// ```
pub fn copy_plugins_from_manifest() -> io::Result<()> {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(io::Error::other)?);
    let manifest_path = manifest_dir.join("Cargo.toml");
    println!("cargo:rerun-if-changed={}", manifest_path.display());

    let manifest = fs::read_to_string(&manifest_path)?;
    let value: toml::Value = toml::from_str(&manifest)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

    let Some(plugins) = value
        .get("package")
        .and_then(|value| value.get("metadata"))
        .and_then(|value| value.get("plugins"))
    else {
        return Ok(());
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(io::Error::other)?);
    let profile_dir = out_dir
        .ancestors()
        .nth(3)
        .ok_or_else(|| io::Error::other("failed to resolve Cargo profile output directory"))?;
    let plugin_dir = profile_dir.join("plugins");
    fs::create_dir_all(&plugin_dir)?;

    let profile = env::var("PROFILE").map_err(io::Error::other)?;
    copy_plugin_entries(plugins, &manifest_dir, &plugin_dir, &profile)
}

/// Recursively copies plugin entries from TOML configuration to the plugin directory.
///
/// This function processes plugin entries from the `[package.metadata.plugins]` section
/// of a `Cargo.toml` manifest. It handles both string paths (individual plugin files)
/// and nested tables (groups of plugins), copying each plugin library to the specified
/// plugin directory.
///
/// For string entries, the function:
/// - Replaces `{PROFILE}` placeholders with the actual build profile
/// - Resolves relative paths against the manifest directory
/// - Emits `cargo:rerun-if-changed` directives for build system integration
/// - Copies the plugin file to the destination directory
///
/// For table entries, the function recursively processes all nested values.
///
/// # Parameters
///
/// * `value` - A TOML value representing either a plugin path (string) or a nested
///   table of plugin entries. Must be either a `toml::Value::String` or
///   `toml::Value::Table`.
/// * `manifest_dir` - The directory containing the `Cargo.toml` manifest file. Used
///   as the base directory for resolving relative plugin paths.
/// * `plugin_dir` - The destination directory where plugin libraries should be copied.
///   Typically `target/{PROFILE}/plugins`.
/// * `profile` - The current Cargo build profile (e.g., "debug" or "release"). Used
///   to replace `{PROFILE}` placeholders in plugin paths.
///
/// # Returns
///
/// Returns `Ok(())` if all plugin entries are successfully processed and copied.
/// Returns an `Err` containing an `io::Error` if:
/// - A plugin path is not a valid string or table
/// - A plugin path has no filename component
/// - A plugin file cannot be read or copied
/// - Any nested entry fails to process
///
/// # Errors
///
/// This function will return an error if:
/// - The `value` is neither a string nor a table
/// - A plugin path string has no filename (e.g., ends with `/`)
/// - A source plugin file does not exist or cannot be read
/// - The destination directory is not writable
/// - File copy operations fail for any reason
fn copy_plugin_entries(
    value: &toml::Value,
    manifest_dir: &Path,
    plugin_dir: &Path,
    profile: &str,
) -> io::Result<()> {
    match value {
        toml::Value::String(raw_path) => {
            let resolved = raw_path.replace("{PROFILE}", profile);
            let source = resolve_source_path(manifest_dir, &resolved);
            println!("cargo:rerun-if-changed={}", source.display());

            let filename = source.file_name().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("plugin path has no filename: {}", source.display()),
                )
            })?;
            let destination = plugin_dir.join(filename);
            fs::copy(&source, &destination)?;
            Ok(())
        }
        toml::Value::Table(table) => {
            for nested in table.values() {
                copy_plugin_entries(nested, manifest_dir, plugin_dir, profile)?;
            }
            Ok(())
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "package.metadata.plugins entries must be strings or tables of strings",
        )),
    }
}

/// Resolves a plugin source path relative to the manifest directory.
///
/// This function takes a raw path string and resolves it to an absolute path.
/// If the path is already absolute, it is returned as-is. If the path is relative,
/// it is joined with the manifest directory to produce an absolute path.
///
/// # Parameters
///
/// * `manifest_dir` - The base directory containing the `Cargo.toml` manifest file.
///   Used as the reference point for resolving relative paths.
/// * `raw_path` - The raw path string from the plugin configuration. Can be either
///   an absolute path or a relative path.
///
/// # Returns
///
/// Returns a `PathBuf` containing the resolved absolute path. If `raw_path` is
/// absolute, it is returned unchanged. If `raw_path` is relative, it is resolved
/// relative to `manifest_dir`.
fn resolve_source_path(manifest_dir: &Path, raw_path: &str) -> PathBuf {
    let path = PathBuf::from(raw_path);
    if path.is_absolute() {
        path
    } else {
        manifest_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_source_path;
    use std::path::Path;

    #[test]
    fn resolve_source_path_uses_manifest_dir_for_relative_paths() {
        let base = Path::new("/tmp/app");
        let resolved = resolve_source_path(base, "target/debug/libplugin.so");
        assert_eq!(resolved, base.join("target/debug/libplugin.so"));
    }

    #[test]
    fn resolve_source_path_preserves_absolute_paths() {
        let base = Path::new("/tmp/app");
        let resolved = resolve_source_path(base, "/opt/plugins/libplugin.so");
        assert_eq!(resolved, Path::new("/opt/plugins/libplugin.so"));
    }
}
