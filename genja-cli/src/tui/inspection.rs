//! Shared inspection content and viewport measurements for drawing and scrolling.
//!
//! Ratatui's optional rendered-line measurement uses its own word wrapping,
//! keeping scroll bounds consistent with styled and Unicode paragraph output.
//! JSON highlighting uses Syntect through `tui-syntax-highlight`; its syntax
//! grammar assets and a terminal-palette theme are loaded once, independently
//! of terminal ownership.
//! No measurements are retained in browser state or require a live terminal.

use std::sync::LazyLock;

use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
    text::{Line, Text},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use syntect::{
    highlighting::{Color as SyntaxColor, StyleModifier, Theme, ThemeItem, ThemeSettings},
    parsing::SyntaxSet,
};
use tui_syntax_highlight::Highlighter;

use super::{
    BrowserPanel, TaskBrowserState,
    detail::{detail_content, field},
};
use crate::discovery::TaskDescriptor;

/// Reuse grammar and theme data rather than loading bundled assets per frame.
static JSON_HIGHLIGHT_ASSETS: LazyLock<(SyntaxSet, Theme)> =
    LazyLock::new(|| (SyntaxSet::load_defaults_newlines(), terminal_theme()));

/// Encode terminal palette colours in Syntect's ANSI colour representation.
/// The adapter converts these to Ratatui's terminal-defined colour variants.
const fn ansi(index: u8) -> SyntaxColor {
    SyntaxColor {
        r: index,
        g: 0,
        b: 0,
        a: 0,
    }
}

/// Let unstyled JSON and all backgrounds inherit terminal defaults.
const TERMINAL_DEFAULT: SyntaxColor = SyntaxColor {
    r: 0,
    g: 0,
    b: 0,
    a: 1,
};

/// Let the terminal theme supply ANSI colours for JSON semantics.
fn terminal_theme() -> Theme {
    let rule = |selector: &str, colour| ThemeItem {
        scope: selector
            .parse()
            .expect("valid built-in JSON scope selector"),
        style: StyleModifier {
            foreground: Some(colour),
            ..StyleModifier::default()
        },
    };
    Theme {
        name: Some("Genja terminal palette".into()),
        settings: ThemeSettings {
            foreground: Some(TERMINAL_DEFAULT),
            background: Some(TERMINAL_DEFAULT),
            ..ThemeSettings::default()
        },
        scopes: vec![
            rule(
                "meta.structure.dictionary.key.json string.quoted.double.json",
                ansi(6),
            ),
            rule("string", ansi(2)),
            rule("constant.numeric.json", ansi(5)),
            rule("constant.language.json", ansi(3)),
        ],
        ..Theme::default()
    }
}

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
        field("Identity", format!("{}@{}", task.id, task.version)),
        Line::default(),
    ]);
    let schema = match task.input_schema.as_ref() {
        Some(schema) => match serde_json::to_string_pretty(schema) {
            Ok(json) => highlighted_json(&json).unwrap_or_else(|| Text::from(json)),
            Err(error) => Text::from(format!("Unable to format input schema: {error}")),
        },
        None => Text::from(
            "No input schema available.\n\nMissing schema metadata does not imply the task accepts no input.",
        ),
    };
    text.lines.extend(schema.lines);
    text
}

/// Colour JSON without changing the pretty-printed bytes or adding a gutter.
/// If grammar lookup or highlighting fails, the caller displays plain JSON.
fn highlighted_json(json: &str) -> Option<Text<'static>> {
    let (syntaxes, theme) = &*JSON_HIGHLIGHT_ASSETS;
    let syntax = syntaxes.find_syntax_by_extension("json")?;
    Highlighter::new(theme.clone())
        .line_numbers(false)
        .override_background(Color::Reset)
        .highlight_reader(json.as_bytes(), syntax, syntaxes)
        .ok()
}
