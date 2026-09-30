#![cfg(feature = "tui")]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use genja_cli::discovery::{DiscoveryResult, TaskDescriptor, TaskDescriptorSource};
use genja_cli::tui::{BrowserAction, BrowserOutcome, BrowserPanel, TaskBrowser};
use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};
use ratatui::{Terminal, backend::TestBackend, layout::Rect, style::Color};
use serde_json::{Value, json};

struct Source(TaskDescriptor);

impl TaskDescriptorSource for Source {
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        Ok(vec![self.0.clone()])
    }
}

fn browser(schema: Option<Value>) -> TaskBrowser {
    let task = TaskDescriptor::explicit(
        "acme.inspect",
        "1.0.0",
        TaskDescriptorMetadata {
            name: "inspect".into(),
            description: Some(
                (0..40)
                    .map(|index| {
                        format!("Description line {index:02}: Unicode é東京 text wraps safely.")
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
            ),
            execution_mode: TaskExecutionMode::Blocking,
            connection_plugin_name: None,
            processor_names: Vec::new(),
            retry: None,
        },
        schema,
        true,
    );
    let mut browser = TaskBrowser::new();
    browser.load_from(&Source(task)).unwrap();
    browser
}

fn row(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn draw(browser: &TaskBrowser, width: u16, height: u16) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();
    terminal
}

fn text(terminal: &Terminal<TestBackend>) -> String {
    (0..terminal.backend().buffer().area.height)
        .map(|y| row(terminal, y))
        .collect::<Vec<_>>()
        .join("\n")
}

fn color_at(terminal: &Terminal<TestBackend>, needle: &str) -> (Color, Color) {
    let area = terminal.backend().buffer().area;
    let (x, y) = (0..area.height)
        .find_map(|y| {
            let line = row(terminal, y);
            line.find(needle)
                .map(|byte| (line[..byte].chars().count() as u16, y))
        })
        .unwrap_or_else(|| panic!("missing rendered token {needle:?}"));
    let cell = &terminal.backend().buffer()[(x, y)];
    (cell.fg, cell.bg)
}

#[test]
fn schema_json_highlights_keys_and_values_without_a_coloured_background() {
    let mut browser = browser(Some(json!({
        "count": 42,
        "flag": true,
        "label": "value",
        "nothing": null
    })));
    browser.handle_action(BrowserAction::OpenSchema);
    let terminal = draw(&browser, 100, 24);
    let key = color_at(&terminal, "\"count\"");
    let number = color_at(&terminal, "42");
    let boolean = color_at(&terminal, "true");
    let string = color_at(&terminal, "\"value\"");
    let null = color_at(&terminal, "null");
    for (name, style) in [
        ("key", key),
        ("number", number),
        ("boolean", boolean),
        ("string", string),
        ("null", null),
    ] {
        assert_ne!(style.0, Color::Reset, "{name}");
        assert_eq!(style.1, Color::Reset, "{name}");
    }
    assert_eq!(key.0, Color::Cyan);
    assert_eq!(string.0, Color::Green);
    assert_eq!(number.0, Color::Magenta);
    assert_eq!(boolean.0, Color::Yellow);
    assert_eq!(null.0, Color::Yellow);
    assert!(text(&terminal).contains("\"count\": 42"));
}

fn long_schema() -> Value {
    let properties: serde_json::Map<String, Value> = (0..60)
        .map(|index| (format!("field_{index:02}"), json!({"type": "string"})))
        .collect();
    json!({"type": "object", "properties": properties, "zz_final": "schema_end_marker"})
}

fn press(browser: &mut TaskBrowser, code: KeyCode, area: Rect) -> BrowserOutcome {
    browser.handle_event_in_area(&Event::Key(KeyEvent::new(code, KeyModifiers::NONE)), area)
}

#[test]
fn keyboard_inspection_preserves_query_selection_and_independent_scroll_positions() {
    let mut browser = browser(Some(long_schema()));
    let area = Rect::new(0, 0, 100, 12);
    browser.state_mut().set_filter_text("inspect");
    let selected = browser.state().selected_index();
    assert_eq!(
        press(&mut browser, KeyCode::Enter, area),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().active_panel(), BrowserPanel::Details);
    let rendered = text(&draw(&browser, area.width, area.height));
    assert!(rendered.contains("Tab: details/schema"));
    assert!(rendered.contains("Esc: tasks"));
    for (code, offset) in [
        (KeyCode::Down, 1),
        (KeyCode::Char('j'), 2),
        (KeyCode::Up, 1),
        (KeyCode::Char('k'), 0),
        (KeyCode::PageDown, 8),
        (KeyCode::PageUp, 0),
    ] {
        assert_eq!(press(&mut browser, code, area), BrowserOutcome::Changed);
        assert_eq!(
            browser
                .state()
                .inspection_scroll_offset(BrowserPanel::Details),
            Some(offset)
        );
        assert_eq!(browser.state().selected_index(), selected);
    }
    press(&mut browser, KeyCode::PageDown, area);
    assert_eq!(
        press(&mut browser, KeyCode::Tab, area),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().active_panel(), BrowserPanel::Schema);
    assert_eq!(
        press(&mut browser, KeyCode::End, area),
        BrowserOutcome::Changed
    );
    let bottom = browser
        .state()
        .inspection_scroll_offset(BrowserPanel::Schema)
        .unwrap();
    assert!(bottom > 8);
    assert!(text(&draw(&browser, area.width, area.height)).contains("schema_end_marker"));
    assert_eq!(
        press(&mut browser, KeyCode::Down, area),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        press(&mut browser, KeyCode::Home, area),
        BrowserOutcome::Changed
    );
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(0)
    );
    press(&mut browser, KeyCode::End, area);
    press(&mut browser, KeyCode::Tab, area);
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Details),
        Some(8)
    );
    press(&mut browser, KeyCode::Tab, area);
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(bottom)
    );
    assert_eq!(
        press(&mut browser, KeyCode::Char('q'), area),
        BrowserOutcome::QuitRequested
    );
    assert_eq!(
        press(&mut browser, KeyCode::Esc, area),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().active_panel(), BrowserPanel::Tasks);
    assert_eq!(browser.state().filter_text(), "inspect");
    assert_eq!(browser.state().selected_index(), selected);
    assert_eq!(
        press(&mut browser, KeyCode::Esc, area),
        BrowserOutcome::Changed
    );
    assert!(browser.state().filter_text().is_empty());
    assert_eq!(
        press(&mut browser, KeyCode::Esc, area),
        BrowserOutcome::QuitRequested
    );
}

