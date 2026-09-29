//! Terminal-independent browser state and selection invariants.
//!
//! Descriptor snapshots are replaced only by loading. Callers can update
//! selection, live filter text, panel focus, and inspection scroll positions
//! without terminal access. Inspection reads the selected snapshot descriptor.
//! Quit state belongs to the host app rather than the embedded browser.

use crate::discovery::{DiscoveryError, TaskDescriptor};
use genja_core::task::TaskExecutionMode;

/// Browser view selection, independent of terminal ownership.
/// Details rendering is available through direct actions. Schema rendering,
/// bounded scrolling, and new inspection keyboard transitions are pending.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrowserPanel {
    /// Task navigation panel; the initial focus.
    #[default]
    Tasks,
    /// Inspect the selected descriptor's metadata.
    Details,
    /// Inspect the selected descriptor's optional input schema.
    Schema,
}

/// Owned task snapshot and terminal-independent browser presentation state.
///
/// Private fields ensure selection is absent or points to a matching descriptor
/// in the full snapshot. Filtering preserves discovery order and never reloads
/// descriptors. Inspection requires a selected task; full-screen quit belongs
/// to the host. Detail and schema scroll positions belong to that selection.
#[derive(Debug, Default)]
pub struct TaskBrowserState {
    descriptors: Vec<TaskDescriptor>,
    matching_indices: Vec<usize>,
    selected_index: Option<usize>,
    filter_text: String,
    search_active: bool,
    active_panel: BrowserPanel,
    detail_scroll_offset: usize,
    schema_scroll_offset: usize,
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
        if self.selected_index != index {
            self.reset_inspection_scroll();
        }
        self.selected_index = index;
        self.ensure_inspectable_selection();
        true
    }

    /// Return the search text as supplied, including surrounding whitespace.
    pub fn filter_text(&self) -> &str {
        &self.filter_text
    }

    /// Return whether keyboard input is editing the search query.
    /// Search focus is independent of the retained query and is available only
    /// in Tasks; entering inspection leaves search without clearing its text.
    pub fn is_search_active(&self) -> bool {
        self.search_active
    }

    /// Set search focus without changing the query or selection.
    pub(super) fn set_search_active(&mut self, active: bool) {
        self.search_active = active;
    }

    /// Filter immediately using a case-insensitive substring of ID, name,
    /// version, description, or execution mode (`blocking` or `async`).
    ///
    /// Surrounding whitespace is ignored; blank queries match every task.
    /// Matching uses Unicode lowercase conversion, without fuzzy matching or
    /// accent normalization. A still-visible selected task is retained;
    /// otherwise its previous visible position is clamped to the new list.
    /// No matches clears selection; new matches without selection select first.
    /// A changed selection resets inspection offsets; losing selection also
    /// returns to Tasks. Retaining the selected task preserves its view offsets.
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

    /// Set the view without loading descriptors or rendering.
    ///
    /// Details/Schema requests are ignored without a selected task. Entering
    /// inspection leaves search input while retaining the query and selection.
    /// Switching views preserves their independent scroll offsets.
    pub fn set_active_panel(&mut self, panel: BrowserPanel) {
        if panel != BrowserPanel::Tasks && self.selected_index.is_none() {
            return;
        }
        if panel != BrowserPanel::Tasks {
            self.search_active = false;
        }
        self.active_panel = panel;
    }

    /// Return an inspection view's requested scroll offset in display rows.
    /// Task-list scrolling follows selection and has no inspection offset.
    pub fn inspection_scroll_offset(&self, panel: BrowserPanel) -> Option<usize> {
        match panel {
            BrowserPanel::Tasks => None,
            BrowserPanel::Details => Some(self.detail_scroll_offset),
            BrowserPanel::Schema => Some(self.schema_scroll_offset),
        }
    }

    /// Store a requested display-row offset for an inspection view.
    ///
    /// Return `false` for Tasks or when no task is selected, leaving offsets
    /// unchanged. This state-only operation has no content or viewport bounds;
    /// callers supplying custom inspection rendering must clamp appropriately.
    /// The Details renderer currently starts at the top; browser-owned bounded
    /// scrolling will be added with schema rendering.
    pub fn set_inspection_scroll_offset(&mut self, panel: BrowserPanel, offset: usize) -> bool {
        if self.selected_index.is_none() {
            return false;
        }
        match panel {
            BrowserPanel::Tasks => return false,
            BrowserPanel::Details => self.detail_scroll_offset = offset,
            BrowserPanel::Schema => self.schema_scroll_offset = offset,
        }
        true
    }

    /// Return the most recent loading error, cleared by a successful load.
    pub fn error(&self) -> Option<&DiscoveryError> {
        self.error.as_ref()
    }

    /// Replace the discovery snapshot, reapply the query, and select its first
    /// match. A successful load also clears any previous discovery error.
    pub(super) fn replace_descriptors(&mut self, descriptors: Vec<TaskDescriptor>) {
        self.reset_inspection_scroll();
        self.selected_index = None;
        self.descriptors = descriptors;
        self.rebuild_matches(0);
        self.error = None;
    }

    /// Record a discovery failure and clear tasks, matches, and selection.
    /// Preserve the query, reset inspection offsets, and return to Tasks.
    pub(super) fn record_load_error(&mut self, error: DiscoveryError) {
        self.descriptors.clear();
        self.matching_indices.clear();
        self.selected_index = None;
        self.reset_inspection_scroll();
        self.ensure_inspectable_selection();
        self.error = Some(error);
    }

    /// Recompute matching full-snapshot indices in discovery order.
    ///
    /// Trim and lowercase the query before checking each descriptor. Keep the
    /// selected task if it still matches; otherwise use `previous_position`,
    /// its position in the previous filtered list, clamped to the new list.
    /// An empty result clears selection. The full snapshot remains unchanged.
    fn rebuild_matches(&mut self, previous_position: usize) {
        let previous_selection = self.selected_index;
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
        if self.selected_index != previous_selection {
            self.reset_inspection_scroll();
        }
        self.ensure_inspectable_selection();
    }

    /// Reset both view offsets when their selected descriptor changes.
    fn reset_inspection_scroll(&mut self) {
        self.detail_scroll_offset = 0;
        self.schema_scroll_offset = 0;
    }

    /// Leave inspection when there is no descriptor to inspect.
    fn ensure_inspectable_selection(&mut self) {
        if self.selected_index.is_none() {
            self.active_panel = BrowserPanel::Tasks;
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
