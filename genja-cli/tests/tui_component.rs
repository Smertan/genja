#![cfg(feature = "tui")]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use genja_cli::discovery::{DiscoveryError, DiscoveryResult, TaskDescriptor, TaskDescriptorSource};
use genja_cli::tui::{BrowserAction, BrowserOutcome, TaskBrowser, action_from_event};
use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

struct Source(DiscoveryResult<Vec<TaskDescriptor>>);

impl TaskDescriptorSource for Source {
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        self.0.clone()
    }
}

fn descriptor(id: &str) -> TaskDescriptor {
    TaskDescriptor::explicit(
        id,
        "1.0.0",
        TaskDescriptorMetadata {
            name: id.to_string(),
            description: None,
            execution_mode: TaskExecutionMode::Blocking,
            connection_plugin_name: None,
            processor_names: Vec::new(),
            retry: None,
        },
        None,
        false,
    )
}

fn key(code: KeyCode, modifiers: KeyModifiers, kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent::new_with_kind(code, modifiers, kind))
}

fn row(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

#[test]
fn action_dispatch_changes_selection_and_asks_host_to_quit() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&Source(Ok(vec![descriptor("first"), descriptor("second")])))
        .unwrap();

    assert_eq!(
        browser.handle_action(BrowserAction::SelectPrevious),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        browser.handle_action(BrowserAction::SelectNext),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().selected_index(), Some(1));
    assert_eq!(
        browser.handle_action(BrowserAction::SelectNext),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        browser.handle_action(BrowserAction::SelectPrevious),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().selected_index(), Some(0));
    assert_eq!(
        browser.handle_action(BrowserAction::Quit),
        BrowserOutcome::QuitRequested
    );
    assert_eq!(browser.state().selected_index(), Some(0));
}

#[test]
fn unselected_and_empty_browsers_keep_selection_valid() {
    let mut browser = TaskBrowser::new();
    assert_eq!(
        browser.handle_action(BrowserAction::SelectNext),
        BrowserOutcome::Ignored
    );
    browser
        .load_from(&Source(Ok(vec![descriptor("first")])))
        .unwrap();
    browser.state_mut().select(None);
    assert_eq!(
        browser.handle_action(BrowserAction::SelectPrevious),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().selected_index(), Some(0));
    browser.load_from(&Source(Ok(Vec::new()))).unwrap();
    assert_eq!(
        browser.handle_action(BrowserAction::SelectPrevious),
        BrowserOutcome::Ignored
    );
    assert_eq!(browser.state().selected_index(), None);
}

#[test]
fn event_translation_leaves_unrecognized_events_to_host() {
    let mut browser = TaskBrowser::new();
    let down = key(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Press);
    let up = key(KeyCode::Up, KeyModifiers::NONE, KeyEventKind::Press);
    let quit = key(KeyCode::Char('q'), KeyModifiers::NONE, KeyEventKind::Press);
    assert_eq!(action_from_event(&down), Some(BrowserAction::SelectNext));
    assert_eq!(action_from_event(&up), Some(BrowserAction::SelectPrevious));
    assert_eq!(browser.handle_event(&quit), BrowserOutcome::QuitRequested);
    assert_eq!(
        browser.handle_event(&key(KeyCode::Esc, KeyModifiers::NONE, KeyEventKind::Press)),
        BrowserOutcome::QuitRequested
    );

    let ignored = [
        key(
            KeyCode::Char('q'),
            KeyModifiers::CONTROL,
            KeyEventKind::Press,
        ),
        key(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Repeat),
        key(
            KeyCode::Char('q'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        ),
        Event::Resize(10, 5),
        Event::FocusGained,
    ];
    for event in ignored {
        assert_eq!(action_from_event(&event), None);
        assert_eq!(browser.handle_event(&event), BrowserOutcome::Ignored);
    }
}

#[test]
fn shell_renders_count_panels_and_status_without_terminal_ownership() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&Source(Ok(vec![descriptor("first")])))
        .unwrap();
    let mut terminal = Terminal::new(TestBackend::new(64, 8)).unwrap();
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();

    assert!(row(&terminal, 0).contains("Genja tasks (1)"));
    assert!(row(&terminal, 1).contains("Tasks"));
    assert!(row(&terminal, 1).contains("Details"));
    assert!(row(&terminal, 7).contains("q / Esc: quit"));
    assert_eq!(browser.state().selected_index(), Some(0));
}

#[test]
fn shell_shows_empty_count_and_discovery_error() {
    let mut browser = TaskBrowser::new();
    browser.load_from(&Source(Ok(Vec::new()))).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 4)).unwrap();
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();
    assert!(row(&terminal, 0).contains("Genja tasks (0)"));

    let error = DiscoveryError::source_failed("registry unavailable");
    assert_eq!(browser.load_from(&Source(Err(error.clone()))), Err(error));
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();
    assert!(
        row(&terminal, 3)
            .contains("Discovery error: task descriptor source failed: registry unavailable")
    );
}

#[test]
fn rendering_clips_to_supplied_area_and_handles_tiny_areas() {
    let browser = TaskBrowser::new();
    let mut terminal = Terminal::new(TestBackend::new(20, 6)).unwrap();
    terminal
        .draw(|frame| browser.render(frame, Rect::new(2, 2, 10, 3)))
        .unwrap();
    assert!(row(&terminal, 0).trim().is_empty());
    assert!(row(&terminal, 2).starts_with("  Genja"));
    assert!(row(&terminal, 2)[12..].trim().is_empty());
    assert!(row(&terminal, 5).trim().is_empty());

    terminal
        .draw(|frame| browser.render(frame, Rect::new(0, 0, 0, 0)))
        .unwrap();
    terminal
        .draw(|frame| browser.render(frame, Rect::new(0, 0, 1, 1)))
        .unwrap();
    terminal
        .draw(|frame| browser.render(frame, Rect::new(19, 5, 20, 10)))
        .unwrap();
    terminal
        .draw(|frame| browser.render(frame, Rect::new(30, 30, 1, 1)))
        .unwrap();
}
