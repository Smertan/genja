# Genja CLI

First-party command-line interface for Genja automation workflows.

## Dependency Options

Projects already using `genja` can enable its optional `genja-cli` feature and
call `genja::cli::run_main()`. This feature is disabled by default and is
currently unreleased, planned for v0.5.0. Once published:

```bash
cargo add genja --features genja-cli
```

Dedicated CLI wrapper packages can depend directly on `genja-cli` and call
`genja_cli::run_main()`, as shown below. Both paths expose the same library.
See the [CLI guide](../docs/cli.md) for dependency setup from a checkout before
publication.

Neither option installs an executable automatically. Build a project-specific
binary that links your tasks, then distribute that executable to end users.

## Optional TUI Foundation

The opt-in `tui` feature exposes browser state and loading at `genja_cli::tui`
and enables Ratatui and Crossterm support. It is disabled by default. Projects
using `genja` can enable `genja-tui`, which forwards to `genja-cli/tui` and also
enables CLI support, exposing the same module at `genja::cli::tui`.

This is currently a foundation only: `TaskBrowser` supports state and descriptor
loading, but rendering, input handling, the TUI runner, `run_main()` helper
inside `tui`, and the `genja tui` command are not available yet. No separate TUI
installation is required. See the [CLI guide](../docs/cli.md#optional-tui-foundation)
for checkout-based feature configuration and dependency behavior.

## Project-Local Task Discovery

Compiled Rust task discovery is process-local. A generic installed `genja`
binary can only discover tasks linked into that binary.

Rust projects that define their own `#[genja_task]` registrations should build
a project-specific binary that links the task crate and delegates to
`genja-cli`:

```rust
use my_project_tasks as _;

fn main() -> std::process::ExitCode {
    genja_cli::run_main()
}
```

Use a project-specific binary name such as `my_project_cli`, `acme_genja`, or
`network_tasks_cli` so it does not shadow a globally installed `genja` binary.
Cargo uses `[package].name` as the executable name for a default `src/main.rs`
binary:

```toml
[package]
name = "my_project_cli"
```

You can keep a different package name and override the executable name with an
explicit binary target:

```toml
[package]
name = "my-project"

[[bin]]
name = "my_project_cli"
path = "src/bin/my_project_cli.rs"
```

Then run discovery through that project binary:

```bash
my_project_cli task list
my_project_cli task describe auto:my_project::tasks::BackupTask@0.1.0
my_project_cli task docs > task-catalog.md
```

For file exports, use JSON or YAML for complete serialized descriptors,
`task list --output markdown` for a compact summary, and `task docs` for a
complete Markdown catalogue. See [Exporting Task Descriptors](../docs/cli.md#exporting-task-descriptors)
for redirection examples and documentation-platform considerations.

During development, run the same binary through Cargo:

```bash
cargo run --bin my_project_cli -- task list
cargo run --bin my_project_cli -- task docs
```

Use `-p <package>` as well when the binary belongs to another workspace
package.

For deployment, build the project-local binary during release or CI and copy
the resulting executable to the target machine. The target machine does not
need Cargo when you deploy a prebuilt binary.

For tests or embedded callers that provide their own arguments, use
`genja_cli::run(args)` instead of `genja_cli::run_main()`.
