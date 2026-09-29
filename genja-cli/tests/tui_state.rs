#![cfg(feature = "tui")]

use std::cell::Cell;

use genja_cli::discovery::{DiscoveryError, DiscoveryResult, TaskDescriptor, TaskDescriptorSource};
use genja_cli::tui::{BrowserAction, BrowserOutcome, BrowserPanel, TaskBrowser};
use genja_core::task::{TaskDescriptorMetadata, TaskExecutionMode};

struct FakeSource {
    result: DiscoveryResult<Vec<TaskDescriptor>>,
    calls: Cell<usize>,
}

impl FakeSource {
    fn new(result: DiscoveryResult<Vec<TaskDescriptor>>) -> Self {
        Self {
            result,
            calls: Cell::new(0),
        }
    }
}

impl TaskDescriptorSource for FakeSource {
    fn list_tasks(&self) -> DiscoveryResult<Vec<TaskDescriptor>> {
        self.calls.set(self.calls.get() + 1);
        self.result.clone()
    }
}

fn descriptor(id: &str, version: &str) -> TaskDescriptor {
    TaskDescriptor::explicit(
        id,
        version,
        TaskDescriptorMetadata {
            name: id.to_string(),
            description: Some("Browser test task".to_string()),
            execution_mode: TaskExecutionMode::Blocking,
            connection_plugin_name: None,
            processor_names: Vec::new(),
            retry: None,
        },
        None,
        false,
    )
}

#[test]
fn initial_state_and_empty_load_have_no_selection() {
    let mut browser = TaskBrowser::new();
    assert!(browser.state().descriptors().is_empty());
    assert_eq!(browser.state().selected_index(), None);
    assert!(browser.state().selected_descriptor().is_none());
    assert_eq!(browser.state().filter_text(), "");
    assert_eq!(browser.state().active_panel(), BrowserPanel::Tasks);
    assert!(browser.state().error().is_none());
    assert!(!browser.state_mut().select(Some(0)));

    browser.load_from(&FakeSource::new(Ok(Vec::new()))).unwrap();
    assert_eq!(browser.state().selected_index(), None);
    assert!(browser.state().error().is_none());
}

#[test]
fn trait_object_load_owns_snapshot_and_preserves_source_order() {
    let descriptors = vec![descriptor("z.task", "2.0.0"), descriptor("a.task", "1.0.0")];
    let mut browser = TaskBrowser::new();
    {
        let source = FakeSource::new(Ok(descriptors.clone()));
        let erased: &dyn TaskDescriptorSource = &source;
        browser.load_from(erased).unwrap();
        assert_eq!(source.calls.get(), 1);
        assert_eq!(browser.state().selected_descriptor(), Some(&descriptors[0]));
        assert_eq!(source.calls.get(), 1);
    }
    assert_eq!(browser.state().descriptors(), descriptors);
    assert_eq!(browser.state().selected_index(), Some(0));
}

#[test]
fn selection_rejects_invalid_indices_and_can_be_cleared() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&FakeSource::new(Ok(vec![
            descriptor("a.task", "1.0.0"),
            descriptor("a.task", "2.0.0"),
        ])))
        .unwrap();
    assert!(browser.state_mut().select(Some(1)));
    assert_eq!(
        browser.state().selected_descriptor().unwrap().version,
        "2.0.0"
    );
    for invalid in [2, usize::MAX] {
        assert!(!browser.state_mut().select(Some(invalid)));
        assert_eq!(browser.state().selected_index(), Some(1));
    }
    assert!(browser.state_mut().select(None));
    assert!(browser.state().selected_descriptor().is_none());
}

#[test]
fn replacing_snapshot_reapplies_filter_and_preserves_panel() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&FakeSource::new(Ok(vec![
            descriptor("a.task", "1.0.0"),
            descriptor("b.task", "1.0.0"),
        ])))
        .unwrap();
    browser.state_mut().select(Some(1));
    browser
        .state_mut()
        .set_filter_text("does not match any task");
    browser.state_mut().set_active_panel(BrowserPanel::Details);
    assert_eq!(browser.state().descriptors().len(), 2);
    assert_eq!(browser.state().selected_index(), None);

    browser
        .load_from(&FakeSource::new(Ok(vec![descriptor("c.task", "1.0.0")])))
        .unwrap();
    assert_eq!(browser.state().selected_index(), None);
    assert!(browser.state().matching_indices().is_empty());
    browser.load_from(&FakeSource::new(Ok(Vec::new()))).unwrap();
    assert!(browser.state().descriptors().is_empty());
    assert_eq!(browser.state().selected_index(), None);
    assert_eq!(browser.state().filter_text(), "does not match any task");
    assert_eq!(browser.state().active_panel(), BrowserPanel::Details);
}

