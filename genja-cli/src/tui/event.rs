//! Browser actions and Crossterm event translation.
//!
//! Hosts own event polling. They can translate a supplied Crossterm event here
//! or dispatch actions directly, while retaining events the browser ignores.
//! The browser selects translation rules according to its search focus.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

/// An action understood by the task browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrowserAction {
    /// Select the next descriptor, if one exists.
    SelectNext,
    /// Select the previous descriptor, if one exists.
    SelectPrevious,
    /// Select the first descriptor, if one exists.
    SelectFirst,
    /// Select the last descriptor, if one exists.
    SelectLast,
    /// Open metadata inspection for the selected task, leaving search input.
    OpenDetails,
    /// Open input-schema inspection for the selected task, leaving search input.
    OpenSchema,
    /// Switch between Details and Schema; ignored in Tasks.
    ToggleInspectionView,
    /// Return to Tasks without clearing the query or changing selection.
    ReturnToTasks,
    /// Focus search input without clearing the current query.
    FocusSearch,
    /// Leave search input while retaining the current query.
    LeaveSearch,
    /// Append a printable character while search input is focused.
    AppendSearchCharacter(char),
    /// Remove the last Unicode scalar value while search input is focused.
    DeleteSearchCharacter,
    /// Clear the query without changing search focus.
    ClearSearch,
    /// Leave search, then inspection, otherwise clear a query or request quit.
    Escape,
    /// Ask the host application to quit.
    Quit,
}

/// Result of dispatching a browser action or terminal event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrowserOutcome {
    /// The browser state changed and should be redrawn.
    Changed,
    /// No browser state changed; the host may handle the event.
    Ignored,
    /// The browser requests an exit; the host decides whether to quit.
    QuitRequested,
}

/// Translate a Crossterm event using task-navigation controls.
///
/// Key releases and repeats are ignored. Unmodified Down/`j` and Up/`k` change
/// selection; Home and End select the first and last descriptor. `q` without
/// Control or Alt requests exit. `/` focuses search. Escape yields
/// [`BrowserAction::Escape`], whose effect depends on browser state.
/// For context-aware search editing, use [`super::TaskBrowser::handle_event`].
/// Resize, focus, mouse, and paste events remain available to the host.
pub fn action_from_event(event: &Event) -> Option<BrowserAction> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match key.code {
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
            Some(BrowserAction::SelectNext)
        }
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => {
            Some(BrowserAction::SelectPrevious)
        }
        KeyCode::Home if key.modifiers.is_empty() => Some(BrowserAction::SelectFirst),
        KeyCode::End if key.modifiers.is_empty() => Some(BrowserAction::SelectLast),
        KeyCode::Char('q')
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(BrowserAction::Quit)
        }
        KeyCode::Char('/') if (key.modifiers - KeyModifiers::SHIFT).is_empty() => {
            Some(BrowserAction::FocusSearch)
        }
        KeyCode::Esc if key.modifiers.is_empty() => Some(BrowserAction::Escape),
        _ => None,
    }
}

/// Translate supplied key presses for an active search input.
/// Printable characters allow Shift; Control/Alt combinations are ignored
/// except Ctrl+u. Navigation keys, releases, repeats, and non-key events are
/// left to the host. This function never polls terminal input.
pub(super) fn search_action_from_event(event: &Event) -> Option<BrowserAction> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }
    match key.code {
        KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
            Some(BrowserAction::ClearSearch)
        }
        KeyCode::Char(character)
            if !character.is_control() && (key.modifiers - KeyModifiers::SHIFT).is_empty() =>
        {
            Some(BrowserAction::AppendSearchCharacter(character))
        }
        KeyCode::Backspace if key.modifiers.is_empty() => {
            Some(BrowserAction::DeleteSearchCharacter)
        }
        KeyCode::Enter if key.modifiers.is_empty() => Some(BrowserAction::LeaveSearch),
        KeyCode::Esc if key.modifiers.is_empty() => Some(BrowserAction::Escape),
        _ => None,
    }
}