#[test]
fn search_enter_leaves_input_before_another_enter_opens_details() {
    let mut browser = browser(None);
    let area = Rect::new(0, 0, 100, 12);
    press(&mut browser, KeyCode::Char('/'), area);
    for character in "inspect".chars() {
        press(&mut browser, KeyCode::Char(character), area);
    }
    assert_eq!(
        press(&mut browser, KeyCode::Enter, area),
        BrowserOutcome::Changed
    );
    assert_eq!(browser.state().active_panel(), BrowserPanel::Tasks);
    assert!(!browser.state().is_search_active());
    assert_eq!(
        press(&mut browser, KeyCode::Tab, area),
        BrowserOutcome::Ignored
    );
    press(&mut browser, KeyCode::Enter, area);
    press(&mut browser, KeyCode::Tab, area);
    assert_eq!(browser.state().active_panel(), BrowserPanel::Schema);
    assert!(text(&draw(&browser, area.width, area.height)).contains("No input schema available"));
    assert_eq!(
        press(&mut browser, KeyCode::Char('/'), area),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        press(&mut browser, KeyCode::Enter, area),
        BrowserOutcome::Ignored
    );
    assert!(!browser.state().is_search_active());
    press(&mut browser, KeyCode::Esc, area);
    browser.state_mut().set_filter_text("missing");
    assert_eq!(
        press(&mut browser, KeyCode::Enter, area),
        BrowserOutcome::Ignored
    );
    assert_eq!(browser.state().active_panel(), BrowserPanel::Tasks);
    let mut empty = TaskBrowser::new();
    assert_eq!(
        press(&mut empty, KeyCode::Enter, area),
        BrowserOutcome::Ignored
    );
}

