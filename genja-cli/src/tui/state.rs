//! Terminal-independent browser state and selection invariants.
//!
//! Descriptor snapshots are replaced only by loading. Callers can update
//! selection, live filter text, and panel focus without terminal access.
//! Quit state belongs to the host app rather than the embedded browser.

use crate::discovery::{DiscoveryError, TaskDescriptor};
use genja_core::task::TaskExecutionMode;

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
/// Private fields ensure selection is absent or points to a matching descriptor
/// in the full snapshot. Filtering preserves discovery order and never reloads
/// descriptors. Panel focus is reserved; full-screen quit belongs to the host.
#[derive(Debug, Default)]
pub struct TaskBrowserState {
    descriptors: Vec<TaskDescriptor>,
    matching_indices: Vec<usize>,
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

    /// Return the selected full-snapshot index, not its filtered position.
    pub fn selected_index(&self) -> Option<usize> {
        self.selected_index
    }

    /// Return full-snapshot indices of matching tasks, in discovery order.
    pub fn matching_indices(&self) -> &[usize] {
        &self.matching_indices
    }

    /// Iterate over matching descriptors in discovery order.
    pub fn filtered_descriptors(&self) -> impl ExactSizeIterator<Item = &TaskDescriptor> {
        self.matching_indices
            .iter()
            .map(|&index| &self.descriptors[index])
    }

    /// Return the selected task's position within the filtered list.
    pub fn selected_visible_index(&self) -> Option<usize> {
        self.selected_index.and_then(|selected| {
            self.matching_indices
                .iter()
                .position(|&index| index == selected)
        })
    }

    /// Return the selected descriptor, if any.
    pub fn selected_descriptor(&self) -> Option<&TaskDescriptor> {
        self.selected_index
            .and_then(|index| self.descriptors.get(index))
    }

    /// Set selection, returning whether the update was accepted.
    ///
    /// Indices refer to the full snapshot. `None` clears selection. An
    /// out-of-range or filtered-out index returns `false` without changing it.
    pub fn select(&mut self, index: Option<usize>) -> bool {
        if index.is_some_and(|index| !self.matching_indices.contains(&index)) {
            return false;
        }
        self.selected_index = index;
        true
    }

    /// Return the search text as supplied, including surrounding whitespace.
    pub fn filter_text(&self) -> &str {
        &self.filter_text
    }

    /// Filter immediately using a case-insensitive substring of ID, name,
    /// version, description, or execution mode (`blocking` or `async`).
    ///
    /// Surrounding whitespace is ignored; blank queries match every task.
    /// Matching uses Unicode lowercase conversion, without fuzzy matching or
    /// accent normalization. A still-visible selected task is retained;
    /// otherwise its previous visible position is clamped to the new list.
    /// No matches clears selection; new matches without selection select first.
    pub fn set_filter_text(&mut self, text: impl Into<String>) {
        let text = text.into();
        if self.filter_text == text {
            return;
        }
        let previous_position = self.selected_visible_index().unwrap_or(0);
        self.filter_text = text;
        self.rebuild_matches(previous_position);
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

    /// Replace the discovery snapshot, reapply the query, and select its first
    /// match. A successful load also clears any previous discovery error.
    pub(super) fn replace_descriptors(&mut self, descriptors: Vec<TaskDescriptor>) {
        self.selected_index = None;
        self.descriptors = descriptors;
        self.rebuild_matches(0);
        self.error = None;
    }

    /// Record a discovery failure and clear tasks, matches, and selection.
    /// The query and panel focus are preserved for a later successful load.
    pub(super) fn record_load_error(&mut self, error: DiscoveryError) {
        self.descriptors.clear();
        self.matching_indices.clear();
        self.selected_index = None;
        self.error = Some(error);
    }

    /// Recompute matching full-snapshot indices in discovery order.
    ///
    /// Trim and lowercase the query before checking each descriptor. Keep the
    /// selected task if it still matches; otherwise use `previous_position`,
    /// its position in the previous filtered list, clamped to the new list.
    /// An empty result clears selection. The full snapshot remains unchanged.
    fn rebuild_matches(&mut self, previous_position: usize) {
        let query = self.filter_text.trim().to_lowercase();
        self.matching_indices = self
            .descriptors
            .iter()
            .enumerate()
            .filter_map(|(index, task)| matches_query(task, &query).then_some(index))
            .collect();
        if !self
            .selected_index
            .is_some_and(|index| self.matching_indices.contains(&index))
        {
            self.selected_index = self
                .matching_indices
                .get(previous_position.min(self.matching_indices.len().saturating_sub(1)))
                .copied();
        }
    }
}

/// Check whether any searchable descriptor field contains `query`.
///
/// The caller supplies a trimmed, lowercased query. An empty query matches
/// immediately. Otherwise, lowercase each field and use substring matching
/// across ID, name, version, optional description, and execution mode.
/// Stop at the first matching field; a missing description is an empty string.
fn matches_query(task: &TaskDescriptor, query: &str) -> bool {
    let mode = match task.execution_mode {
        TaskExecutionMode::Blocking => "blocking",
        TaskExecutionMode::Async => "async",
    };
    query.is_empty()
        || [
            task.id.as_str(),
            task.name.as_str(),
            task.version.as_str(),
            task.description.as_deref().unwrap_or(""),
            mode,
        ]
        .iter()
        .any(|field| field.to_lowercase().contains(query))
}
