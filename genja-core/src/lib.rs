//! # Genja Core
//!
//! A network automation library.
//!
//! ## Initialization
//!
//! This library does not initialize logging or other global state.
//! Users must handle initialization in their application.
//!
//! ### Example: Basic Setup
//!
//! ```no_run
//! use genja_core::Settings;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Load settings
//!     let settings = Settings::from_file("config.yaml")?;
//!
//!     // Initialize your application with settings
//!     // ...
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Example: With Logging
//!
//! If you want file-based logging, set it up yourself using the config values:
//!
//! ```no_run
//! use genja_core::Settings;
//! use tracing_subscriber::prelude::*;
//! use tracing_rolling_file::RollingFileAppender;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let settings = Settings::from_file("config.yaml")?;
//!
//!     // Set up logging based on settings
//!     if settings.logging().enabled() {
//!         let appender = RollingFileAppender::builder()
//!             .filename(settings.logging().log_file().to_string())
//!             .condition_max_file_size(settings.logging().file_size())
//!             .max_filecount(settings.logging().max_file_count())
//!             .build()?;
//!
//!         let (non_blocking, _guard) = appender.get_non_blocking_appender();
//!
//!         tracing_subscriber::registry()
//!             .with(tracing_subscriber::fmt::layer().with_writer(non_blocking))
//!             .init();
//!
//!         // Keep _guard alive for the program duration
//!         std::mem::forget(_guard); // Or store it somewhere
//!     }
//!
//!     // Your application logic
//!     Ok(())
//! }
//! ```

pub mod errors;
pub mod inventory;
pub mod settings;
pub mod task;
pub mod types;

pub use errors::InventoryLoadError;
pub use settings::Settings;
pub use types::{CustomTreeMap, NatString};