#[test]
fn embedded_inspection_events_require_area_for_scrolling_and_leave_host_events_available() {
    let mut browser = browser(Some(long_schema()));
    let area = Rect::new(5, 3, 50, 10);
    assert_eq!(
        browser.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE
        ))),
        BrowserOutcome::Changed
    );
    assert_eq!(
        browser.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE
        ))),
        BrowserOutcome::Ignored
    );
    assert_eq!(browser.state().selected_index(), Some(0));
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Details),
        Some(0)
    );
    assert_eq!(
        press(&mut browser, KeyCode::Down, area),
        BrowserOutcome::Changed
    );
    let ignored = [
        Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::CONTROL)),
        Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::ALT)),
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Down,
            KeyModifiers::NONE,
            KeyEventKind::Repeat,
        )),
        Event::Key(KeyEvent::new_with_kind(
            KeyCode::Tab,
            KeyModifiers::NONE,
            KeyEventKind::Release,
        )),
        Event::Resize(80, 24),
        Event::FocusGained,
        Event::Paste("q".into()),
    ];
    for event in ignored {
        assert_eq!(
            browser.handle_event_in_area(&event, area),
            BrowserOutcome::Ignored
        );
        assert_eq!(browser.state().active_panel(), BrowserPanel::Details);
        assert_eq!(
            browser
                .state()
                .inspection_scroll_offset(BrowserPanel::Details),
            Some(1)
        );
    }
}

#[test]
fn schema_renders_formatted_nested_metadata_without_changing_descriptor() {
    let schema =
        json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]});
    let mut browser = browser(Some(schema.clone()));
    browser.handle_action(BrowserAction::OpenSchema);
    let terminal = draw(&browser, 100, 24);
    let rendered = text(&terminal);
    for expected in [
        "Identity: acme.inspect@1.0.0",
        "\"properties\": {",
        "\"path\": {",
        "\"type\": \"string\"",
        "\"required\": [",
    ] {
        assert!(
            rendered.contains(expected),
            "Missing {expected:?}:\n{rendered}"
        );
    }
    assert!(rendered.contains("│  \"properties\""));
    assert!(!rendered.contains("No input schema available"));
    assert_eq!(
        browser.state().selected_descriptor().unwrap().input_schema,
        Some(schema)
    );
    assert_eq!(browser.state().active_panel(), BrowserPanel::Schema);
}

#[test]
fn absent_schema_is_distinct_from_present_empty_or_boolean_schema() {
    for (schema, expected) in [
        (None, "No input schema available."),
        (Some(json!({})), "{}"),
        (Some(json!(true)), "true"),
        (Some(json!(false)), "false"),
        (Some(Value::Null), "null"),
    ] {
        let absent = schema.is_none();
        let mut browser = browser(schema);
        browser.handle_action(BrowserAction::OpenSchema);
        let rendered = text(&draw(&browser, 100, 16));
        assert!(rendered.contains(expected), "{rendered}");
        assert_eq!(
            rendered.contains("Missing schema metadata does not imply the task accepts no input."),
            absent
        );
        assert_eq!(rendered.contains("No input schema available."), absent);
    }
}

