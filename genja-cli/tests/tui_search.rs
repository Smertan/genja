#![cfg(feature = "tui")]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use genja_cli::discovery::{DiscoveryResult, TaskDescriptor, TaskDescriptorSource};
use genja_cli::tui::{BrowserAction, BrowserOutcome, TaskBrowser, action_from_event};
use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};

struct Source;

impl TaskDescriptorSource for Source {
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        Ok(["backup", "collect"]
            .into_iter()
            .map(|id| {
                TaskDescriptor::explicit(
                    id,
                    "1.0.0",
                    TaskDescriptorMetadata {
                        name: id.into(),
                        description: None,
                        execution_mode: TaskExecutionMode::Blocking,
                        connection_plugin_name: None,
                        processor_names: Vec::new(),
                        retry: None,
                    },
                    None,
                    false,
                )
            })
            .collect())
    }
}

fn press(browser: &mut TaskBrowser, code: KeyCode) -> BrowserOutcome {
    browser.handle_event(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)))
}

#[test]
fn search_filters_live_and_enter_restores_navigation() {
    let mut browser = TaskBrowser::new();
    browser.load_from(&Source).unwrap();
    assert!(!browser.state().is_search_active());
    assert_eq!(
        press(&mut browser, KeyCode::Char('/')),
        BrowserOutcome::Changed
    );
    assert!(browser.state().is_search_active());
    for character in "collect".chars() {
        assert_eq!(
            press(&mut browser, KeyCode::Char(character)),
            BrowserOutcome::Changed
        );
    }
    assert_eq!(browser.state().filter_text(), "collect");
    assert_eq!(browser.state().matching_indices(), [1]);
    assert_eq!(browser.state().selected_index(), Some(1));
    assert_eq!(press(&mut browser, KeyCode::Enter), BrowserOutcome::Changed);
    assert!(!browser.state().is_search_active());
    assert_eq!(browser.state().filter_text(), "collect");
    assert_eq!(press(&mut browser, KeyCode::Down), BrowserOutcome::Ignored);
    assert_eq!(
        press(&mut browser, KeyCode::Char('q')),
        BrowserOutcome::QuitRequested
    );
}

#[test]
fn escape_leaves_search_then_clears_query_then_quits() {
    let mut browser = TaskBrowser::new();
    browser.load_from(&Source).unwrap();
    browser.state_mut().set_filter_text("backup");
    press(&mut browser, KeyCode::Char('/'));
    assert_eq!(browser.state().filter_text(), "backup");
    assert_eq!(press(&mut browser, KeyCode::Esc), BrowserOutcome::Changed);
    assert!(!browser.state().is_search_active());
    assert_eq!(browser.state().filter_text(), "backup");
    assert_eq!(press(&mut browser, KeyCode::Esc), BrowserOutcome::Changed);
    assert_eq!(browser.state().filter_text(), "");
    assert_eq!(browser.state().matching_indices(), [0, 1]);
    assert_eq!(
        press(&mut browser, KeyCode::Esc),
        BrowserOutcome::QuitRequested
    );

    press(&mut browser, KeyCode::Char('/'));
    assert_eq!(press(&mut browser, KeyCode::Esc), BrowserOutcome::Changed);
    assert_eq!(
        press(&mut browser, KeyCode::Esc),
        BrowserOutcome::QuitRequested
    );
}

#[test]
fn search_treats_navigation_letters_and_slash_as_text() {
    let mut browser = TaskBrowser::new();
    press(&mut browser, KeyCode::Char('/'));
    for character in "qjk/".chars() {
        assert_eq!(
            press(&mut browser, KeyCode::Char(character)),
            BrowserOutcome::Changed
        );
    }
    let uppercase = Event::Key(KeyEvent::new(KeyCode::Char('B'), KeyModifiers::SHIFT));
    assert_eq!(browser.handle_event(&uppercase), BrowserOutcome::Changed);
    assert_eq!(browser.state().filter_text(), "qjk/B");
    for code in [KeyCode::Up, KeyCode::Down, KeyCode::Home, KeyCode::End] {
        assert_eq!(press(&mut browser, code), BrowserOutcome::Ignored);
    }
    assert_eq!(browser.state().filter_text(), "qjk/B");
}

#[test]
fn backspace_and_ctrl_u_handle_unicode_and_clear_without_leaving_search() {
    let mut browser = TaskBrowser::new();
    browser.load_from(&Source).unwrap();
    press(&mut browser, KeyCode::Char('/'));
    assert_eq!(
        press(&mut browser, KeyCode::Backspace),
        BrowserOutcome::Ignored
    );
    for character in "é東京".chars() {
        press(&mut browser, KeyCode::Char(character));
    }
    assert!(browser.state().matching_indices().is_empty());
    assert_eq!(
        press(&mut browser, KeyCode::Backspace),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().filter_text(), "é東");
    let clear = Event::Key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert_eq!(browser.handle_event(&clear), BrowserOutcome::Changed);
    assert!(browser.state().is_search_active());
    assert_eq!(browser.state().filter_text(), "");
    assert_eq!(browser.state().matching_indices(), [0, 1]);
    assert_eq!(browser.handle_event(&clear), BrowserOutcome::Ignored);
}

#[test]
fn unsupported_events_and_modifiers_do_not_edit_or_quit_search() {
    let mut browser = TaskBrowser::new();
    press(&mut browser, KeyCode::Char('/'));
    let ignored = [
        Event::Resize(80, 24),
        Event::Paste("backup".into()),
        Event::FocusLost,
        Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::CONTROL)),
        Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::ALT)),
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::SUPER)),
        Event::Key(KeyEvent::new(KeyCode::Char('\n'), KeyModifiers::NONE)),
        Event::Key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::ALT)),
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )),
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Char('x'),
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )),
    ];
    for event in ignored {
        assert_eq!(
            browser.handle_event(&event),
            BrowserOutcome::Ignored,
            "{event:?}"
        );
    }
    assert!(browser.state().is_search_active());
    assert_eq!(browser.state().filter_text(), "");
}

#[test]
fn embedded_hosts_can_dispatch_search_actions_and_explicit_quit() {
    let mut browser = TaskBrowser::new();
    assert_eq!(
        browser.handle_action(BrowserAction::AppendSearchCharacter('b')),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        browser.handle_action(BrowserAction::DeleteSearchCharacter),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        browser.handle_action(BrowserAction::LeaveSearch),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        browser.handle_action(BrowserAction::FocusSearch),
        BrowserOutcome::Changed
    );
    assert_eq!(
        browser.handle_action(BrowserAction::FocusSearch),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        browser.handle_action(BrowserAction::AppendSearchCharacter('b')),
        BrowserOutcome::Changed
    );
    assert_eq!(
        browser.handle_action(BrowserAction::AppendSearchCharacter('\t')),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        browser.handle_action(BrowserAction::Quit),
        BrowserOutcome::QuitRequested
    );
    assert!(browser.state().is_search_active());
    assert_eq!(browser.state().filter_text(), "b");
    browser.handle_action(BrowserAction::LeaveSearch);
    assert_eq!(
        browser.handle_action(BrowserAction::ClearSearch),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().filter_text(), "");
    assert_eq!(
        action_from_event(&Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))),
        Some(BrowserAction::Escape)
    );
    assert_eq!(
        action_from_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('/'),
            KeyModifiers::SHIFT
        ))),
        Some(BrowserAction::FocusSearch)
    );
}
