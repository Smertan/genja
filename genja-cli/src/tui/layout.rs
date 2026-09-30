//! Task browser layout and rendering.
//!
//! The browser reserves search, filtered task table, result count, and control
//! space. Short areas omit the title and count to prioritize input and tasks.
//! Inspection uses a wrapped descriptor view with scroll and return controls.
//! Rendering never changes state, performs discovery, or owns the terminal.

use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::inspection::{inspection_rows, inspection_viewport};
use super::widgets::render_tasks;
use super::{BrowserPanel, TaskBrowserState};

/// Draw a browser inside the clipped host area without changing its state.
pub(super) fn render_browser(state: &TaskBrowserState, frame: &mut Frame<'_>, area: Rect) {
    let area = area.intersection(frame.area());
    if area.width == 0 || area.height == 0 {
        return;
    }
    if state.active_panel() != BrowserPanel::Tasks {
        render_inspection(state, frame, area);
        return;
    }

    let extra_rows = u16::from(area.height >= 8);
    let rows = Layout::vertical([
        Constraint::Length(extra_rows),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(extra_rows),
        Constraint::Length(1),
    ])
    .split(area);

    frame.render_widget(
        Paragraph::new(format!("Genja tasks ({})", state.descriptors().len())),
        rows[0],
    );
    render_search(state, frame, rows[1]);

    if rows[2].height > 0 {
        if let Some(error) = state.error() {
            render_message(
                frame,
                rows[2],
                "Discovery error",
                format!(
                    "Unable to load task descriptors.\n\n{error}\n\nFor compiled Rust tasks, check discovery with your project's CLI:\nmy_project_cli task list\nReplace my_project_cli with your binary name."
                ),
            );
        } else if state.descriptors().is_empty() {
            render_message(frame, rows[2], "Tasks", empty_message());
        } else if state.matching_indices().is_empty() {
            let guidance = if state.is_search_active() {
                "Press Ctrl+u to clear the search, or edit the text."
            } else {
                "Press Esc to clear the search, or / to edit it."
            };
            render_message(
                frame,
                rows[2],
                "Tasks",
                format!("No tasks match the current search.\n\n{guidance}"),
            );
        } else {
            render_tasks(state, frame, rows[2]);
        }
    }

    if rows[3].height > 0 && state.error().is_none() {
        let shown = state.matching_indices().len();
        let noun = if shown == 1 { "task" } else { "tasks" };
        frame.render_widget(
            Paragraph::new(format!(
                "{shown} {noun} shown, {} total",
                state.descriptors().len()
            )),
            rows[3],
        );
    }
    frame.render_widget(Paragraph::new(controls(state, rows[4].width)), rows[4]);
}

/// Draw selected-descriptor inspection without polling events or changing state.
/// Drawing clamps the effective offset after resize without mutating state.
fn render_inspection(state: &TaskBrowserState, frame: &mut Frame<'_>, area: Rect) {
    let rows = inspection_rows(area);
    let schema = state.active_panel() == BrowserPanel::Schema;
    let title = if schema { "Schema" } else { "Details" };
    frame.render_widget(
        Paragraph::new(format!("Genja Task Browser - {title}")),
        rows[0],
    );
    let mut position = None;
    if let Some(viewport) = inspection_viewport(state, area) {
        let requested = state
            .inspection_scroll_offset(state.active_panel())
            .unwrap_or(0);
        let offset = viewport.effective_offset(requested);
        if viewport.inner.height > 0 && viewport.line_count > 0 {
            position = Some(format!(
                "Lines {}-{} of {}",
                offset + 1,
                offset
                    .saturating_add(usize::from(viewport.inner.height))
                    .min(viewport.line_count),
                viewport.line_count
            ));
        }
        frame.render_widget(viewport.block, viewport.body);
        frame.render_widget(
            viewport
                .paragraph
                .scroll((u16::try_from(offset).unwrap_or(u16::MAX), 0)),
            viewport.inner,
        );
    } else {
        render_message(
            frame,
            rows[1],
            title,
            "No task selected. Return to the task list to select one.",
        );
    }
    let controls = [
        "Up/k Down/j: scroll | PgUp/PgDn | Home/End | Tab: details/schema | Esc: tasks | q: quit",
        "j/k: scroll | Tab: details/schema | Esc: tasks | q: quit",
        "Tab: view | Esc: tasks | q: quit",
        "Esc: tasks | q: quit",
        "Esc: back | q: quit",
        "q: quit",
        "q",
    ]
    .into_iter()
    .find(|text| text.len() <= usize::from(rows[2].width))
    .unwrap_or("");
    let status = position
        .map(|position| format!("{position} | {controls}"))
        .filter(|text| text.len() <= usize::from(rows[2].width))
        .unwrap_or_else(|| controls.to_string());
    frame.render_widget(Paragraph::new(status), rows[2]);
}