#[test]
fn failure_clears_snapshot_returns_error_and_preserves_filter_and_panel() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&FakeSource::new(Ok(vec![descriptor("a.task", "1.0.0")])))
        .unwrap();
    browser.state_mut().set_filter_text("query");
    browser.state_mut().set_active_panel(BrowserPanel::Details);
    let error = DiscoveryError::source_failed("registry unavailable");
    let source = FakeSource::new(Err(error.clone()));

    assert_eq!(browser.load_from(&source), Err(error.clone()));
    assert_eq!(source.calls.get(), 1);
    assert_eq!(browser.state().error(), Some(&error));
    assert!(browser.state().descriptors().is_empty());
    assert!(browser.state().matching_indices().is_empty());
    assert_eq!(browser.state().selected_index(), None);
    assert!(browser.state().selected_descriptor().is_none());
    assert_eq!(browser.state().filter_text(), "query");
    assert_eq!(browser.state().active_panel(), BrowserPanel::Details);
}

#[test]
fn filter_matches_each_field_case_insensitively_in_source_order() {
    let mut first = descriptor("acme.backup", "2.7.3");
    first.name = "Save Config".into();
    first.description = Some("Safely preserve settings".into());
    let mut second = descriptor("acme.facts", "1.0.0");
    second.name = "Collect Facts".into();
    second.description = None;
    second.execution_mode = TaskExecutionMode::Async;
    let tasks = vec![first, second];
    let mut browser = TaskBrowser::new();
    let source = FakeSource::new(Ok(tasks.clone()));
    browser.load_from(&source).unwrap();
    for (query, expected) in [
        ("BACKUP", vec![0]),
        ("sAvE cOnFiG", vec![0]),
        ("2.7", vec![0]),
        ("PRESERVE", vec![0]),
        ("BLOCKING", vec![0]),
        ("ASYNC", vec![1]),
        ("acme", vec![0, 1]),
        ("missing", vec![]),
        ("", vec![0, 1]),
        ("  \t ", vec![0, 1]),
        ("  backup  ", vec![0]),
    ] {
        browser.state_mut().set_filter_text(query);
        assert_eq!(browser.state().filter_text(), query);
        assert_eq!(browser.state().matching_indices(), expected, "{query}");
        let matches: Vec<_> = browser.state().filtered_descriptors().collect();
        assert_eq!(
            matches,
            expected
                .iter()
                .map(|&index| &tasks[index])
                .collect::<Vec<_>>()
        );
        assert_eq!(browser.state().descriptors(), tasks);
    }
    assert_eq!(source.calls.get(), 1);
}

#[test]
fn filter_preserves_selected_task_and_clamps_previous_visible_position() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&FakeSource::new(Ok(vec![
            descriptor("keep.a", "1.0.0"),
            descriptor("drop.b", "1.0.0"),
            descriptor("keep.c", "1.0.0"),
            descriptor("keep.d", "1.0.0"),
        ])))
        .unwrap();
    browser.state_mut().select(Some(2));
    browser.state_mut().set_filter_text("keep");
    assert_eq!(browser.state().selected_index(), Some(2));
    assert_eq!(browser.state().selected_visible_index(), Some(1));
    assert!(!browser.state_mut().select(Some(1)));
    browser.state_mut().set_filter_text("keep.a");
    assert_eq!(browser.state().selected_index(), Some(0));
    browser.state_mut().set_filter_text("");
    assert_eq!(browser.state().matching_indices(), [0, 1, 2, 3]);
    assert_eq!(browser.state().selected_index(), Some(0));
    browser.state_mut().select(Some(1));
    browser.state_mut().set_filter_text("keep");
    assert_eq!(browser.state().selected_index(), Some(2));
    browser.state_mut().set_filter_text("missing");
    assert_eq!(browser.state().selected_index(), None);
    browser.state_mut().set_filter_text("");
    assert_eq!(browser.state().selected_index(), Some(0));
}

