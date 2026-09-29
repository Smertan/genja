#![cfg(feature = "tui")]

use std::cell::Cell;

use genja_cli::discovery::{DiscoveryResult, TaskDescriptor, TaskDescriptorSource};
use genja_cli::tui::{BrowserAction, BrowserPanel, TaskBrowser};
use genja_core::task::{RetryConfig, TaskDescriptorMetadata, TaskExecutionMode};
use ratatui::{Terminal, backend::TestBackend, layout::Rect};

struct Source {
    tasks: Vec<TaskDescriptor>,
    calls: Cell<usize>,
}

impl Source {
    fn new(tasks: Vec<TaskDescriptor>) -> Self {
        Self {
            tasks,
            calls: Cell::new(0),
        }
    }
}

impl TaskDescriptorSource for Source {
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        self.calls.set(self.calls.get() + 1);
        Ok(self.tasks.clone())
    }
}

fn metadata() -> TaskDescriptorMetadata {
    TaskDescriptorMetadata {
        name: "backup_config".into(),
        description: Some("Save the configuration.\n\nUses the configured destination.".into()),
        execution_mode: TaskExecutionMode::Blocking,
        connection_plugin_name: None,
        processor_names: Vec::new(),
        retry: None,
    }
}

fn backup() -> TaskDescriptor {
    TaskDescriptor::explicit(
        "acme.backup",
        "1.2.3",
        metadata(),
        Some(serde_json::json!({"type": "object"})),
        true,
    )
}

