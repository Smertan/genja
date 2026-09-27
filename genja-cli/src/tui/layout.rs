//! Task browser layout and rendering.
//!
//! The browser reserves title, task table, and status space. It renders an
//! owned snapshot without discovery or terminal lifecycle changes.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::Paragraph,
};

use super::TaskBrowserState;
use super::widgets::render_tasks;

pub(super) fn render_browser(state: &TaskBrowserState, frame: &mut Frame<'_>, area: Rect) {
    let area = area.intersection(frame.area());
    if area.width == 0 || area.height == 0 {
        return;
    }

    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(format!("Genja tasks ({})", state.descriptors().len())),
        rows[0],
    );

    if rows[1].height > 0 {
        render_tasks(state, frame, rows[1]);
    }

    if rows[2].height > 0 {
        let status = state.error().map_or_else(
            || {
                if rows[2].width >= 56 {
                    "Up/k Down/j: move | Home/End: first/last | q / Esc: quit"
                } else if rows[2].width >= 25 {
                    "j/k: move | q / Esc: quit"
                } else {
                    "q/Esc: quit"
                }
                .to_string()
            },
            |error| format!("Discovery error: {error}"),
        );
        frame.render_widget(Paragraph::new(status), rows[2]);
    }
}
