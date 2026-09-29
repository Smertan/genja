//! Descriptor-to-text formatting without discovery, state updates, or a terminal.
//!
//! Optional execution metadata is displayed as recorded. Formatting never
//! resolves runner defaults, constructs tasks, or interprets input schemas.

use std::{borrow::Cow, fmt::Display};

use genja_core::task::{TaskExecutionMode, TaskIdSource};
use ratatui::{
    style::{Modifier, Style},
    text::{Line, Span, Text},
};

use crate::discovery::TaskDescriptor;

/// Format the selected descriptor's core and execution metadata for inspection.
/// Description lines retain their paragraph breaks; widgets handle wrapping.
/// Schema contents and viewport scrolling are separate rendering concerns.
pub(super) fn detail_content(task: &TaskDescriptor) -> Text<'_> {
    let id_source = match task.id_source {
        TaskIdSource::Generated => "generated",
        TaskIdSource::Explicit => "explicit",
    };
    let mode = match task.execution_mode {
        TaskExecutionMode::Blocking => "blocking",
        TaskExecutionMode::Async => "async",
    };
    let constructible = if task.constructible {
        "yes (registered JSON input factory)"
    } else {
        "no (no registered JSON input factory; direct construction may still be possible)"
    };
    let mut lines = vec![
        field("Identity", format!("{}@{}", task.id, task.version)),
        field("Task ID", task.id.as_str()),
        field("ID source", id_source),
        field("Version", task.version.as_str()),
        field("Name", task.name.as_str()),
        field("Execution mode", mode),
        field("Constructible", constructible),
        field(
            "Input schema",
            if task.input_schema.is_some() {
                "Available"
            } else {
                "Not provided"
            },
        ),
        Line::default(),
        heading("Description"),
    ];
    match task
        .description
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        Some(description) => lines.extend(description.lines().map(Line::raw)),
        None => lines.push(Line::raw("No description available.")),
    }
    lines.extend([
        Line::default(),
        heading("Execution metadata"),
        field(
            "Connection plugin",
            task.connection_plugin_name
                .as_deref()
                .unwrap_or("Not specified"),
        ),
        field(
            "Processors",
            if task.processor_names.is_empty() {
                "None specified".to_string()
            } else {
                task.processor_names.join(", ")
            },
        ),
    ]);
    if let Some(retry) = task.retry.as_ref() {
        lines.extend([
            field("Retry allowed", retry_value(retry.allow())),
            field(
                "Retry max attempts (including first)",
                retry_value(retry.max_attempts()),
            ),
            field(
                "Retry delay",
                retry_value(retry.delay_ms().map(|value| format!("{value} ms"))),
            ),
            Line::raw("Unspecified retry fields use runner or built-in defaults."),
        ]);
    } else {
        lines.push(field(
            "Retry",
            "Not configured (runner or built-in defaults apply)",
        ));
    }
    Text::from(lines)
}

/// Format one labelled value while borrowing descriptor strings where possible.
fn field<'a>(label: &str, value: impl Into<Cow<'a, str>>) -> Line<'a> {
    Line::from(vec![
        Span::styled(
            format!("{label}: "),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Span::raw(value),
    ])
}

/// Distinguish descriptor sections without embedding Markdown markup.
fn heading(text: &str) -> Line<'_> {
    Line::styled(text, Style::default().add_modifier(Modifier::BOLD))
}

/// Display an optional retry override without resolving its effective value.
fn retry_value<T: Display>(value: Option<T>) -> String {
    value.map_or_else(
        || "Not specified (runner/default policy)".into(),
        |value| value.to_string(),
    )
}
