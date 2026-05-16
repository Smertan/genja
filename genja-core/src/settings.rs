//! Configuration and settings for Genja Core.
//!
//! This module defines the configuration structs that drive Genja behavior,
//! plus helpers for loading from config files and environment variables.
//!
//! **Key points**
//! - All configs implement `Default` and can be created with `::default()`.
//! - Builders allow partial configuration; missing fields are filled with defaults.
//! - `Settings::from_file` loads JSON or YAML and validates SSH config when present.
//!
//! # Configuration Precedence
//!
//! 1. Configuration files (JSON/YAML) are loaded first
//! 2. Environment variables provide defaults for missing fields
//! 3. Hard-coded defaults are used as final fallback
//!
//! # Environment Variables
//!
//! The following environment variables are supported:
//!
//! - `GENJA_CORE_RAISE_ON_ERROR` - Controls error handling behavior (default: false)
//! - `GENJA_INVENTORY_PLUGIN` - Inventory plugin name (default: "FileInventoryPlugin")
//! - `GENJA_RUNNER_PLUGIN` - Runner plugin name (default: "threaded")
//! - `GENJA_LOGGING_LEVEL` - Log level (default: "info")
//! - `GENJA_LOGGING_LOG_FILE` - Log file path (default: "genja.log")
//! - `GENJA_LOGGING_TO_CONSOLE` - Enable console logging (default: false)
//!
//! # Settings Reference
//!
//! See `docs/settings.md` for a complete schema summary and example config files.
//!
//! # Examples
//!
//! ## Defaults
//! ```
//! use genja_core::Settings;
//!
//! let settings = Settings::default();
//! ```
//!
//! ## Builders
//! ```
//! use genja_core::Settings;
//! use genja_core::settings::{LoggingConfig, RunnerConfig};
//!
//! let settings = Settings::builder()
//!     .logging(LoggingConfig::builder().level("debug").build())
//!     .runner(RunnerConfig::builder().plugin("threaded").build())
//!     .build();
//! ```
//!
//! ## Load From File
//! ```no_run
//! use genja_core::Settings;
//!
//! let settings = Settings::from_file("config.yaml")?;
//! # Ok::<(), genja_core::ConfigLoadError>(())
//! ```
//!
//! ## SSH Validation
//! SSH config is validated automatically when calling `Settings::from_file`.
//! For manual validation, use `SSHConfig::validate`.

mod core;
mod env_defaults;
mod inventory;
mod inventory_loading;
mod logging;
mod root;
mod runner;
mod ssh;
mod ssh_loading;

#[cfg(test)]
mod tests;

pub use self::core::{CoreConfig, CoreConfigBuilder};
pub use self::inventory::{
    InventoryConfig, InventoryConfigBuilder, OptionsConfig, OptionsConfigBuilder,
};
pub use self::logging::{LoggingConfig, LoggingConfigBuilder};
pub use self::root::{Settings, SettingsBuilder};
pub use self::runner::{RunnerConfig, RunnerConfigBuilder};
pub use self::ssh::{SSHConfig, SSHConfigBuilder};