fn screen(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render(browser: &TaskBrowser) -> String {
    let mut terminal = Terminal::new(TestBackend::new(140, 32)).unwrap();
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();
    screen(&terminal)
}

#[test]
fn details_render_core_metadata_from_selected_discovery_descriptor() {
    let source = Source::new(vec![backup()]);
    let mut browser = TaskBrowser::new();
    let erased: &dyn TaskDescriptorSource = &source;
    browser.load_from(erased).unwrap();
    browser.handle_action(BrowserAction::OpenDetails);
    for _ in 0..2 {
        let text = render(&browser);
        for expected in [
            "Identity: acme.backup@1.2.3",
            "Task ID: acme.backup",
            "ID source: explicit",
            "Version: 1.2.3",
            "Name: backup_config",
            "Execution mode: blocking",
            "Constructible: yes (registered JSON input factory)",
            "Input schema: Available",
            "Save the configuration.",
            "Uses the configured destination.",
            "Esc: tasks",
            "q: quit",
        ] {
            assert!(text.contains(expected), "Missing {expected:?}:\n{text}");
        }
        assert!(!text.contains("Search:"));
        assert!(!text.contains("tasks shown"));
    }
    assert_eq!(browser.state().descriptors(), source.tasks);
    assert_eq!(browser.state().active_panel(), BrowserPanel::Details);
    assert_eq!(source.calls.get(), 1);
}

#[test]
fn generated_details_explain_missing_optional_metadata_and_factory() {
    for description in [None, Some(" \n\t ".into())] {
        let mut meta = metadata();
        meta.name = "collect_facts".into();
        meta.execution_mode = TaskExecutionMode::Async;
        meta.description = description;
        let task = TaskDescriptor::generated("auto:examples::CollectFacts", "0.4.0", meta);
        let mut browser = TaskBrowser::new();
        browser.load_from(&Source::new(vec![task])).unwrap();
        browser.handle_action(BrowserAction::OpenDetails);
        let text = render(&browser);
        for expected in [
            "Identity: auto:examples::CollectFacts@0.4.0",
            "ID source: generated",
            "Execution mode: async",
            "Constructible: no (no registered JSON input factory",
            "direct construction may still be possible",
            "No description available.",
            "Input schema: Not provided",
            "Connection plugin: Not specified",
            "Processors: None specified",
            "Retry: Not configured (runner or built-in defaults apply)",
        ] {
            assert!(text.contains(expected), "Missing {expected:?}:\n{text}");
        }
    }
}

#[test]
fn details_show_explicit_retry_overrides_without_resolving_missing_values() {
    for (retry, allowed, attempts, delay) in [
        (
            RetryConfig::new(Some(false), None, Some(250)),
            "false",
            "Not specified (runner/default policy)",
            "250 ms",
        ),
        (
            RetryConfig::builder()
                .allow(true)
                .max_attempts(3)
                .delay_ms(0)
                .build(),
            "true",
            "3",
            "0 ms",
        ),
    ] {
        let mut task = backup();
        task.connection_plugin_name = Some("ssh".into());
        task.processor_names = vec!["audit".into(), "notify".into()];
        task.retry = Some(retry);
        let mut browser = TaskBrowser::new();
        browser.load_from(&Source::new(vec![task])).unwrap();
        browser.handle_action(BrowserAction::OpenDetails);
        let text = render(&browser);
        for expected in [
            "Connection plugin: ssh".to_string(),
            "Processors: audit, notify".into(),
            format!("Retry allowed: {allowed}"),
            format!("Retry max attempts (including first): {attempts}"),
            format!("Retry delay: {delay}"),
        ] {
            assert!(text.contains(&expected), "Missing {expected:?}:\n{text}");
        }
    }
}

#[test]
fn details_follow_filtered_selection_and_return_to_unchanged_list() {
    let mut meta = metadata();
    meta.name = "collect_facts".into();
    meta.description = None;
    let source = Source::new(vec![
        backup(),
        TaskDescriptor::generated("auto:examples::CollectFacts", "0.4.0", meta),
    ]);
    let mut browser = TaskBrowser::new();
    browser.load_from(&source).unwrap();
    browser.state_mut().set_filter_text("collect");
    browser.handle_action(BrowserAction::OpenDetails);
    let text = render(&browser);
    assert!(text.contains("Identity: auto:examples::CollectFacts@0.4.0"));
    assert!(!text.contains("acme.backup"));
    assert_eq!(browser.state().selected_index(), Some(1));
    browser.handle_action(BrowserAction::ReturnToTasks);
    let text = render(&browser);
    assert!(text.contains("Search: collect"));
    assert!(text.contains("1 task shown, 2 total"));
    assert_eq!(browser.state().filter_text(), "collect");
    assert_eq!(browser.state().selected_index(), Some(1));
    assert_eq!(source.calls.get(), 1);
}

#[test]
fn details_wrap_descriptions_and_render_safely_in_small_embedded_areas() {
    let mut task = backup();
    task.description = Some("first paragraph\n\nsecond paragraph 東京 é🌱\nA long description continues across several narrow terminal lines without editing the descriptor.".into());
    let source = Source::new(vec![task]);
    let mut browser = TaskBrowser::new();
    browser.load_from(&source).unwrap();
    browser.handle_action(BrowserAction::OpenDetails);
    let mut terminal = Terminal::new(TestBackend::new(55, 40)).unwrap();
    terminal
        .draw(|frame| browser.render(frame, frame.area()))
        .unwrap();
    let text = screen(&terminal);
    for expected in ["first paragraph", "second paragraph", "descriptor.", "é🌱"] {
        assert!(text.contains(expected), "Missing {expected:?}:\n{text}");
    }
    assert!(text.contains('東') && text.contains('京'));
    browser
        .state_mut()
        .set_inspection_scroll_offset(BrowserPanel::Details, 7);
    for (width, height) in [(0, 0), (1, 1), (2, 2), (8, 3), (20, 6), (40, 10)] {
        terminal.backend_mut().resize(width, height);
        terminal
            .draw(|frame| browser.render(frame, frame.area()))
            .unwrap();
        assert_eq!(browser.state().active_panel(), BrowserPanel::Details);
        assert_eq!(browser.state().selected_index(), Some(0));
        assert_eq!(
            browser
                .state()
                .inspection_scroll_offset(BrowserPanel::Details),
            Some(7)
        );
        assert_eq!(browser.state().descriptors(), source.tasks);
    }
    terminal.backend_mut().resize(40, 12);
    let area = Rect::new(2, 2, 20, 6);
    terminal.draw(|frame| browser.render(frame, area)).unwrap();
    let buffer = terminal.backend().buffer();
    for y in 0..12 {
        for x in 0..40 {
            if x < area.x || x >= area.right() || y < area.y || y >= area.bottom() {
                assert_eq!(buffer[(x, y)].symbol(), " ");
            }
        }
    }
    assert_eq!(source.calls.get(), 1);
}
