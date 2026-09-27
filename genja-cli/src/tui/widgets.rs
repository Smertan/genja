//! Task descriptor table rendering without discovery or terminal ownership.

use genja_core::task::TaskExecutionMode;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Row, Table},
};

use super::TaskBrowserState;

pub(super) fn render_tasks(state: &TaskBrowserState, frame: &mut Frame<'_>, area: Rect) {
    let rows = state.descriptors().iter().enumerate().map(|(index, task)| {
        let selected = state.selected_index() == Some(index);
        let mode = match task.execution_mode {
            TaskExecutionMode::Blocking => "blocking",
            TaskExecutionMode::Async => "async",
        };
        let row = Row::new([
            if selected { ">" } else { "" },
            task.id.as_str(),
            task.version.as_str(),
            task.name.as_str(),
            mode,
            if task.constructible { "yes" } else { "no" },
        ]);
        if selected {
            row.style(
                Style::default()
                    .fg(Color::White)
                    .bg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            row
        }
    });
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Fill(3),
            Constraint::Length(9),
            Constraint::Fill(2),
            Constraint::Length(8),
            Constraint::Length(13),
        ],
    )
    .header(
        Row::new(["", "ID", "VERSION", "NAME", "MODE", "CONSTRUCTIBLE"])
            .style(Style::default().add_modifier(Modifier::BOLD)),
    )
    .column_spacing(1)
    .block(Block::default().title("Tasks").borders(Borders::ALL));
    frame.render_widget(table, area);
}
