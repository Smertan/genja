//! Embeddable browser and synchronous descriptor loading boundary.
//!
//! Loading owns a descriptor snapshot and retains no source reference. State
//! access never performs discovery. Rendering and action handling will be added
//! separately; the browser never owns terminal setup, polling, or shutdown.

use super::TaskBrowserState;
use crate::discovery::{DiscoveryResult, TaskDescriptorSource};

/// Terminal-independent task browser with an owned descriptor snapshot.
///
/// Supply any shared descriptor source, including a trait object. The browser
/// does not select a registry backend or construct or execute tasks.
///
/// ```
/// use genja_cli::discovery::{DiscoveryResult, TaskDescriptor, TaskDescriptorSource};
/// use genja_cli::tui::TaskBrowser;
///
/// struct EmptySource;
/// impl TaskDescriptorSource for EmptySource {
///     fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
///         Ok(Vec::new())
///     }
/// }
///
/// let mut browser = TaskBrowser::new();
/// browser.load_from(&EmptySource)?;
/// assert!(browser.state().descriptors().is_empty());
/// # Ok::<(), genja_cli::discovery::DiscoveryError>(())
/// ```
#[derive(Debug, Default)]
pub struct TaskBrowser {
    state: TaskBrowserState,
}

impl TaskBrowser {
    /// Create an empty browser with task-panel focus and no error or selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return browser state without loading descriptors.
    pub fn state(&self) -> &TaskBrowserState {
        &self.state
    }

    /// Access validated state updates without exposing mutable descriptors.
    pub fn state_mut(&mut self) -> &mut TaskBrowserState {
        &mut self.state
    }

    /// Load one owned snapshot synchronously using the shared discovery source.
    ///
    /// Calls `list_tasks()` once and preserves its ordering. Success replaces
    /// the snapshot, selects its first task (or none when empty), and clears
    /// any prior error. Failure clears the snapshot and selection, records the
    /// discovery error for presentation, and returns the same error to the host.
    /// Filter text and panel focus are preserved in either case.
    ///
    /// This operation does not retain the source, poll for changes, enter a
    /// terminal mode, or invoke CLI commands. Background loading and refresh
    /// scheduling are not implemented.
    pub fn load_from<S>(&mut self, source: &S) -> DiscoveryResult<()>
    where
        S: TaskDescriptorSource + ?Sized,
    {
        match source.list_tasks() {
            Ok(descriptors) => {
                self.state.replace_descriptors(descriptors);
                Ok(())
            }
            Err(error) => {
                self.state.record_load_error(error.clone());
                Err(error)
            }
        }
    }
}
