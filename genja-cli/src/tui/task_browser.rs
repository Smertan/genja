//! Embeddable browser and synchronous descriptor loading boundary.
//!
//! Loading owns a descriptor snapshot and retains no source reference. State
//! access never performs discovery. Action handling stays separate from terminal
//! event polling; the browser never owns terminal setup or shutdown.

use super::event::{inspection_action_from_event, search_action_from_event};
use super::inspection::inspection_viewport;
use super::layout::render_browser;
use super::{BrowserAction, BrowserOutcome, BrowserPanel, TaskBrowserState, action_from_event};
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
    /// Selection stops at either end of the filtered list. First/last actions
    /// jump to the corresponding descriptor. With no selection, next/previous
    /// select the first descriptor. Empty snapshots ignore navigation. Quit
    /// requests are returned to the host and do not alter browser state.
    /// Inspection actions use the selected snapshot descriptor without loading.
    /// Search editing actions filter immediately. Escape leaves search focus
    /// or inspection before clearing a retained query or requesting quit.
    /// Explicit quit actions always request exit, even while search is focused.
    /// Scroll actions require [`Self::handle_action_in_area`] and are ignored here.
    pub fn handle_action(&mut self, action: BrowserAction) -> BrowserOutcome {
        let visible = self.state.matching_indices();
        let position = self.state.selected_visible_index();
        let selection = match action {
            BrowserAction::SelectNext => self
                .state
                .selected_visible_index()
                .map(|index| index.saturating_add(1))
                .or_else(|| (!visible.is_empty()).then_some(0)),
            BrowserAction::SelectPrevious => self
                .state
                .selected_visible_index()
                .map(|index| index.saturating_sub(1))
                .or_else(|| (!visible.is_empty()).then_some(0)),
            BrowserAction::SelectFirst => (!visible.is_empty()).then_some(0),
            BrowserAction::SelectLast => visible.len().checked_sub(1),
            BrowserAction::OpenDetails => return self.set_panel(BrowserPanel::Details),
            BrowserAction::OpenSchema => return self.set_panel(BrowserPanel::Schema),
            BrowserAction::ReturnToTasks => return self.set_panel(BrowserPanel::Tasks),
            BrowserAction::ScrollUp
            | BrowserAction::ScrollDown
            | BrowserAction::PageUp
            | BrowserAction::PageDown
            | BrowserAction::ScrollToTop
            | BrowserAction::ScrollToBottom => {
                return BrowserOutcome::Ignored;
            }
            BrowserAction::ToggleInspectionView => {
                let panel = match self.state.active_panel() {
                    BrowserPanel::Tasks => return BrowserOutcome::Ignored,
                    BrowserPanel::Details => BrowserPanel::Schema,
                    BrowserPanel::Schema => BrowserPanel::Details,
                };
                return self.set_panel(panel);
            }
            BrowserAction::FocusSearch => return self.set_search_focus(true),
            BrowserAction::LeaveSearch => return self.set_search_focus(false),
            BrowserAction::AppendSearchCharacter(character) => {
                if !self.state.is_search_active() || character.is_control() {
                    return BrowserOutcome::Ignored;
                }
                let mut query = self.state.filter_text().to_owned();
                query.push(character);
                self.state.set_filter_text(query);
                return BrowserOutcome::Changed;
            }
            BrowserAction::DeleteSearchCharacter => {
                if !self.state.is_search_active() {
                    return BrowserOutcome::Ignored;
                }
                let mut query = self.state.filter_text().to_owned();
                if query.pop().is_none() {
                    return BrowserOutcome::Ignored;
                }
                self.state.set_filter_text(query);
                return BrowserOutcome::Changed;
            }
            BrowserAction::ClearSearch => {
                if self.state.filter_text().is_empty() {
                    return BrowserOutcome::Ignored;
                }
                self.state.set_filter_text("");
                return BrowserOutcome::Changed;
            }
            BrowserAction::Escape => {
                if self.state.is_search_active() {
                    return self.set_search_focus(false);
                }
                if self.state.active_panel() != BrowserPanel::Tasks {
                    return self.set_panel(BrowserPanel::Tasks);
                }
                if !self.state.filter_text().is_empty() {
                    return self.handle_action(BrowserAction::ClearSearch);
                }
                return BrowserOutcome::QuitRequested;
            }
            BrowserAction::Quit => return BrowserOutcome::QuitRequested,
        };

        if selection == position {
            return BrowserOutcome::Ignored;
        }
        let Some(selection) = selection.and_then(|index| visible.get(index).copied()) else {
            return BrowserOutcome::Ignored;
        };
        if !self.state.select(Some(selection)) {
            BrowserOutcome::Ignored
        } else {
            BrowserOutcome::Changed
        }
    }

    /// Apply an action with the host's current browser area for bounded scrolling.
    ///
    /// Pass the same clipped browser area used for [`Self::render`], including
    /// its title and footer. Scroll actions use wrapped content and inner-area
    /// height; they do nothing in Tasks or when there is no usable viewport.
    /// After resize, actions start from the effective clamped offset. Other
    /// actions delegate to [`Self::handle_action`]. No terminal is accessed.
    pub fn handle_action_in_area(&mut self, action: BrowserAction, area: Rect) -> BrowserOutcome {
        if !matches!(
            action,
            BrowserAction::ScrollUp
                | BrowserAction::ScrollDown
                | BrowserAction::PageUp
                | BrowserAction::PageDown
                | BrowserAction::ScrollToTop
                | BrowserAction::ScrollToBottom
        ) {
            return self.handle_action(action);
        }
        let Some(viewport) = inspection_viewport(&self.state, area) else {
            return BrowserOutcome::Ignored;
        };
        if viewport.inner.width == 0 || viewport.inner.height == 0 {
            return BrowserOutcome::Ignored;
        }
        let panel = self.state.active_panel();
        let requested = self.state.inspection_scroll_offset(panel).unwrap_or(0);
        let current = viewport.effective_offset(requested);
        let maximum = viewport.max_offset();
        let page = usize::from(viewport.inner.height);
        let next = match action {
            BrowserAction::ScrollUp => current.saturating_sub(1),
            BrowserAction::ScrollDown => current.saturating_add(1).min(maximum),
            BrowserAction::PageUp => current.saturating_sub(page),
            BrowserAction::PageDown => current.saturating_add(page).min(maximum),
            BrowserAction::ScrollToTop => 0,
            BrowserAction::ScrollToBottom => maximum,
            _ => return BrowserOutcome::Ignored,
        };
        if next == requested {
            return BrowserOutcome::Ignored;
        }
        self.state.set_inspection_scroll_offset(panel, next);
        BrowserOutcome::Changed
    }

    /// Translate and dispatch one supplied Crossterm event.
    ///
    /// Unrecognized events return [`BrowserOutcome::Ignored`] for the host to
    /// handle. This method never reads from the terminal or quits the process.
    /// `/` focuses search; printable characters then append to the query.
    /// Backspace deletes its last Unicode scalar value and Ctrl+u clears it.
    /// Enter/Escape leave search with the query retained. In task mode, Enter
    /// opens Details. Tab switches Details/Schema; Escape returns to Tasks
    /// or clears a nonempty query before quitting;
    /// `q` always requests quit outside search.
    /// Search mode does not translate task-navigation keys into navigation.
    /// Inspection scroll keys are ignored without an area; use
    /// [`Self::handle_event_in_area`] for complete inspection controls.
    pub fn handle_event(&mut self, event: &Event) -> BrowserOutcome {
        self.action_for_event(event)
            .map(|action| self.handle_action(action))
            .unwrap_or(BrowserOutcome::Ignored)
    }

    /// Dispatch supplied events with the current clipped browser area.
    ///
    /// This supports all [`Self::handle_event`] controls plus bounded inspection
    /// scrolling: Up/`k`, Down/`j`, PageUp/PageDown, and Home/End. Pass the same
    /// area used for [`Self::render`], including the title and footer. Resize
    /// events remain available to the host, which must redraw and provide the
    /// updated area. No terminal input is read and the host decides when to quit.
    pub fn handle_event_in_area(&mut self, event: &Event, area: Rect) -> BrowserOutcome {
        self.action_for_event(event)
            .map(|action| self.handle_action_in_area(action, area))
            .unwrap_or(BrowserOutcome::Ignored)
    }

    /// Select event translation rules without falling through between modes.
    fn action_for_event(&self, event: &Event) -> Option<BrowserAction> {
        if self.state.is_search_active() {
            search_action_from_event(event)
        } else if self.state.active_panel() != BrowserPanel::Tasks {
            inspection_action_from_event(event)
        } else {
            action_from_event(event)
        }
    }

    /// Update search focus and report whether a redraw is needed.
    fn set_search_focus(&mut self, active: bool) -> BrowserOutcome {
        if active && self.state.active_panel() != BrowserPanel::Tasks {
            return BrowserOutcome::Ignored;
        }
        if self.state.is_search_active() == active {
            return BrowserOutcome::Ignored;
        }
        self.state.set_search_active(active);
        BrowserOutcome::Changed
    }

    /// Apply a validated panel transition and report changes in view or focus.
    fn set_panel(&mut self, panel: BrowserPanel) -> BrowserOutcome {
        let previous_panel = self.state.active_panel();
        let previous_search = self.state.is_search_active();
        self.state.set_active_panel(panel);
        if self.state.active_panel() == previous_panel
            && self.state.is_search_active() == previous_search
        {
            BrowserOutcome::Ignored
        } else {
            BrowserOutcome::Changed
        }
    }

    /// Render the active task-list, Details, or Schema view in a caller-owned frame.
    ///
    /// The area is clipped to the frame and may be empty. Rendering does not
    /// load tasks or change browser state. The caller retains terminal and
    /// drawing ownership, so this component can be embedded in another app.
    /// Rows preserve discovery order. The visible range follows selection and
    /// is recomputed on resize without changing state. Narrow areas shorten
    /// headers and omit lower-priority columns; cell text is clipped to fit.
    /// Search focus is styled without taking over the terminal cursor. Short
    /// areas omit the title and count; empty matches and discovery failures
    /// have separate messages. Footer controls reflect the current input mode.
    /// Details formats the selected snapshot descriptor and wraps its text.
    /// Schema shows formatted JSON metadata or an explicit absence message.
    /// Inspection offsets are clamped against wrapped content and the current
    /// viewport without changing state. Inspection never loads or constructs tasks.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect) {
        render_browser(&self.state, frame, area);
    }

    /// Load one owned snapshot synchronously using the shared discovery source.
    ///
    /// Calls `list_tasks()` once and preserves its ordering. Success replaces
    /// the snapshot, reapplies the query, selects its first match, and clears
    /// any prior error. Failure clears the snapshot and selection, records the
    /// discovery error for presentation, and returns the same error to the host.
    /// Filter text is preserved. Every load resets inspection scroll offsets;
    /// a valid selection preserves the current view, while empty matches or a
    /// failure return to Tasks. No descriptor is copied for inspection.
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
