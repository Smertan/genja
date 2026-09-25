//! Embeddable browser and synchronous descriptor loading boundary.
//!
//! Loading owns a descriptor snapshot and retains no source reference. State
//! access never performs discovery. Action handling stays separate from terminal
//! event polling; the browser never owns terminal setup or shutdown.

use super::layout::render_browser;
use super::{BrowserAction, BrowserOutcome, TaskBrowserState, action_from_event};
use crate::discovery::{DiscoveryResult, TaskDescriptorSource};
use crossterm::event::Event;
use ratatui::{Frame, layout::Rect};

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

    /// Apply a browser action without polling terminal input.
    ///
    /// Selection stops at either end of the loaded snapshot. Quit requests are
    /// returned to the host and do not alter browser state.
    pub fn handle_action(&mut self, action: BrowserAction) -> BrowserOutcome {
        let selection = match action {
            BrowserAction::SelectNext => self
                .state
                .selected_index()
                .map(|index| index.saturating_add(1))
                .or_else(|| (!self.state.descriptors().is_empty()).then_some(0)),
            BrowserAction::SelectPrevious => self
                .state
                .selected_index()
                .map(|index| index.saturating_sub(1))
                .or_else(|| (!self.state.descriptors().is_empty()).then_some(0)),
            BrowserAction::Quit => return BrowserOutcome::QuitRequested,
        };

        if selection == self.state.selected_index() || !self.state.select(selection) {
            BrowserOutcome::Ignored
        } else {
            BrowserOutcome::Changed
        }
    }

    /// Translate and dispatch one supplied Crossterm event.
    ///
    /// Unrecognized events return [`BrowserOutcome::Ignored`] for the host to
    /// handle. This method never reads from the terminal or quits the process.
    pub fn handle_event(&mut self, event: &Event) -> BrowserOutcome {
        action_from_event(event)
            .map(|action| self.handle_action(action))
            .unwrap_or(BrowserOutcome::Ignored)
    }

    /// Render the minimal browser shell inside a caller-owned Ratatui frame.
    ///
    /// The area is clipped to the frame and may be empty. Rendering does not
    /// load tasks or change browser state. The caller retains terminal and
    /// drawing ownership, so this component can be embedded in another app.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        render_browser(&self.state, frame, area);
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
