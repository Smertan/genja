//! Optional Ratatui task browser and full-screen runner.
//!
//! Enable the `tui` feature on `genja-cli`, or `genja-tui` on `genja`, to
//! compile this module. [`TaskBrowser`] provides state and synchronous descriptor
//! loading, context-aware search input, and a renderable filtered task table
//! with result counts and empty/error states.
//! Direct actions open Details/Schema views for the selected descriptor.
//! Area-aware actions scroll wrapped content; new keyboard bindings are pending.
//! [`run_tui`] owns a full-screen terminal session. [`run_main`] selects compiled Rust tasks for
//! project-local binaries.
//!
//! # Architecture
//!
//! The implementation is divided into the following private modules. Public
//! browser types, the lower-level runner, and the process helper are re-exported
//! here.
//!
//! - `app`: full-screen orchestration and runner errors.
//! - `terminal`: Crossterm terminal setup and restoration.
//! - `task_browser`: embeddable browser and descriptor loading boundary.
//! - `detail`: descriptor metadata formatting without state or terminal ownership.
//! - `inspection`: schema formatting and shared wrapped-content viewport bounds.
//! - `state`: snapshot, selection, search, views, inspection scroll offsets, and errors.
//! - `event`: browser actions and translation from Crossterm events.
//! - `layout`: layout calculation and rendering within a caller-provided area.
//! - `widgets`: descriptor row formatting and task table rendering.
//!
//! Descriptor loading must use [`crate::discovery::TaskDescriptorSource`].
//! Rendering and state transitions must neither load descriptors nor shell out
//! to CLI commands. The process helper and CLI launch command select
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
mod detail;
mod event;
mod inspection;
mod layout;
mod state;
mod task_browser;
mod terminal;
mod widgets;

pub use app::{TuiError, TuiOptions, run_main, run_tui};
pub use event::{BrowserAction, BrowserOutcome, action_from_event};
pub use state::{BrowserPanel, TaskBrowserState};
pub use task_browser::TaskBrowser;