/// Show search focus and keep the end of an edited query visible.
/// Use widget styling rather than taking control of the host's terminal cursor.
fn render_search(state: &TaskBrowserState, frame: &mut Frame<'_>, area: Rect) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let active = state.is_search_active();
    let label = if active {
        "Search [editing]: "
    } else {
        "Search: "
    };
    let style = if active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let columns = Layout::horizontal([Constraint::Length(label.len() as u16), Constraint::Min(0)])
        .split(area);
    frame.render_widget(Paragraph::new(label).style(style), columns[0]);
    let query = Line::raw(state.filter_text());
    let offset = if active {
        query.width().saturating_sub(usize::from(columns[1].width))
    } else {
        0
    };
    frame.render_widget(
        Paragraph::new(query).scroll((0, u16::try_from(offset).unwrap_or(u16::MAX))),
        columns[1],
    );
}

/// Select mode-appropriate controls that fit the available ASCII column width.
fn controls(state: &TaskBrowserState, width: u16) -> &'static str {
    let candidates: &[&str] = if state.is_search_active() {
        &[
            "Enter/Esc: done | Backspace: delete | Ctrl+u: clear",
            "Enter/Esc: done | Ctrl+u: clear",
            "Esc: done",
            "Esc",
        ]
    } else if !state.filter_text().is_empty() {
        &[
            "/: search | Up/k Down/j: move | Home/End | Enter: details | q: quit | Esc: clear",
            "/: search | j/k: move | Enter: details | q: quit | Esc: clear",
            "Enter: details | /: search | q: quit | Esc: clear",
            "/: search | Up/k Down/j: move | Home/End: first/last | q: quit | Esc: clear",
            "/: search | j/k: move | q: quit | Esc: clear",
            "Esc: clear | q: quit | /: edit",
            "q: quit | Esc: clear",
            "q: quit",
            "q",
        ]
    } else if state.error().is_some() || state.descriptors().is_empty() {
        &[
            "/: search | q / Esc: quit",
            "q / Esc: quit",
            "q/Esc: quit",
            "q",
        ]
    } else {
        &[
            "/: search | Up/k Down/j: move | Home/End | Enter: details | q / Esc: quit",
            "/: search | j/k: move | Enter: details | q / Esc: quit",
            "Enter: details | /: search | q / Esc: quit",
            "/: search | Up/k Down/j: move | Home/End: first/last | q / Esc: quit",
            "/: search | j/k: move | q / Esc: quit",
            "/: search | q/Esc: quit",
            "q/Esc: quit",
            "q",
        ]
    };
    candidates
        .iter()
        .copied()
        .find(|text| text.len() <= usize::from(width))
        .unwrap_or("")
}

/// Provide registration and linking guidance for a genuinely empty snapshot.
fn empty_message() -> Text<'static> {
    let heading = Style::default().add_modifier(Modifier::BOLD);
    let code = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
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

/// Render a wrapped message with borders only when the area can hold them.
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
