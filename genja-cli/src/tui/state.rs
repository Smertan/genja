//! Terminal-independent browser state and selection invariants.
//!
//! Descriptor snapshots are replaced only by loading. Callers can update
//! selection, reserved filter text, and panel focus without terminal access.
//! Quit state belongs to the host app rather than the embedded browser.

use crate::discovery::{DiscoveryError, TaskDescriptor};

/// Browser panel focus, reserved for the future rendering and event layers.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrowserPanel {
    /// Task navigation panel; the initial focus.
    #[default]
    Tasks,
    /// Descriptor inspection panel; detail rendering is not implemented yet.
    Details,
}

/// Owned task snapshot and terminal-independent browser presentation state.
///
/// Private fields ensure selection is either absent or within the snapshot.
/// Filter text and panel focus are reserved state; they do not filter tasks or
/// render panels yet. Full-screen quit state remains the host's responsibility.
#[derive(Debug, Default)]
pub struct TaskBrowserState {
    descriptors: Vec<TaskDescriptor>,
    selected_index: Option<usize>,
    filter_text: String,
    active_panel: BrowserPanel,
    error: Option<DiscoveryError>,
}

impl TaskBrowserState {
    /// Return the loaded snapshot in the order supplied by discovery.
    pub fn descriptors(&self) -> &[TaskDescriptor] {
        &self.descriptors
    }

    /// Return the selected snapshot index, if any.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Return the selected descriptor, if any.
    pub fn selected_descriptor(&self) -> Option<&TaskDescriptor> {
        self.selected_index
            .and_then(|index| self.descriptors.get(index))
    }

    /// Set selection, returning whether the update was accepted.
    ///
    /// `None` clears selection. An out-of-range index returns `false` and
    /// leaves the current selection unchanged.
    pub fn select(&mut self, index: Option<usize>) -> bool {
        if index.is_some_and(|index| index >= self.descriptors.len()) {
            return false;
        }
        self.selected_index = index;
        true
    }

    /// Return reserved search/filter text; no filtering is applied yet.
    pub fn filter_text(&self) -> &str {
        &self.filter_text
    }

    /// Store search/filter text without changing the snapshot or selection.
    pub fn set_filter_text(&mut self, text: impl Into<String>) {
        self.filter_text = text.into();
    }

    /// Return the active panel, initially [`BrowserPanel::Tasks`].
    pub fn active_panel(&self) -> BrowserPanel {
        self.active_panel
    }

    /// Set panel focus without loading descriptors or rendering.
    pub fn set_active_panel(&mut self, panel: BrowserPanel) {
        self.active_panel = panel;
    }

    /// Return the most recent loading error, cleared by a successful load.
    pub fn error(&self) -> Option<&DiscoveryError> {
        self.error.as_ref()
    }

    pub(super) fn replace_descriptors(&mut self, descriptors: Vec<TaskDescriptor>) {
        self.selected_index = (!descriptors.is_empty()).then_some(0);
        self.descriptors = descriptors;
        self.error = None;
    }

    pub(super) fn record_load_error(&mut self, error: DiscoveryError) {
        self.descriptors.clear();
        self.selected_index = None;
        self.error = Some(error);
    }
}
