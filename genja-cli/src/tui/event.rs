//! Browser actions and Crossterm event translation.
//!
//! Hosts own event polling. They can translate a supplied Crossterm event here
//! or dispatch actions directly, while retaining events the browser ignores.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};

/// An action understood by the task browser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum BrowserAction {
    /// Select the next descriptor, if one exists.
    SelectNext,
    /// Select the previous descriptor, if one exists.
    SelectPrevious,
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

/// Translate a Crossterm event into a browser action.
///
/// Key releases and repeats are ignored. Arrow keys change selection; `q`
/// without Control or Alt and Escape request exit. Resize, focus, mouse, and
/// paste events remain available to the host application.
pub fn action_from_event(event: &Event) -> Option<BrowserAction> {
    let Event::Key(key) = event else {
        return None;
    };
    if key.kind != KeyEventKind::Press {
        return None;
    }

    match key.code {
        KeyCode::Down if key.modifiers.is_empty() => Some(BrowserAction::SelectNext),
        KeyCode::Up if key.modifiers.is_empty() => Some(BrowserAction::SelectPrevious),
        KeyCode::Char('q')
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(BrowserAction::Quit)
        }
        KeyCode::Esc if key.modifiers.is_empty() => Some(BrowserAction::Quit),
        _ => None,
    }
}
