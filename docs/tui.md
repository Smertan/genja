# Terminal User Interface (TUI)

Genja is developing a terminal user interface for browsing task descriptors.
It displays configured Python tasks and Rust tasks linked into the running
binary in one list. Run a binary built with the `tui` feature to open the
full-screen browser. Rust applications can also embed the browser in an
existing Ratatui application.
Press `q` outside search to quit; Escape first leaves search, returns from
inspection, or clears a query.
The table shows task ID, version, name, implementation language, execution
mode, and constructible status. The selected row has a `>` marker and reversed
terminal colours.
The search field filters tasks as you type and shows matching and
total task counts. Press Enter on a selected task to inspect its descriptor
details, and Tab to switch to its input schema metadata. Both views support
scrolling. Applications can also embed these views in their own terminal event
loop. Task execution is not implemented.

The TUI uses the terminal's default foreground and background for ordinary text.
Selected rows reverse those colours, while search and metadata labels use
terminal-defined ANSI accents. It does not detect light or dark mode; the
terminal's own palette controls how those accents appear.

The **CONSTRUCTIBLE** column shows `yes` when the task registration includes a
factory to create the task from JSON input in its source language, and `no`
when only its descriptor is available through discovery. It does not indicate
whether the task has run or will succeed. The TUI does not construct or execute
tasks.

The **LANGUAGE** column identifies the discovery source as `🦀 Rust` or
`🐍 Python`. On narrow screens, it shows only 🦀 or 🐍. Details shows the full
label separately from **ID source** (`generated` or `explicit`). Custom
descriptor sources that do not supply language metadata show `? Unknown`
(or `?` on narrow screens). Rust and Python implementations may share an ID
and version; they appear as separate rows in the same list.