#[test]
fn schema_line_and_page_scrolling_stop_at_both_bounds_and_reach_the_end() {
    let schema = long_schema();
    let total = serde_json::to_string_pretty(&schema)
        .unwrap()
        .lines()
        .count()
        + 2;
    let area = Rect::new(0, 0, 120, 12);
    // Title, footer, and two border rows leave eight content rows.
    let page = 8;
    let maximum = total - page;
    let mut browser = browser(Some(schema));
    browser.handle_action(BrowserAction::OpenSchema);
    for (action, expected, outcome) in [
        (BrowserAction::ScrollUp, 0, BrowserOutcome::Ignored),
        (BrowserAction::ScrollDown, 1, BrowserOutcome::Changed),
        (BrowserAction::PageDown, 1 + page, BrowserOutcome::Changed),
        (BrowserAction::PageUp, 1, BrowserOutcome::Changed),
        (
            BrowserAction::ScrollToBottom,
            maximum,
            BrowserOutcome::Changed,
        ),
        (BrowserAction::ScrollDown, maximum, BrowserOutcome::Ignored),
        (BrowserAction::PageDown, maximum, BrowserOutcome::Ignored),
    ] {
        assert_eq!(browser.handle_action_in_area(action, area), outcome);
        assert_eq!(
            browser
                .state()
                .inspection_scroll_offset(BrowserPanel::Schema),
            Some(expected)
        );
    }
    let terminal = draw(&browser, area.width, area.height);
    assert!(text(&terminal).contains("schema_end_marker"));
    assert!(row(&terminal, 11).contains(&format!("Lines {}-{total} of {total}", maximum + 1)));
    assert!(row(&terminal, 9).contains('}'));
    assert_eq!(
        browser.handle_action_in_area(BrowserAction::ScrollToTop, area),
        BrowserOutcome::Changed
    );
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(0)
    );
    assert!(text(&draw(&browser, 120, 12)).contains("Identity: acme.inspect@1.0.0"));
}

#[test]
fn detail_scrolling_accounts_for_wrapping_and_keeps_view_offsets_independent() {
    let mut browser = browser(Some(long_schema()));
    let area = Rect::new(0, 0, 50, 12);
    browser.handle_action(BrowserAction::OpenDetails);
    browser.handle_action_in_area(BrowserAction::PageDown, area);
    let details_offset = browser
        .state()
        .inspection_scroll_offset(BrowserPanel::Details)
        .unwrap();
    assert_eq!(details_offset, 8);
    browser.handle_action(BrowserAction::ToggleInspectionView);
    browser.handle_action_in_area(BrowserAction::ScrollDown, area);
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(1)
    );
    browser.handle_action(BrowserAction::ToggleInspectionView);
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Details),
        Some(details_offset)
    );
    browser.handle_action_in_area(BrowserAction::ScrollToBottom, area);
    let maximum = browser
        .state()
        .inspection_scroll_offset(BrowserPanel::Details)
        .unwrap();
    assert!(maximum > 40, "Description wrapping must add display rows");
    let rendered = text(&draw(&browser, 50, 12));
    assert!(rendered.contains("defaults apply)"), "{rendered}");
    assert_eq!(
        browser.handle_action_in_area(BrowserAction::ScrollDown, area),
        BrowserOutcome::Ignored
    );
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Details),
        Some(maximum)
    );
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(1)
    );
    assert_eq!(browser.state().selected_index(), Some(0));
}

