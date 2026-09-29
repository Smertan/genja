//! Shared inspection content and viewport measurements for drawing and scrolling.
//!
//! Ratatui's optional rendered-line measurement uses its own word wrapping,
//! keeping scroll bounds consistent with styled and Unicode paragraph output.
//! No measurements are retained in browser state or require a live terminal.

use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::{BrowserPanel, TaskBrowserState, detail::detail_content};
use crate::discovery::TaskDescriptor;

/// Measured content and inner area shared by inspection drawing and actions.
pub(super) struct InspectionViewport<'a> {
    pub(super) paragraph: Paragraph<'a>,
    pub(super) block: Block<'static>,
    pub(super) body: Rect,
    pub(super) inner: Rect,
    pub(super) line_count: usize,
}

impl InspectionViewport<'_> {
    /// Bound scrolling by content height and Ratatui's addressable row range.
    /// Paragraph scroll offsets address up to `u16::MAX` wrapped display rows.
    pub(super) fn max_offset(&self) -> usize {
        if self.inner.width == 0 || self.inner.height == 0 {
            return 0;
        }
        self.line_count
            .saturating_sub(usize::from(self.inner.height))
            .min(usize::from(u16::MAX))
    }

    /// Clamp a requested offset to the viewport without updating browser state.
    pub(super) fn effective_offset(&self, requested: usize) -> usize {
        requested.min(self.max_offset())
    }
}

/// Calculate title, inspection body, and footer areas from the host's browser area.
pub(super) fn inspection_rows(area: Rect) -> [Rect; 3] {
    Layout::vertical([
        Constraint::Length(u16::from(area.height >= 4)),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(area)
}

/// Measure the selected descriptor using exactly the paragraph drawn on screen.
/// Borders are measured separately so wrapping receives the actual inner width.
pub(super) fn inspection_viewport(
    state: &TaskBrowserState,
    area: Rect,
) -> Option<InspectionViewport<'_>> {
    let task = state.selected_descriptor()?;
    let (title, text) = match state.active_panel() {
        BrowserPanel::Tasks => return None,
        BrowserPanel::Details => ("Details", detail_content(task)),
        BrowserPanel::Schema => ("Schema", schema_content(task)),
    };
    let body = inspection_rows(area)[1];
    let block = if body.height >= 3 && body.width >= 4 {
        Block::default().title(title).borders(Borders::ALL)
    } else {
        Block::default()
    };
    let inner = block.inner(body);
    let paragraph = Paragraph::new(text).wrap(Wrap { trim: false });
    let line_count = paragraph.line_count(inner.width);
    Some(InspectionViewport {
        paragraph,
        block,
        body,
        inner,
        line_count,
    })
}

/// Format available JSON schema metadata without interpreting it as an input form.
/// Missing metadata is distinct from a present empty object, boolean, or null.
fn schema_content(task: &TaskDescriptor) -> Text<'static> {
    let mut text = Text::from(vec![
        Line::raw(format!("Identity: {}@{}", task.id, task.version)),
        Line::default(),
    ]);
    let schema = match task.input_schema.as_ref() {
        Some(schema) => match serde_json::to_string_pretty(schema) {
            Ok(json) => Text::from(json),
            Err(error) => Text::from(format!("Unable to format input schema: {error}")),
        },
        None => Text::from(
            "No input schema available.\n\nMissing schema metadata does not imply the task accepts no input.",
        ),
    };
    text.lines.extend(schema.lines);
    text
}
