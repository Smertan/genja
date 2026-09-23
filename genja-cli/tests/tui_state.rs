#![cfg(feature = "tui")]

use std::cell::Cell;

use genja_cli::discovery::{DiscoveryError, DiscoveryResult, TaskDescriptor, TaskDescriptorSource};
use genja_cli::tui::{BrowserPanel, TaskBrowser};
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
fn replacing_snapshot_resets_selection_and_preserves_reserved_state() {
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
    assert_eq!(browser.state().selected_index(), Some(1));

    browser
        .load_from(&FakeSource::new(Ok(vec![descriptor("c.task", "1.0.0")])))
        .unwrap();
    assert_eq!(browser.state().selected_index(), Some(0));
    assert_eq!(browser.state().selected_descriptor().unwrap().id, "c.task");
    browser.load_from(&FakeSource::new(Ok(Vec::new()))).unwrap();
    assert!(browser.state().descriptors().is_empty());
    assert_eq!(browser.state().selected_index(), None);
    assert_eq!(browser.state().filter_text(), "does not match any task");
    assert_eq!(browser.state().active_panel(), BrowserPanel::Details);
}

#[test]
fn failure_clears_snapshot_returns_error_and_preserves_reserved_state() {
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
    assert_eq!(browser.state().selected_index(), None);
    assert!(browser.state().selected_descriptor().is_none());
    assert_eq!(browser.state().filter_text(), "query");
    assert_eq!(browser.state().active_panel(), BrowserPanel::Details);
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
