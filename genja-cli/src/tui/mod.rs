//! Optional Ratatui task browser foundation.
//!
//! Enable the `tui` feature on `genja-cli`, or `genja-tui` on `genja`, to
//! compile this module. [`TaskBrowser`] provides state and synchronous descriptor
//! loading, event handling, and a minimal renderable shell. The full-screen
//! runner and process helper are not implemented yet.
//!
//! # Architecture
//!
//! The implementation is divided into the following private modules. Public
//! browser types are re-exported here; runner helpers will follow in later phases.
//!
//! - `app`: full-screen orchestration and process entrypoint integration.
//! - `terminal`: Crossterm terminal setup and restoration.
//! - `task_browser`: embeddable browser and descriptor loading boundary.
//! - `state`: descriptor snapshot, selection, filter text, panel, and error state.
//! - `event`: browser actions and translation from Crossterm events.
//! - `layout`: layout calculation and rendering within a caller-provided area.
//!
//! Descriptor loading must use [`crate::discovery::TaskDescriptorSource`].
//! Rendering and state transitions must neither load descriptors nor shell out
//! to CLI commands. Only the process helper will select
//! [`crate::discovery::rust::CompiledTaskDescriptorSource`] by default; the
//! browser and lower-level runner must accept the shared source abstraction.
//!
//! Terminal ownership belongs to the full-screen runner. Embedded callers
//! retain control of terminal setup, event polling, drawing, and shutdown.
//! Browser actions may request an exit, but only the host decides whether to
//! quit. State transitions and rendering must remain testable without a live
//! terminal, using synthetic actions and Ratatui's test backend.
//!
//! # Dependency boundary
//!
//! Ratatui provides rendering and Crossterm provides terminal control and
//! events. There is no direct dependency on `ratatui-crossterm`; Ratatui may
//! use it internally. CLI-only builds do not activate Ratatui through this
//! crate. Crossterm is already a transitive dependency of CLI table output.

mod app;
mod event;
mod layout;
mod state;
mod task_browser;
mod terminal;

pub use event::{BrowserAction, BrowserOutcome, action_from_event};
pub use state::{BrowserPanel, TaskBrowserState};
pub use task_browser::TaskBrowser;
