# Terminal User Interface (TUI)

Genja is developing a terminal user interface for browsing task descriptors.
Today, Rust applications can embed a basic browser screen in an existing
Ratatui application or run that screen in a full-screen terminal session.
Run `genja tui` in a build with the `tui` feature to open the basic screen.
Press `q` or Escape to quit. The table shows task ID, version, name, execution
mode, and constructible status. The selected row has a `>` marker and colour
highlight. Search, details, and execution are not implemented yet.

The **CONSTRUCTIBLE** column shows `yes` when the running binary has a registered
factory to create that task from JSON input, and `no` when only its descriptor
is available through discovery. It does not indicate whether the task has run
or will succeed. The TUI does not construct or execute tasks.

The `no` value means registry-based creation by task identity and JSON input
is unavailable until a construction factory is registered. Direct creation
of the Rust task struct remains possible. See
[Constructible Descriptor Field](task-registration.md#constructible-descriptor-field)
for the registration rules and example structs.

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
descriptor loading, event handling, a basic screen, and a full-screen runner.
The TUI's `run_main()` helper is available for project-local binaries.

## Launch from the CLI

From a checkout, launch the screen in an interactive terminal:

```bash
cargo run -p genja-cli --features tui -- tui
```

To launch with two sample task registrations, use the
[task browser example](examples.md#cli-and-tui-task-browser).
**This requires a local checkout of the Genja GitHub repository.** Run the
command from the checkout's root; `cargo add genja` does not make dependency
examples runnable from your own project:

```bash
cargo run -p genja --features genja-tui --example task_browser -- tui
```

You can inspect the command help without entering terminal mode:

```bash
cargo run -p genja-cli --features tui -- tui --help
```

Without the `tui` feature, the command is omitted from help and rejected by
the parser. Piped or redirected input/output is rejected before terminal setup.
Startup and runtime errors return a failure exit code; errors are reported
after the runner's terminal cleanup.

A project-local CLI binary that calls `genja_cli::run_main()` can also expose
this command when built with `tui`. For `genja::cli::run_main()`, enable
`genja-tui`. Link the project task crate as described in the
[CLI binary guide](cli.md#project-local-cli-binary), then run:

```bash
my_project_cli tui
```

Discovery sees compiled Rust tasks linked into the running binary, including
when launched through the CLI command. It does not inspect another project's
registrations on disk or call another CLI command.

The design separates descriptor loading through the shared
`TaskDescriptorSource` from browser state, events, rendering, and terminal
ownership.

## Project-local TUI binary

Compiled Rust task discovery is process-local. A generic installed binary only
sees tasks linked into that binary. Link your task crate in a project-specific
binary, then call the TUI helper:

```rust
use my_project_tasks as _;

fn main() -> std::process::ExitCode {
    genja_cli::tui::run_main()
}
```

Replace `my_project_tasks` with your task crate. If your tasks live in the
binary crate, declare their modules there instead. For a dependency on `genja`
with the `genja-tui` feature, use the forwarded helper path:

```rust
use my_project_tasks as _;

fn main() -> std::process::ExitCode {
    genja::cli::tui::run_main()
}
```

Name the binary for your project, such as `my_project_tui`. Enabling the feature
provides library code; it does not install a TUI executable automatically.
`run_main()` chooses the compiled Rust descriptor source and returns an exit
code. It does not parse CLI arguments. See the [CLI binary guide](cli.md#project-local-cli-binary)
for Cargo package and binary naming examples.

## Runner for a selected descriptor source

To run the basic screen in a terminal, pass any `TaskDescriptorSource` to
`run_tui`. For compiled Rust tasks, the binary must link its task crate:

```rust
use my_project_tasks as _;
use genja_cli::discovery::rust::CompiledTaskDescriptorSource;
use genja_cli::tui::{TuiOptions, run_tui};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = CompiledTaskDescriptorSource::new();
    run_tui(&source, TuiOptions::default())?;
    Ok(())
}
```

This form lets callers choose a source instead of using the compiled Rust
source selected by `run_main()`.

The runner loads descriptors before entering raw mode, then draws on startup,
selection changes, and terminal resize. Press `q` or Escape to exit. It restores
the terminal on normal exit, errors, and panic unwinding: it leaves the alternate
screen, makes the cursor visible, and restores normal keyboard input so the
shell can be used again. If a cleanup operation fails, the runner reports the
error and its guard retries unfinished cleanup when dropped. `TuiOptions` has no
configurable settings yet. Terminal failures return a `TuiError`. The table
preserves discovery ordering.
The visible range follows selection, keeping it near the middle where possible.
Home and End reveal the first and last task; resizing recomputes the visible
range without changing selection or descriptor ordering. A browser area needs
at least six lines to display a task row with the title, borders, header, and
quit guidance.

Columns adapt to the browser area's width:

| Width in terminal cells | Visible columns |
| --- | --- |
| 100 or more | ID, version, name, mode, constructible |
| 64–99 | All five fields, with shorter headers |
| 40–63 | ID, version, mode, constructible |
| Under 40 | ID only |

Long cell values are clipped to fit. Widen the terminal to reveal more text and
columns. The `CONSTR.` header means constructible. Quit guidance is shortened
on narrow screens, and rendering safely handles areas too small for task rows.

### Empty Results And Loading Errors

An empty result shows **No registered tasks available**, with a reminder that
compiled Rust tasks must be linked into a project-local CLI/TUI binary. This
is a successful discovery result; quitting returns a success exit code. The
screen shows the minimum annotation for discovering an existing task
implementation and the line that links its crate into your CLI/TUI binary:

```rust
#[genja_task(name = "my_task")]
impl MyTask { /* start or start_async method */ }
```

```rust
use my_project_tasks as _;
```

These are illustrative snippets: replace the names and supply the task's
implementation. The annotation enables descriptor discovery with a generated
ID; explicit `registration(...)` adds a stable ID and construction factory.
See [Task Registration](task-registration.md) for complete construction and
registration guidance, or run the
[task browser example](examples.md#cli-and-tui-task-browser) from a repository
checkout to inspect working sample tasks.

A discovery failure shows a **Discovery error** screen with the source's error
message. The screen suggests checking compiled Rust discovery with
`my_project_cli task list`; replace `my_project_cli` with your project-local CLI
binary's name. Press `q` or Escape to close the screen. The runner restores the
terminal, then returns `TuiError::Discovery`; the CLI reports the error on stderr
and exits with failure. A terminal startup, rendering, or cleanup failure takes
precedence over the discovery error.

Discovery still finishes before terminal setup, but a discovery failure no
longer returns immediately from `run_tui`. Embedded callers retain the existing
`TaskBrowser::load_from()` behaviour: it returns the error immediately and stores
it in browser state so the host can choose whether to render it. A later
successful load clears that error and shows the loaded tasks or empty state.

`TaskBrowser::new()` creates an empty browser. Call `load_from(&source)` with
any `TaskDescriptorSource`, including a trait object, to synchronously load an
owned snapshot. Loading calls `list_tasks()` once, preserves source ordering,
and retains no source reference. Success reapplies the query and selects the
first matching descriptor, or none when there are no matches. Failure clears descriptors and selection, stores the
`DiscoveryError` in state, and returns it to the caller. A later successful
load clears that error. Loading does not schedule refreshes or own a terminal.

Use `state()` for read access and `state_mut()` for validated selection and
presentation-state updates. `select(None)` clears selection;
`select(Some(index))` uses the full snapshot index and rejects out-of-range or
filtered-out indices without changing selection. `descriptors()` always returns
the full snapshot; `matching_indices()` and `filtered_descriptors()` expose matches.
`selected_visible_index()` gives the selected task's filtered position.

`set_filter_text()` filters immediately using a case-insensitive substring of
ID, name, version, description, or execution mode (`blocking` or `async`).
Surrounding whitespace is ignored; blank queries show all tasks. Matching uses
Unicode lowercase conversion without fuzzy matching or accent normalization.
Selection stays on the same task when it matches; otherwise the previous visible
position is clamped to the new list. No matches clears selection; clearing the
query restores all tasks. Filter text and `BrowserPanel` focus survive loads.
Interactive search input and filtered rendering are not implemented yet; this
API currently supplies filtering state and navigation for embedding applications.
Panel rendering is reserved. Quit state belongs to the host app.

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
supports these keyboard controls:

| Keys | Action |
| --- | --- |
| Up / `k` | Select the previous task |
| Down / `j` | Select the next task |
| Home | Select the first task |
| End | Select the last task |
| `q` / Escape | Request quit |

Navigation stops at either end and does nothing for an empty list. If selection
has been cleared, Up/Down, `j`/`k`, and Home select the first task; End selects
the last. Navigation keys require no modifiers. Key releases and repeats are
ignored. Unhandled events return `Ignored` to the host. The browser
shows a task count, descriptor table, selection highlight, and status or
discovery errors; interactive search rendering and descriptor details are future work.
The browser never enters raw mode, polls events, or restores the terminal.

CLI-only users should keep using `genja-cli` on `genja`, or a direct `genja-cli`
dependency without `tui`. This avoids activating Ratatui and its dependencies
through Genja. Crossterm is already used by the CLI table renderer. Cargo
features are additive: another dependency enabling the TUI feature in the same
resolved build can activate it. Cargo.lock may list optional packages even
when they are not compiled for a CLI-only build.
