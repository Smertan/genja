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

fn assert_selected_visible(terminal: &Terminal<TestBackend>, id: &str) {
    let selected: Vec<_> = (0..terminal.backend().buffer().area.height)
        .map(|y| row(terminal, y))
        .filter(|line| line.contains('>'))
        .collect();
    assert_eq!(selected.len(), 1, "{selected:?}");
    assert!(selected[0].contains(id), "{selected:?}");
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
fn navigation_keys_move_selection_and_stop_at_both_ends() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&Source(Ok(vec![
            descriptor("first"),
            descriptor("middle"),
            descriptor("last"),
        ])))
        .unwrap();

    let cases = [
        (KeyCode::End, 2, BrowserOutcome::Changed),
        (KeyCode::End, 2, BrowserOutcome::Ignored),
        (KeyCode::Down, 2, BrowserOutcome::Ignored),
        (KeyCode::Char('j'), 2, BrowserOutcome::Ignored),
        (KeyCode::Char('k'), 1, BrowserOutcome::Changed),
        (KeyCode::Home, 0, BrowserOutcome::Changed),
        (KeyCode::Home, 0, BrowserOutcome::Ignored),
        (KeyCode::Up, 0, BrowserOutcome::Ignored),
        (KeyCode::Char('k'), 0, BrowserOutcome::Ignored),
        (KeyCode::Char('j'), 1, BrowserOutcome::Changed),
        (KeyCode::Down, 2, BrowserOutcome::Changed),
        (KeyCode::Char('q'), 2, BrowserOutcome::QuitRequested),
        (KeyCode::Esc, 2, BrowserOutcome::QuitRequested),
    ];
    for (code, selection, outcome) in cases {
        assert_eq!(
            browser.handle_event(&key(code, KeyModifiers::NONE, KeyEventKind::Press)),
            outcome,
            "{code:?}"
        );
        assert_eq!(
            browser.state().selected_index(),
            Some(selection),
            "{code:?}"
        );
    }
}

#[test]
fn navigation_handles_empty_single_task_and_cleared_selection() {
    let actions = [
        BrowserAction::SelectNext,
        BrowserAction::SelectPrevious,
        BrowserAction::SelectFirst,
        BrowserAction::SelectLast,
    ];
    let mut browser = TaskBrowser::new();
    for action in actions {
        assert_eq!(browser.handle_action(action), BrowserOutcome::Ignored);
        assert_eq!(browser.state().selected_index(), None);
    }

    for count in [1, 3] {
        browser
            .load_from(&Source(Ok((0..count)
                .map(|index| descriptor(&format!("task-{index}")))
                .collect())))
            .unwrap();
        for action in actions {
            assert!(browser.state_mut().select(None));
            assert_eq!(browser.handle_action(action), BrowserOutcome::Changed);
            let expected = if action == BrowserAction::SelectLast {
                count - 1
            } else {
                0
            };
            assert_eq!(browser.state().selected_index(), Some(expected));
        }
    }

    browser.load_from(&Source(Ok(Vec::new()))).unwrap();
    for action in actions {
        assert_eq!(browser.handle_action(action), BrowserOutcome::Ignored);
        assert_eq!(browser.state().selected_index(), None);
    }
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

    for code in [
        KeyCode::Up,
        KeyCode::Down,
        KeyCode::Char('j'),
        KeyCode::Char('k'),
        KeyCode::Home,
        KeyCode::End,
    ] {
        for modifiers in [
            KeyModifiers::SHIFT,
            KeyModifiers::CONTROL,
            KeyModifiers::ALT,
        ] {
            assert_eq!(
                action_from_event(&key(code, modifiers, KeyEventKind::Press)),
                None
            );
        }
        for kind in [KeyEventKind::Release, KeyEventKind::Repeat] {
            assert_eq!(
                action_from_event(&key(code, KeyModifiers::NONE, kind)),
                None
            );
        }
    }
}

#[test]
fn browser_renders_count_table_and_controls_without_terminal_ownership() {
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
    assert!(row(&terminal, 2).contains("ID"));
    assert!(row(&terminal, 3).contains("first"));
    assert!(row(&terminal, 7).contains("q / Esc: quit"));
    assert_eq!(browser.state().selected_index(), Some(0));
}

#[test]
fn task_table_renders_fields_in_discovery_order_and_moves_highlight() {
    let mut first = descriptor("z.examples.backup_config");
    first.version = "2.1.0".to_string();
    first.name = "backup_config".to_string();
    first.constructible = true;
    let mut second = descriptor("a.examples.collect_facts");
    second.name = "collect_facts".to_string();
    second.execution_mode = TaskExecutionMode::Async;

    let expected = vec![first, second];
    let mut browser = TaskBrowser::new();
    browser.load_from(&Source(Ok(expected.clone()))).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 8)).unwrap();
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();

    let header = row(&terminal, 2);
    for label in ["ID", "VERSION", "NAME", "MODE", "CONSTRUCTIBLE"] {
        assert!(header.contains(label), "{header}");
    }
    let first_row = row(&terminal, 3);
    for value in [
        ">",
        "z.examples.backup_config",
        "2.1.0",
        "backup_config",
        "blocking",
        "yes",
    ] {
        assert!(first_row.contains(value), "{first_row}");
    }
    let second_row = row(&terminal, 4);
    for value in [
        "a.examples.collect_facts",
        "1.0.0",
        "collect_facts",
        "async",
        "no",
    ] {
        assert!(second_row.contains(value), "{second_row}");
    }
    assert!(!second_row.contains('>'));
    assert_eq!(
        terminal.backend().buffer()[(1, 3)].bg,
        ratatui::style::Color::Blue
    );
    assert_eq!(browser.state().descriptors(), expected);
    assert_eq!(browser.state().selected_index(), Some(0));

    assert_eq!(
        browser.handle_action(BrowserAction::SelectNext),
        BrowserOutcome::Changed
    );
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();
    assert!(!row(&terminal, 3).contains('>'));
    assert!(row(&terminal, 4).contains('>'));
    assert_ne!(
        terminal.backend().buffer()[(1, 3)].bg,
        ratatui::style::Color::Blue
    );
    assert_eq!(
        terminal.backend().buffer()[(1, 4)].bg,
        ratatui::style::Color::Blue
    );
    assert_eq!(browser.state().descriptors(), expected);
    assert_eq!(browser.state().selected_index(), Some(1));

    assert!(browser.state_mut().select(None));
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();
    assert!(!row(&terminal, 3).contains('>'));
    assert!(!row(&terminal, 4).contains('>'));
    assert_eq!(browser.state().selected_index(), None);
}