The `no` value means registry-based creation by task identity and JSON input
is unavailable until a construction factory is registered. Direct creation
of a Rust task struct remains possible in Rust application code. See
[Constructible Descriptor Field](task-registration.md#constructible-descriptor-field)
for the registration rules and example structs.

## Choose Your Project Setup

| Project | Command | Discovery |
| --- | --- | --- |
| Python only | `genja tui` | Modules declared in `[tool.genja.tasks]` |
| Rust only | `my_project_cli tui` | Rust tasks linked into the project-local binary |
| Mixed Rust and Python | `my_project_cli tui` | Linked Rust tasks and configured Python modules |

The list, search, Details, Schema, and keyboard controls below work the same
way for every setup. A Python-only project needs the `genja` binary from
`genja-cli` and `genja-py` in the selected interpreter. Rust and mixed projects
need a project-local binary to discover their compiled Rust tasks. See
[Installation](installation.md#python-task-discovery-from-the-cli) and the
[CLI binary guide](cli.md#project-local-cli-binary-rust-and-mixed-projects).

## Rust Dependency Setup

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

=== ":fontawesome-brands-python: Python"

    Declare task modules in `[tool.genja.tasks]`, install `genja-py` in the
    Python environment containing them, and run a `genja` binary built with
    `tui` from the project directory:

    ```bash
    genja tui
    ```

    The CLI uses `pyproject.toml` in the current directory. A wheel-installed
    `genja` uses the interpreter beside its executable; a Cargo-built binary
    uses `python` from `PATH`. Use `--pyproject PATH` or `--python PATH` to
    select another project file or interpreter. See
    [Installation](installation.md#python-task-discovery-from-the-cli) for the
    two installation steps.

=== ":fontawesome-brands-rust: Rust"

    Link your Rust task crate into a [project-local CLI binary](cli.md#project-local-cli-binary-rust-and-mixed-projects)
    built with `tui` (or `genja-tui` through the `genja` crate), then run:

    ```bash
    my_project_cli tui
    ```

    Without `[tool.genja.tasks]`, discovery uses linked Rust tasks and does
    not start Python.

=== "Mixed Rust and Python"

    Use a [project-local CLI binary](cli.md#project-local-cli-binary-rust-and-mixed-projects)
    that links your Rust tasks and has `tui` enabled. Declare Python task
    modules in `[tool.genja.tasks]` and install `genja-py` in the selected
    interpreter. Then run:

    ```bash
    my_project_cli tui
    ```

    The binary combines both languages in one task list. Use `--pyproject PATH`
    or `--python PATH` to select another project file or interpreter.

### Repository Checkout Examples

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
Discovery errors return before terminal setup. Runtime terminal errors return
after the runner restores terminal modes.

The standalone TUI helper uses `pyproject.toml` in the current directory and
`python` from `PATH`; it does not parse the CLI override flags.

The design separates descriptor loading through the shared
`TaskDescriptorSource` from browser state, events, rendering, and terminal
ownership.

### Example Screen (Mixed Project)

The [mixed discovery example](examples.md#mixed-rust-and-python-discovery)
shows Rust and Python implementations of the same task. Searching for
`backup_config` gives a screen like this; column widths vary with the terminal:

<div class="tui-example">
  <div class="tui-example__title">Genja tasks (4)</div>
  <div class="tui-example__search">Search: backup_config</div>
  <div class="tui-example__scroll">
    <table class="tui-example__table" aria-label="Example task implementations">
      <thead>
        <tr><th scope="col"></th><th scope="col">ID</th><th scope="col">VER</th><th scope="col">NAME</th><th scope="col">LANG</th><th scope="col">MODE</th><th scope="col">CONSTRUCTIBLE</th></tr>
      </thead>
      <tbody>
        <tr class="tui-example__selected"><td>&gt;</td><td>acme.examples.backup_config</td><td>1.0.0</td><td>backup_config</td><td>🦀 Rust</td><td>blocking</td><td>yes</td></tr>
        <tr><td></td><td>acme.examples.backup_config</td><td>1.0.0</td><td>backup_config</td><td>🐍 Python</td><td>blocking</td><td>yes</td></tr>
      </tbody>
    </table>
  </div>
  <div class="tui-example__status">2 tasks shown, 4 total</div>
</div>

The preview uses illustrative colours and row lines. The real TUI uses your
terminal's palette and adapts column widths to the available space.

Select the Python row and press Enter to see Details with `Language: 🐍 Python`
alongside `ID source: explicit`. Press Tab to inspect that implementation's
input schema, or Escape to return to the shared task list.

## Search And Navigation (All Projects)

Press `/` to focus the search field. Type text to filter immediately; Enter
returns to task navigation while keeping the query. The search field shows
`Search [editing]:` while focused, and the footer shows the available controls.

### Keyboard Controls

In task-navigation mode:

| Keys | Action |
| --- | --- |
| Up / `k` | Select the previous matching task |
| Down / `j` | Select the next matching task |
| Home | Select the first matching task |
| End | Select the last matching task |
| Enter | Open Details for the selected task |
| `/` | Focus search, retaining the current query |
| Escape | Clear a retained query, or quit if the query is empty |
| `q` | Quit, including when a query is active |

In Details or Schema:

| Keys | Action |
| --- | --- |
| Tab | Switch between Details and Schema |
| Up / `k` | Scroll up one wrapped row |
| Down / `j` | Scroll down one wrapped row |
| PageUp / PageDown | Scroll by one visible page |
| Home / End | Scroll to the first / last page |
| Escape | Return to the task list, retaining selection and search |
| `q` | Quit |

Each inspection view retains its scroll position for the selected task.
Changing the selected task resets both positions. Search is available in the
task list; `/` and Enter do nothing during inspection.

While search is focused:

| Keys | Action |
| --- | --- |
| Printable characters | Append to the query and filter immediately |
| Backspace | Remove the last Unicode scalar value |
| Ctrl+u | Clear the query and stay in search mode |
| Enter / Escape | Leave search mode, retaining the query |

`q`, `j`, `k`, and `/` enter text during search. To quit from search mode,
press Enter or Escape, then `q`. To clear and quit using Escape alone, press it
once to leave search, again to clear a nonempty query, and again to quit.
Arrow keys and Home/End are ignored during search; leave search to navigate.
Enter while editing only leaves search; press Enter again to open Details.
Shift is supported for printable input. Key releases, repeats, and paste events
are not handled. The input edits at the end of the query; it has no movable
text cursor.

### Try Descriptor Inspection

Select the language of the sample you want to inspect.

=== ":fontawesome-brands-python: Python"

    With `genja-py` installed in the selected Python environment and a `genja`
    binary built with `tui`, run this from the repository root. The sample module
    and `pyproject.toml` are in the Genja source checkout; they are not installed
    with `genja-py` from PyPI:

    ```bash
    genja --pyproject genja/examples/python/pyproject.toml tui
    ```

    This uses the [configured Python task module](examples.md#mixed-rust-and-python-discovery)
    without linking Rust tasks into the binary. In your own Python project, declare
    your task modules in its `pyproject.toml` and run `genja tui` from that project
    directory instead.

    To fetch the example directory without checking out the full repository, use a
    sparse partial clone:

    ```bash
    git clone --depth 1 --filter=blob:none --sparse https://github.com/Smertan/genja.git genja-python-example
    git -C genja-python-example sparse-checkout set genja/examples/python
    cd genja-python-example/genja/examples/python
    genja tui
    ```

    The final clone argument creates `genja-python-example` in your current
    directory. Sparse checkout then places the example files under
    `genja-python-example/genja/examples/python`; the `cd` command enters that
    directory before launching the TUI.

    The clone command checks out `main`, this repository's default branch. If the
    example has not yet been merged into `main`, add `--branch` followed by the
    branch name to `git clone` so Git fetches the branch containing these files.

    If you do not want a Git checkout, open the
    [Genja repository](https://github.com/Smertan/genja) on GitHub and select a
    branch containing this example. Navigate to `genja/examples/python` and use
    **Raw → Download raw file** for `pyproject.toml` and
    `task_registration_tasks.py`. Save both in the same new directory with their
    original filenames, then run `genja tui` there. Only those two files are needed
    for this TUI sample.

    1. Select `backup_config` and press Enter. Details shows `Language: 🐍 Python`,
       its explicit ID, and its constructible status.
    2. Press Tab to see its input schema, including `backup_path` and `compress`.
    3. Press Escape, select `collect_facts`, and press Enter. Its Details view also
       shows Python as the language. Press Tab to see the missing-schema message,
       then `q` to quit.

=== ":fontawesome-brands-rust: Rust"

    This walkthrough uses the Rust-only `task_browser` example. From a local
    checkout, launch it in an interactive terminal:

    ```bash
    cargo run -p genja --features genja-tui --example task_browser -- tui
    ```

    1. Select `backup_config` and press Enter. Details shows its identity, version,
       description, blocking execution mode, and registered JSON factory status.
    2. Press Tab to inspect its input schema, including the `backup_path` field.
       Use Down/Up, PageDown/PageUp, or Home/End to scroll as needed.
    3. Press Escape to return to the list, select `collect_facts`, and press Enter.
       It has a generated ID, async execution mode, and no registered JSON factory.
    4. Press Tab. This task has no input schema metadata, so the view explains that
       no schema is available. Press Escape to return or `q` to quit.

    To try inspection after filtering, press `/`, type `backup`, then Enter to leave
    search and Enter again to open Details. Escape returns to the filtered list
    without clearing the query. The sample tasks are linked into this example
    binary; a generic CLI binary may have no tasks to inspect.

### Matching And Selection

The query is a case-insensitive substring matched against task ID, name,
version, description, and execution mode (`blocking` or `async`). A match in
any one field includes the task. Descriptions can match even though the table
does not display them; tasks without descriptions can still match other fields.
Constructible status and input schema contents are not searched.

The whole query is used as one substring: `backup config` does not mean two
separate search terms. Surrounding whitespace is ignored, and an empty or
whitespace-only query shows all tasks. Matching uses Unicode lowercase
conversion without accent normalization. Regex, fuzzy matching, saved filters,
and an advanced query language are **not supported**.

Results stay in discovery order. Filtering keeps the selected task when it
still matches. Otherwise, selection moves to its previous position in the
filtered list, clamped to the last available result. No matches clears
selection; when matches return without a selected task, the first is selected.
Clearing a query restores all tasks and retains the currently selected task
where possible. Navigation stops at either end without wrapping.

### Try Search

Select the language of the sample you want to search.

=== ":fontawesome-brands-python: Python"

    Launch the Python sample with the command in
    [the Python tab of the inspection walkthrough](#try-descriptor-inspection):

    1. Press `/` and type `backup`. The list shows only `backup_config` and reports
       `1 task shown, 2 total` when the terminal has room for the count.
    2. Press Ctrl+u and type `host facts`. The description match shows only
       `collect_facts`, even though the table does not display descriptions.
    3. Press Ctrl+u and type `missing` to see **No tasks match the current search**.
       Press Ctrl+u to restore both Python tasks, then Enter and `q` to quit.

=== ":fontawesome-brands-rust: Rust"

    This walkthrough uses the Rust-only `task_browser` example. **Use a local
    checkout of the Genja GitHub repository and run from its root.**
    Adding Genja as a dependency with `cargo add` does not make this example runnable
    from your own project. Launch the example in an interactive terminal:

    ```bash
    cargo run -p genja --features genja-tui --example task_browser -- tui
    ```

    1. Press `/` and type `backup`. Only `backup_config` remains, with
       `1 task shown, 2 total` on terminals tall enough to show the count.
    2. Press Enter. The query stays active, and navigation operates on the matching
       task. Press Escape to clear it and restore both tasks.
    3. Press `/` and type `ASYNC`. The case-insensitive execution-mode match shows
       only `collect_facts`.
    4. While still editing, press Ctrl+u and type `JSON input schema`. Only
       `backup_config` matches, through its description.
    5. Press Ctrl+u and type `missing`. The browser shows
       **No tasks match the current search**. Press Ctrl+u to restore both tasks.
    6. Press Enter, then `q`, to quit and return to your shell.

    To try ID and version matching, search for `acme.examples.backup_config` or a
    version displayed in the table. Generated task versions follow the example
    crate's version, so a version query can match one or both tasks. Search only
    narrows descriptors; it does not construct or execute tasks.

## Project-Local TUI Binary (Rust And Mixed Projects)

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
`run_main()` selects compiled or combined discovery and returns an exit code.
It does not parse CLI arguments. See the [CLI binary guide](cli.md#project-local-cli-binary-rust-and-mixed-projects)
for Cargo package and binary naming examples.

## Runner For A Selected Descriptor Source (Rust API)

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

This form lets callers choose a source instead of using the project source
selected by `run_main()`.

The runner loads descriptors before entering raw mode, then draws on startup,
selection or search-state changes, and terminal resize. Press `q` in task mode
to exit; Escape first leaves search or clears a query. It restores
the terminal on normal exit, errors, and panic unwinding: it leaves the alternate
screen, makes the cursor visible, and restores normal keyboard input so the
shell can be used again. If a cleanup operation fails, the runner reports the
error and its guard retries unfinished cleanup when dropped. `TuiOptions` has no
configurable settings yet. Terminal failures return a `TuiError`. The table
preserves discovery ordering.
The visible range follows selection, keeping it near the middle where possible.
Home and End reveal the first and last task; resizing recomputes the visible
range without changing selection or descriptor ordering. A browser area needs
at least six lines to display search input, a task row, table borders and
header, and controls. Areas shorter than eight lines omit the title and result
count to prioritize search, tasks, and controls.

The search row shows `Search [editing]:` in a distinct colour while focused.
Long queries scroll horizontally during editing to keep the newest characters
visible. The count shows, for example, `1 task shown, 2 total`; filtering does
not change the total number of loaded descriptors. Rendering does not move the
host application's terminal cursor or mutate browser state.

Columns adapt to the browser area's width:

| Width in terminal cells | Visible columns |
| --- | --- |
| 100 or more | ID, version, name, language, mode, constructible |
| 64–99 | All six fields, with shorter headers |
| 40–63 | ID, version, language icon, mode, constructible |
| Under 40 | ID only |

Long cell values are clipped to fit. Widen the terminal to reveal more text and
columns. The `CONSTR.` header means constructible. Quit guidance is shortened
on narrow screens, and rendering safely handles areas too small for task rows.

### Empty Results And Loading Errors (All Projects)

When descriptors are loaded but none match the query, the browser shows
**No tasks match the current search**, with guidance to clear or edit the query.
In search mode, Ctrl+u clears it; in task mode, Escape clears it. Clearing restores
the full list. This is distinct from an empty discovery snapshot or loading error.

An empty result shows **No registered tasks available**, with Python module
configuration and Rust task linking guidance. This is a successful discovery
result; quitting returns a success exit code. For Rust tasks, the screen shows
the minimum annotation for discovering an existing task
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

The full-screen runner returns `TuiError::Discovery` before terminal setup when
discovery fails. The CLI reports the error on stderr and exits unsuccessfully.
Embedded callers can still render a **Discovery error** screen:
`TaskBrowser::load_from()` returns the error and stores it in browser state so
the host can choose whether to display it. A later successful load clears the
error and shows the loaded tasks or empty state.

`TaskBrowser::new()` creates an empty browser. Call `load_from(&source)` with
any `TaskDescriptorSource`, including a trait object, to synchronously load an
owned snapshot. Loading calls `list_implementations()` once, preserves source ordering,
and retains no source reference. Success reapplies the query and selects the
first matching descriptor, or none when there are no matches. Failure clears
descriptors and selection, stores the
`DiscoveryError` in state, and returns it to the caller. A later successful
load clears that error. Loading does not schedule refreshes or own a terminal.

Use `state()` for read access and `state_mut()` for validated selection and
presentation-state updates. `select(None)` clears selection;
`select(Some(index))` uses the full snapshot index and rejects out-of-range or
filtered-out indices without changing selection. `descriptors()` always returns
the full snapshot; `matching_indices()` and `filtered_descriptors()` expose matches.
`selected_visible_index()` gives the selected task's filtered position.
`languages()` and `selected_language()` expose source-assigned language metadata;
`filtered_implementations()` pairs matching descriptors with their languages.

`set_filter_text()` filters immediately using a case-insensitive substring of
ID, name, version, description, language, or execution mode (`blocking` or `async`).
Surrounding whitespace is ignored; blank queries show all tasks. Matching uses
Unicode lowercase conversion without fuzzy matching or accent normalization.
Selection stays on the same task when it matches; otherwise the previous visible
position is clamped to the new list. No matches clears selection; clearing the
query restores all tasks. Filter text survives loads; inspection focus survives
only when the replacement snapshot has a matching selected task.
Search keyboard handling is available through either event helper, and `render()`
draws the search field, matching tasks, and result counts.
`handle_event_in_area()` also provides the inspection keyboard controls and
bounded scrolling. Direct actions remain available for hosts with custom controls.
Quit state belongs to the host app.

The inspection state API supports `BrowserPanel::Tasks`, `Details`, and `Schema`.
Dispatch `OpenDetails` or `OpenSchema` to inspect the selected snapshot descriptor,
`ToggleInspectionView` to switch between inspection views, and `ReturnToTasks`
to restore the list without changing the query or selection. Inspection requests
are ignored without a selected task. `selected_descriptor()` supplies the data;
view transitions do not copy descriptors or call discovery again. Schema
inspection is allowed when metadata is absent so its renderer can show an empty
state. Entering inspection leaves search input, and search can only be focused
in Tasks. Escape returns from inspection before clearing a query or quitting.

`inspection_scroll_offset(panel)` reads each inspection view's independent
requested display-row offset. `set_inspection_scroll_offset(panel, offset)`
stores it for hosts providing their own rendering; it rejects Tasks and missing
selection. These requested state-only offsets have no content or viewport bounds.
The browser clamps the effective offset for rendering; area-aware scroll actions
also clamp their updates. Switching views or returning to the list retains offsets for the same
selected task. A selection change or descriptor reload resets both offsets.
Clearing selection, filtering to no matches, empty discovery, and loading errors
return to Tasks. `set_active_panel()` now ignores inspection requests without
selection; callers using the formerly reserved panel state should select a
matching task before requesting Details or Schema.

To render the selected task's details in an embedded application:

```rust
use genja_cli::tui::BrowserAction;

browser.handle_action(BrowserAction::OpenDetails);
terminal.draw(|frame| browser.render(frame, browser_area))?;

// Return without changing the selected task or search query.
browser.handle_action(BrowserAction::ReturnToTasks);
```

Details displays identity (`ID@version`), ID source, language, version, name, description,
execution mode, constructible status, and whether input schema metadata is
available. It also shows recorded connection plugin, processors, and retry
overrides. Missing descriptions and optional metadata have explicit labels.
Field labels use a terminal-palette accent and bold text; values retain the
terminal's default foreground for readability on light and dark backgrounds.
Retry fields are descriptor values, not resolved execution policy; unspecified
fields fall back to runner or built-in defaults. Maximum attempts includes the
first attempt. Constructible refers to a registered JSON input factory and
does not indicate execution success or prevent direct construction of the task
struct when false.

Description paragraphs and formatted schema JSON wrap to the available width.
Schema displays the selected task's identity followed by its syntax-coloured,
pretty-printed JSON metadata. Keys, strings, numbers, and boolean/null values
use distinct terminal-palette colours; JSON indentation and punctuation retain
the terminal's default foreground and background. This highlighting is part
of the optional TUI feature and
does not affect CLI JSON output or CLI-only builds.
Tasks without metadata show **No input schema available**; this does not imply
the task accepts no input. Present empty objects, booleans, and JSON `null` are
displayed as supplied rather than treated as missing. Schema inspection does
not generate an input form or execute a task.

Rendering does not reload descriptors, mutate browser state, or own the terminal
cursor. Enter opens Details from Tasks; Tab switches Details/Schema. Escape
returns from inspection to Tasks, and `q` requests quit outside search.

To scroll inspection content, pass the same clipped browser area used for
rendering, including the title and footer:

```rust
browser.handle_action(BrowserAction::OpenSchema);
browser.handle_action_in_area(BrowserAction::PageDown, browser_area);
terminal.draw(|frame| browser.render(frame, browser_area))?;
```

The scroll actions are `ScrollUp`, `ScrollDown`, `PageUp`, `PageDown`,
`ScrollToTop`, and `ScrollToBottom`. Line actions move one wrapped display row;
page actions move by the visible content height. Bounds use the same Ratatui
wrapping and border geometry as rendering. Offsets address up to 65,535 wrapped
rows, matching Ratatui's paragraph scrolling range. On resize, drawing clamps
the effective offset without modifying the requested state; the next scroll
action uses the new bounds. A line-range indicator appears when the footer has
room. The views retain independent offsets for the same selected task.

Scroll actions are ignored by `handle_action()` because it has no viewport.
Use `handle_action_in_area()` for scrolling; it delegates other actions to
`handle_action()`. Likewise, `handle_event()` supports view transitions but
ignores inspection scroll keys; use `handle_event_in_area()` for scrolling.
Scrolling is also ignored in Tasks, without selection, or
when the area has no usable content rows or columns.

Applications that already own a Ratatui frame and Crossterm event loop can
embed the component directly:

```rust
use genja_cli::tui::{BrowserOutcome, TaskBrowser};

let mut browser = TaskBrowser::new();
browser.load_from(&source)?;

// Within the host's existing draw and event loop:
terminal.draw(|frame| browser.render(frame, browser_area))?;
if browser.handle_event_in_area(&event, browser_area) == BrowserOutcome::QuitRequested {
    // The host decides whether to exit its loop.
}
```

To set a query programmatically, use the same filtering entry point as keyboard
editing:

```rust
browser.state_mut().set_filter_text("backup");
let matches = browser.state().filtered_descriptors();
```

`handle_action()` accepts browser actions without Crossterm. Event helpers
choose the [keyboard controls](#keyboard-controls) according to search focus
and the active view. The host must redraw on resize and pass the updated,
clipped area used for rendering to `handle_event_in_area()`.

Embedding applications can inspect `is_search_active()` and dispatch
`FocusSearch`, `LeaveSearch`, `AppendSearchCharacter(char)`,
`DeleteSearchCharacter`, `ClearSearch`, or `Escape` actions directly.
`action_from_event()` translates task-navigation controls; use the browser's
event helpers for context-aware editing and inspection. An explicit `Quit`
action always requests exit, including during search; keyboard `q` requests
exit from Tasks, Details, or Schema when search is not focused.

Navigation stops at either end and does nothing for an empty filtered list. If selection
has been cleared, Up/Down, `j`/`k`, and Home select the first task; End selects
the last. Navigation keys require no modifiers. Key releases and repeats are
ignored. Unhandled events return `Ignored` to the host. The browser
shows search input, result counts, a filtered descriptor table, selection
highlight, and empty/error messages, plus Details and Schema inspection views.
The browser never enters raw mode, polls events, or restores the terminal.

CLI-only users should keep using `genja-cli` on `genja`, or a direct `genja-cli`
dependency without `tui`. This avoids activating Ratatui and its dependencies
through Genja. Crossterm is already used by the CLI table renderer. Cargo
features are additive: another dependency enabling the TUI feature in the same
resolved build can activate it. Cargo.lock may list optional packages even
when they are not compiled for a CLI-only build.