#[test]
fn resize_clamps_effective_scroll_without_mutating_state_and_actions_use_new_bounds() {
    let mut browser = browser(Some(long_schema()));
    browser.handle_action(BrowserAction::OpenSchema);
    let small = Rect::new(0, 0, 100, 10);
    browser.handle_action_in_area(BrowserAction::ScrollToBottom, small);
    let requested = browser
        .state()
        .inspection_scroll_offset(BrowserPanel::Schema)
        .unwrap();
    let terminal = draw(&browser, 120, 300);
    assert!(text(&terminal).contains("Identity: acme.inspect@1.0.0"));
    assert!(text(&terminal).contains("schema_end_marker"));
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(requested)
    );
    assert_eq!(
        browser.handle_action_in_area(BrowserAction::ScrollDown, Rect::new(0, 0, 120, 300)),
        BrowserOutcome::Changed
    );
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(0)
    );
    browser
        .state_mut()
        .set_inspection_scroll_offset(BrowserPanel::Schema, usize::MAX);
    let terminal = draw(&browser, small.width, small.height);
    assert!(text(&terminal).contains("schema_end_marker"));
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(usize::MAX)
    );
    browser.handle_action_in_area(BrowserAction::ScrollUp, small);
    assert_eq!(
        browser
            .state()
            .inspection_scroll_offset(BrowserPanel::Schema),
        Some(requested - 1)
    );
}

#[test]
fn scroll_actions_require_inspection_and_usable_area_and_other_actions_delegate() {
    let actions = [
        BrowserAction::ScrollUp,
        BrowserAction::ScrollDown,
        BrowserAction::PageUp,
        BrowserAction::PageDown,
        BrowserAction::ScrollToTop,
        BrowserAction::ScrollToBottom,
    ];
    let mut browser = browser(None);
    let area = Rect::new(0, 0, 100, 20);
    for action in actions {
        assert_eq!(
            browser.handle_action_in_area(action, area),
            BrowserOutcome::Ignored
        );
    }
    assert_eq!(
        browser.handle_action_in_area(BrowserAction::OpenSchema, area),
        BrowserOutcome::Changed
    );
    assert_eq!(
        browser.handle_action_in_area(BrowserAction::ScrollDown, area),
        BrowserOutcome::Ignored
    );
    browser
        .state_mut()
        .set_inspection_scroll_offset(BrowserPanel::Schema, 7);
    for action in actions {
        assert_eq!(browser.handle_action(action), BrowserOutcome::Ignored);
        for empty in [
            Rect::default(),
            Rect::new(0, 0, 0, 20),
            Rect::new(0, 0, 20, 0),
            Rect::new(0, 0, 20, 1),
        ] {
            assert_eq!(
                browser.handle_action_in_area(action, empty),
                BrowserOutcome::Ignored
            );
        }
        assert_eq!(
            browser
                .state()
                .inspection_scroll_offset(BrowserPanel::Schema),
            Some(7)
        );
    }
    assert_eq!(
        browser.handle_action_in_area(BrowserAction::Quit, area),
        BrowserOutcome::QuitRequested
    );
    browser.state_mut().select(None);
    for action in actions {
        assert_eq!(
            browser.handle_action_in_area(action, area),
            BrowserOutcome::Ignored
        );
    }
}

#[test]
fn narrow_schema_text_remains_accessible_and_tiny_rendering_preserves_state() {
    let mut browser = browser(Some(
        json!({"description": "A long schema description with Unicode é東京 and spaces that wraps across many rows.", "zz_end": "reachable_end"}),
    ));
    browser.handle_action(BrowserAction::OpenSchema);
    let area = Rect::new(0, 0, 24, 8);
    let mut seen = String::new();
    loop {
        seen.push_str(&text(&draw(&browser, area.width, area.height)));
        if browser.handle_action_in_area(BrowserAction::ScrollDown, area) == BrowserOutcome::Ignored
        {
            break;
        }
    }
    assert!(seen.contains('東') && seen.contains('京'));
    assert!(seen.contains("reachable_end"), "{seen}");
    let requested = browser
        .state()
        .inspection_scroll_offset(BrowserPanel::Schema);
    for (width, height) in [(0, 0), (1, 1), (2, 2), (8, 3), (18, 6)] {
        let _ = draw(&browser, width, height);
        assert_eq!(
            browser
                .state()
                .inspection_scroll_offset(BrowserPanel::Schema),
            requested
        );
        assert_eq!(browser.state().active_panel(), BrowserPanel::Schema);
    }
}
