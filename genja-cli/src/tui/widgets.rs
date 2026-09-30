//! Filtered task table rendering without discovery or terminal ownership.

use genja_core::task::TaskExecutionMode;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Row, Table, TableState},
};

use super::TaskBrowserState;

struct ColumnLayout {
    headers: &'static [&'static str],
    widths: Vec<Constraint>,
    // Indices into the selection marker followed by the five descriptor fields.
    fields: &'static [usize],
}

fn columns(width: u16) -> ColumnLayout {
    match width {
        100.. => ColumnLayout {
            headers: &["", "ID", "VERSION", "NAME", "MODE", "CONSTRUCTIBLE"],
            widths: vec![
                Constraint::Length(2),
                Constraint::Fill(3),
                Constraint::Length(9),
                Constraint::Fill(2),
                Constraint::Length(8),
                Constraint::Length(13),
            ],
            fields: &[0, 1, 2, 3, 4, 5],
        },
        64..100 => ColumnLayout {
            headers: &["", "ID", "VER", "NAME", "MODE", "CONSTR."],
            widths: vec![
                Constraint::Length(2),
                Constraint::Fill(3),
                Constraint::Length(7),
                Constraint::Fill(2),
                Constraint::Length(8),
                Constraint::Length(7),
            ],
            fields: &[0, 1, 2, 3, 4, 5],
        },
        40..64 => ColumnLayout {
            headers: &["", "ID", "VER", "MODE", "CONSTR."],
            widths: vec![
                Constraint::Length(2),
                Constraint::Fill(1),
                Constraint::Length(7),
                Constraint::Length(8),
                Constraint::Length(7),
            ],
            fields: &[0, 1, 2, 4, 5],
        },
        _ => ColumnLayout {
            headers: &["", "ID"],
            widths: vec![Constraint::Length(2), Constraint::Fill(1)],
            fields: &[0, 1],
        },
    }
}

/// Render matching tasks using filtered positions for selection and scrolling.
/// The viewport follows selection without changing the snapshot or state.
pub(super) fn render_tasks(state: &TaskBrowserState, frame: &mut Frame<'_>, area: Rect) {
    let columns = columns(area.width);
    let selected_index = state.selected_visible_index();
    let rows = state
        .filtered_descriptors()
        .enumerate()
        .map(|(index, task)| {
            let selected = selected_index == Some(index);
            let mode = match task.execution_mode {
                TaskExecutionMode::Blocking => "blocking",
                TaskExecutionMode::Async => "async",
            };
            let fields = [
                if selected { ">" } else { "" },
                task.id.as_str(),
                task.version.as_str(),
                task.name.as_str(),
                mode,
                if task.constructible { "yes" } else { "no" },
            ];
            let row = Row::new(columns.fields.iter().map(|index| fields[*index]));
            if selected {
                row.style(Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED))
            } else {
                row
            }
        });
    let table = Table::new(rows, columns.widths)
        .header(
            Row::new(columns.headers.iter().copied())
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .column_spacing(1)
        .block(Block::default().title("Tasks").borders(Borders::ALL));
    // Two border lines and one header line leave the remaining height for tasks.
    // Derive the viewport each frame so resize and navigation need no state mutation.
    let visible_rows = usize::from(area.height.saturating_sub(3));
    let offset = selected_index.map_or(0, |selected| {
        selected
            .saturating_sub(visible_rows / 2)
            .min(state.matching_indices().len().saturating_sub(visible_rows))
    });
    let mut table_state = TableState::default()
        .with_selected(selected_index)
        .with_offset(offset);
    frame.render_stateful_widget(table, area, &mut table_state);
}
