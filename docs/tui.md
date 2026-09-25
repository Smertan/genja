# Terminal User Interface (TUI)

Genja is developing a terminal user interface for browsing task descriptors.
Today, Rust applications can embed a basic browser screen in an existing
Ratatui application. There is no standalone TUI command yet.

The browser is an optional part of `genja-cli`, enabled with its `tui` feature.
Projects using the main `genja` crate can enable `genja-tui`, which also enables
`genja-cli` and forwards to `genja-cli/tui`. Both TUI features are disabled by
default; they do not require a separate package installation.

These features are currently unreleased. Choose one dependency setup from a
checkout, adjusting the path for your project:

```toml
[dependencies]
genja = { path = "../genja/genja", features = ["genja-tui"] }
```

Or depend directly on the CLI crate:

```toml
[dependencies]
genja-cli = { path = "../genja/genja-cli", features = ["tui"] }
```

The `genja::cli::tui` or `genja_cli::tui` module provides browser state,
descriptor loading, event handling, and a basic screen. A TUI main
helper and the `genja tui` command are not available yet. The design separates
descriptor loading through the shared `TaskDescriptorSource` from browser state,
events, rendering, and terminal ownership. Compiled task discovery will retain
the project-local linking model of the [CLI](cli.md); the final TUI binary
entrypoint example will accompany its helper.

`TaskBrowser::new()` creates an empty browser. Call `load_from(&source)` with
any `TaskDescriptorSource`, including a trait object, to synchronously load an
owned snapshot. Loading calls `list_tasks()` once, preserves source ordering,
and retains no source reference. Success selects the first descriptor or none
for an empty result. Failure clears descriptors and selection, stores the
`DiscoveryError` in state, and returns it to the caller. A later successful
load clears that error. Loading does not schedule refreshes or own a terminal.

Use `state()` for read access and `state_mut()` for validated selection and
reserved presentation-state updates. `select(None)` clears selection;
`select(Some(index))` rejects out-of-range indices without changing selection.
Filter text and `BrowserPanel` focus are preserved across loads, but do not
filter results or render panels yet. Quit state belongs to the host app.

Applications that already own a Ratatui frame and Crossterm event loop can
embed the component directly:

```rust
use genja_cli::tui::{BrowserOutcome, TaskBrowser};

let mut browser = TaskBrowser::new();
browser.load_from(&source)?;

// Within the host's existing draw and event loop:
terminal.draw(|frame| browser.render(frame, browser_area))?;
if browser.handle_event(&event) == BrowserOutcome::QuitRequested {
    // The host decides whether to exit its loop.
}
```

`handle_action()` accepts browser actions without Crossterm. `handle_event()`
translates pressed Up and Down keys into selection changes and `q` or Escape
into a quit request. Unhandled events return `Ignored` to the host. The shell
shows a task count, reserved navigation and inspection areas, and status or
discovery errors; task rows, filtering, and descriptor details are future work.
The browser never enters raw mode, polls events, or restores the terminal.

CLI-only users should keep using `genja-cli` on `genja`, or a direct `genja-cli`
dependency without `tui`. This avoids activating Ratatui and its dependencies
through Genja. Crossterm is already used by the CLI table renderer. Cargo
features are additive: another dependency enabling the TUI feature in the same
resolved build can activate it. Cargo.lock may list optional packages even
when they are not compiled for a CLI-only build.
