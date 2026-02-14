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
//!     if settings.logging.enabled {
//!         let appender = RollingFileAppender::builder()
//!             .filename(settings.logging.log_file)
//!             .condition_max_file_size(settings.logging.file_size)
//!             .max_filecount(settings.logging.max_file_count)
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

pub mod inventory;
pub mod settings;
pub mod types;

// Re-export commonly used types
use inventory::{Host, Inventory};
pub use settings::Settings;
use std::sync::Arc;
pub use types::{CustomTreeMap, NatString};
/// Represents a Genja inventory and runtime environment.
///
/// `host_ids` is equal to a Vec of NatString's due to the wrapper used
/// to store the CustomTreeMap's keys.
#[derive(Debug)]
pub struct Genja {
    inventory: Arc<Inventory>,
    host_ids: Arc<Vec<NatString>>,
    config: Arc<Settings>,
    // data: Arc<GlobalState>,
    // processors: Arc<Processors>,
    // runner: Option<Arc<dyn RunnerPlugin>>,
}

impl Genja {
    /// The host_ids are a Vec of owned NatString's, therefore they need
    /// to be cloned from the inventory's CustomTreeMap's keys.
    pub fn new(inventory: Inventory) -> Self {
        let host_ids = inventory.hosts.keys().cloned().collect();
        Self {
            inventory: Arc::new(inventory),
            host_ids: Arc::new(host_ids),
            config: Arc::new(Settings::default()),
            // data: Arc::new(GlobalState::default()),
            // processors: Arc::new(Processors::default()),
            // runner: None,
        }
    }
    /// The `host_key` is a NatString due to the wrapper used to store the CustomTreeMap's keys.
    /// The method `into` converts it to a string.
    pub fn filter(&self, pred: impl Fn(&Host) -> bool) -> Self {
        let host_ids = self
            .inventory
            .hosts
            .iter()
            .filter_map(|(id, host)| if pred(host) { Some(id.clone()) } else { None })
            .collect();

        Self {
            inventory: Arc::clone(&self.inventory),
            host_ids: Arc::new(host_ids),
            config: Arc::clone(&self.config),
            // data: Arc::clone(&self.data),
            // processors: Arc::clone(&self.processors),
            // runner: self.runner.as_ref().map(Arc::clone),
        }
    }

    pub fn iter_hosts(&self) -> impl Iterator<Item = &Host> {
        self.host_ids
            .iter()
            .filter_map(|id| self.inventory.hosts.get(id))
    }

    pub fn iter_all_hosts(&self) -> impl Iterator<Item = (&NatString, &Host)> {
        self.inventory.hosts.iter()
    }

    pub fn host_count(&self) -> usize {
        self.host_ids.len()
    }
}