#[test]
fn navigation_uses_filtered_positions_and_stops_at_bounds() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&FakeSource::new(Ok(vec![
            descriptor("hidden", "1.0.0"),
            descriptor("keep.a", "1.0.0"),
            descriptor("hidden.again", "1.0.0"),
            descriptor("keep.b", "1.0.0"),
        ])))
        .unwrap();
    browser.state_mut().set_filter_text("keep");
    for (action, index, outcome) in [
        (BrowserAction::SelectPrevious, 1, BrowserOutcome::Ignored),
        (BrowserAction::SelectNext, 3, BrowserOutcome::Changed),
        (BrowserAction::SelectNext, 3, BrowserOutcome::Ignored),
        (BrowserAction::SelectFirst, 1, BrowserOutcome::Changed),
        (BrowserAction::SelectLast, 3, BrowserOutcome::Changed),
        (BrowserAction::SelectPrevious, 1, BrowserOutcome::Changed),
    ] {
        assert_eq!(browser.handle_action(action), outcome);
        assert_eq!(browser.state().selected_index(), Some(index));
    }
    browser.state_mut().select(None);
    browser.handle_action(BrowserAction::SelectNext);
    assert_eq!(browser.state().selected_index(), Some(1));
    browser.state_mut().set_filter_text("missing");
    for action in [
        BrowserAction::SelectNext,
        BrowserAction::SelectPrevious,
        BrowserAction::SelectFirst,
        BrowserAction::SelectLast,
    ] {
        assert_eq!(browser.handle_action(action), BrowserOutcome::Ignored);
        assert_eq!(browser.state().selected_index(), None);
    }
}

#[test]
fn loading_with_a_query_selects_first_match_and_recovers_after_failure() {
    let mut browser = TaskBrowser::new();
    browser.state_mut().set_filter_text("keep");
    let source = FakeSource::new(Ok(vec![
        descriptor("drop", "1.0.0"),
        descriptor("keep", "1.0.0"),
    ]));
    browser.load_from(&source).unwrap();
    assert_eq!(browser.state().selected_index(), Some(1));
    browser
        .load_from(&FakeSource::new(Err(DiscoveryError::source_failed(
            "failed",
        ))))
        .unwrap_err();
    assert!(browser.state().matching_indices().is_empty());
    browser.load_from(&source).unwrap();
    assert_eq!(browser.state().selected_index(), Some(1));
    assert_eq!(browser.state().filter_text(), "keep");
    assert!(browser.state().error().is_none());
}

#[test]
fn filter_handles_unicode_substrings_and_is_idempotent() {
    let mut browser = TaskBrowser::new();
    browser
        .load_from(&FakeSource::new(Ok(vec![descriptor(
            "ÉTAPE.東京",
            "1.0.0",
        )])))
        .unwrap();
    for query in ["étape", "東京"] {
        browser.state_mut().set_filter_text(query);
        assert_eq!(browser.state().matching_indices(), [0]);
    }
    browser.state_mut().select(None);
    browser.state_mut().set_filter_text("東京");
    assert_eq!(browser.state().selected_index(), None);
}

#[test]
fn successful_load_clears_initial_error_for_empty_and_populated_sources() {
    for descriptors in [Vec::new(), vec![descriptor("a.task", "1.0.0")]] {
        let mut browser = TaskBrowser::new();
        let error = DiscoveryError::source_failed("unavailable");
        assert_eq!(
            browser.load_from(&FakeSource::new(Err(error.clone()))),
            Err(error)
        );
        assert!(browser.state().error().is_some());
        assert_eq!(browser.state().selected_index(), None);
        browser
            .load_from(&FakeSource::new(Ok(descriptors.clone())))
            .unwrap();
        assert!(browser.state().error().is_none());
        assert_eq!(browser.state().descriptors(), descriptors);
        assert_eq!(
            browser.state().selected_index(),
            (!descriptors.is_empty()).then_some(0)
        );
    }
}