#[test]
fn long_list_navigation_keeps_selection_visible_without_changing_descriptors() {
    let descriptors: Vec<_> = (0..30)
        .map(|index| descriptor(&format!("task-{index:02}")))
        .collect();
    let mut browser = TaskBrowser::new();
    browser.load_from(&Source(Ok(descriptors.clone()))).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 9)).unwrap();

    for index in 0..30 {
        terminal
            .draw(|frame| browser.render(frame, frame.area()))
            .unwrap();
        assert_selected_visible(&terminal, &format!("task-{index:02}"));
        browser.handle_action(BrowserAction::SelectNext);
    }
    assert_eq!(browser.state().selected_index(), Some(29));
    for index in (0..30).rev() {
        terminal
            .draw(|frame| browser.render(frame, frame.area()))
            .unwrap();
        assert_selected_visible(&terminal, &format!("task-{index:02}"));
        browser.handle_action(BrowserAction::SelectPrevious);
    }
    assert_eq!(browser.state().selected_index(), Some(0));
    for (action, id, index) in [
        (BrowserAction::SelectLast, "task-29", 29),
        (BrowserAction::SelectFirst, "task-00", 0),
    ] {
        browser.handle_action(action);
        terminal
            .draw(|frame| browser.render(frame, frame.area()))
            .unwrap();
        assert_selected_visible(&terminal, id);
        assert_eq!(browser.state().selected_index(), Some(index));
    }
    assert_eq!(browser.state().descriptors(), descriptors);
}

#[test]
fn resize_and_cleared_selection_recompute_the_viewport() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&Source(Ok((0..30)
            .map(|index| descriptor(&format!("t{index:02}")))
            .collect())))
        .unwrap();
    assert!(browser.state_mut().select(Some(20)));
    let mut terminal = Terminal::new(TestBackend::new(120, 12)).unwrap();
    for (width, height) in [(120, 12), (80, 6), (50, 6), (30, 8), (120, 35)] {
        terminal.backend_mut().resize(width, height);
        terminal
            .draw(|frame| browser.render(frame, frame.area()))
            .unwrap();
        assert_selected_visible(&terminal, "t20");
        assert_eq!(browser.state().selected_index(), Some(20));
    }

    assert!(browser.state_mut().select(None));
    terminal.backend_mut().resize(120, 9);
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();
    assert!(row(&terminal, 3).contains("t00"));
    assert!(!(0..9).any(|y| row(&terminal, y).contains('>')));
    assert_eq!(browser.state().selected_index(), None);
}

#[test]
fn narrow_tables_prioritize_identity_and_keep_quit_guidance() {
    let mut task = descriptor("task-id");
    task.name = "task-name".to_string();
    task.constructible = true;
    let mut browser = TaskBrowser::new();
    browser.load_from(&Source(Ok(vec![task]))).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(120, 8)).unwrap();
    for width in [120, 80, 50, 30] {
        terminal.backend_mut().resize(width, 8);
        terminal
            .draw(|frame| browser.render(frame, frame.area()))
            .unwrap();
        assert_selected_visible(&terminal, "task-id");
        let header = row(&terminal, 2);
        assert!(header.contains("ID"));
        assert_eq!(header.contains("NAME"), width >= 64);
        assert_eq!(header.contains("MODE"), width >= 40);
        if width >= 40 {
            assert!(row(&terminal, 3).contains("1.0.0"));
            assert!(row(&terminal, 3).contains("blocking"));
            assert!(row(&terminal, 3).contains("yes"));
        }
        assert!(row(&terminal, 7).contains("quit"));
    }
}

#[test]
fn unicode_rows_render_safely_in_tiny_areas() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&Source(Ok(vec![descriptor(
            "例子🌱.a_very_long_task_identifier",
        )])))
        .unwrap();
    let mut terminal = Terminal::new(TestBackend::new(80, 8)).unwrap();
    for width in [0, 1, 2, 3, 8, 20, 39, 40, 63, 64, 99, 100] {
        for height in 0..=6 {
            terminal.backend_mut().resize(width, height);
            terminal
                .draw(|frame| browser.render(frame, frame.area()))
                .unwrap();
            assert_eq!(browser.state().selected_index(), Some(0));
        }
    }
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
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&Source(Ok(vec![descriptor("first")])))
        .unwrap();
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
