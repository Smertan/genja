//! Task browser layout and rendering.
//!
//! The shell reserves navigation, inspection, and status space. It renders an
//! owned snapshot without discovery or terminal lifecycle changes. Task rows
//! and descriptor detail widgets belong to later browser work.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    widgets::{Block, Borders, Paragraph},
};

use super::TaskBrowserState;

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
        if rows[1].width >= 48 {
            let panels =
                Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
                    .split(rows[1]);
            frame.render_widget(
                Paragraph::new("Task list coming soon")
                    .block(Block::default().title("Tasks").borders(Borders::ALL)),
                panels[0],
            );
            frame.render_widget(
                Paragraph::new("Descriptor details coming soon")
                    .block(Block::default().title("Details").borders(Borders::ALL)),
                panels[1],
            );
        } else {
            frame.render_widget(Paragraph::new("Task browser foundation"), rows[1]);
        }
    }

    if rows[2].height > 0 {
        let status = state.error().map_or_else(
            || "q / Esc: quit".to_string(),
            |error| format!("Discovery error: {error}"),
        );
        frame.render_widget(Paragraph::new(status), rows[2]);
    }
}
