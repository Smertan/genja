//! Task browser layout and rendering.
//!
//! The browser reserves title, task table, and status space. It renders an
//! owned snapshot without discovery or terminal lifecycle changes.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
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
        if let Some(error) = state.error() {
            render_message(
                frame,
                rows[1],
                "Discovery error",
                format!(
                    "Unable to load task descriptors.\n\n{error}\n\nFor compiled Rust tasks, check discovery with your project's CLI:\nmy_project_cli task list\nReplace my_project_cli with your binary name."
                ),
            );
        } else if state.descriptors().is_empty() {
            render_message(frame, rows[1], "Tasks", empty_message());
        } else {
            render_tasks(state, frame, rows[1]);
        }
    }

    if rows[2].height > 0 {
        let status = if state.error().is_some() || state.descriptors().is_empty() {
            if rows[2].width >= 13 {
                "q / Esc: quit"
            } else {
                "q/Esc: quit"
            }
        } else if rows[2].width >= 56 {
            "Up/k Down/j: move | Home/End: first/last | q / Esc: quit"
        } else if rows[2].width >= 25 {
            "j/k: move | q / Esc: quit"
        } else {
            "q/Esc: quit"
        };
        frame.render_widget(Paragraph::new(status), rows[2]);
    }
}

fn empty_message() -> Text<'static> {
    let heading = Style::default().add_modifier(Modifier::BOLD);
    let code = Style::default().fg(Color::Cyan).bg(Color::DarkGray);
    let code_line = |text| Line::from(vec![Span::raw("  "), Span::styled(text, code)]);
    Text::from(vec![
        Line::from("No registered tasks available."),
        Line::default(),
        Line::styled(
            "In your task crate, annotate an existing task implementation:",
            heading,
        ),
        code_line("#[genja_task(name = \"my_task\")]"),
        code_line("impl MyTask { /* start or start_async method */ }"),
        Line::default(),
        Line::styled(
            "In your project-local CLI/TUI binary, link your task crate:",
            heading,
        ),
        code_line("use my_project_tasks as _;"),
        Line::default(),
        Line::from("Replace MyTask and my_project_tasks with your task and crate."),
        Line::from(
            "The annotation enables discovery; registration(...) adds a stable ID and factory.",
        ),
    ])
}

fn render_message<'a>(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    message: impl Into<Text<'a>>,
) {
    let block = if area.height >= 3 && area.width >= 4 {
        Block::default().title(title).borders(Borders::ALL)
    } else {
        Block::default()
    };
    frame.render_widget(
        Paragraph::new(message)
            .wrap(Wrap { trim: false })
            .block(block),
        area,
    );
}
