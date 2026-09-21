# Command Line Interface

Genja provides a first-party CLI crate, `genja-cli`, for listing, describing,
and documenting registered task descriptors.

## Generic CLI

The `genja-cli` package builds a binary named `genja`. A generic installed
`genja` binary is useful for CLI behavior that does not depend on project-local
compiled task registrations.

Compiled Rust task discovery is process-local. A generic `genja` binary can
only discover tasks linked into that `genja` executable. It cannot inspect task
registrations from another Rust project just because that project is present on
disk.

```bash
genja --help
genja version
```

`version` prints the Genja CLI version.

## Command Reference

The examples below use `my_project_cli` as an example project-specific executable
that links your task registrations. Replace it with your own binary name. See
[Project-Local CLI Binary](#project-local-cli-binary) for setup instructions.
The example task identities are illustrative; use the ID and version returned
by your binary's `task list` command.

### Help And Version

```bash
my_project_cli --help
my_project_cli version
my_project_cli task --help
my_project_cli task list --help
my_project_cli task describe --help
my_project_cli task docs --help
```

`version` prints the linked Genja CLI version, not your project's package version.

### List Tasks

```bash
my_project_cli task list
my_project_cli task list --output table
my_project_cli task list --output json
my_project_cli task list --output yaml
my_project_cli task list --output markdown
```

The default format is `table`. Table and Markdown lists are compact summaries
containing task ID, version, name, ID source, execution mode, and
constructibility. They omit descriptions, connection plugins, processors,
retry metadata, and input schemas.

JSON and YAML lists include the complete serialized descriptor for every listed
task. Use `task describe` to inspect one task or `task docs` to document all tasks.

### Describe A Task

Pass an identity in `<task-id>@<task-version>` form:

```bash
my_project_cli task describe acme.examples.backup_config@1.0.0
my_project_cli task describe acme.examples.backup_config@1.0.0 --output table
my_project_cli task describe acme.examples.backup_config@1.0.0 --output json
my_project_cli task describe acme.examples.backup_config@1.0.0 --output yaml
my_project_cli task describe acme.examples.backup_config@1.0.0 --output markdown
```

Use the exact ID and version shown in the list. Explicit IDs, such as
`acme.examples.backup_config`, and generated IDs, such as
`auto:my_project::tasks::BackupTask`, can both be described. Preserve the
`auto:` prefix, `::` separators, and capitalization in generated IDs:

```bash
my_project_cli task describe auto:my_project::tasks::BackupTask@0.1.0
```

The default table view displays ID, version, name, description, ID source,
execution mode, connection plugin, processors, constructibility, retry metadata,
and input schema availability. Missing optional metadata is shown as `-`.
Retry metadata is shown as compact JSON when present.

When an input schema is available, the table view includes pretty-printed JSON
below the field summary. Markdown includes the same metadata and a fenced JSON
schema block. JSON and YAML include the schema within the serialized descriptor.

### Output Formats

`task list` and `task describe` accept these lowercase `--output` values:

| Format | `task list` | `task describe` |
| --- | --- | --- |
| `table` (default) | Compact task summary rows | Field/value summary with input schema below when available |
| `json` | Array of complete descriptors | One complete descriptor object |
| `yaml` | Sequence of complete descriptors | One complete descriptor mapping |
| `markdown` | Compact task summary table | Task heading, metadata table, and input schema block when available |

JSON and YAML use the existing `TaskDescriptor` serialization contract, including
optional metadata and input schemas. Markdown output is raw Markdown text, not
a rendered terminal view.

### Generate A Task Catalogue

```bash
my_project_cli task docs
my_project_cli task docs --output markdown
```

`task docs` supports Markdown only, and uses it by default. For registered tasks,
it generates a document with a `Task Catalogue` heading, an index linking to the
summary and individual tasks, a summary table, and detailed per-task sections.
Each task section includes descriptor metadata and a fenced JSON input schema
when available.

Unlike the compact `task list --output markdown` table, the catalogue summary
also includes descriptions. Use this command for complete task documentation.

## Project-Local CLI Binary

For Rust projects with compiled task registrations, build a project-specific
binary that links your task crate and delegates command handling to
`genja-cli`.

Use a project-specific binary name instead of `genja`, for example:

- `my_project_cli`
- `acme_genja`
- `network_tasks_cli`

This avoids overwriting or shadowing a globally installed `genja` binary.

The command name comes from the Cargo binary target name. If a package has a
default `src/main.rs` binary and no explicit `[[bin]]` entry, Cargo uses the
package name from `[package].name`:

```toml
[package]
name = "my_project_cli"
```

Building or installing that package produces a `my_project_cli` executable.

You can override the executable name with an explicit `[[bin]]` target:

```toml
[package]
name = "my-project"

[[bin]]
name = "my_project_cli"
path = "src/bin/my_project_cli.rs"
```

In that case the package is still named `my-project`, but the executable is
named `my_project_cli`.

Add `genja-cli` to the package that owns the project-local binary:

```bash
cargo add genja-cli
```

Then add a binary target that references the task crate and delegates to
`genja_cli::run_main()`:

```rust
use my_project_tasks as _;

fn main() -> std::process::ExitCode {
    genja_cli::run_main()
}
```

The `use my_project_tasks as _;` line intentionally links the crate that
contains `#[genja_task]` registrations. If your tasks live in the same package
as the binary, declare or import the task modules from the binary crate instead.

## Running Project-Local Discovery

Run task discovery commands through the project-specific binary:

```bash
my_project_cli task list
my_project_cli task describe auto:my_project::tasks::BackupTask@0.1.0
my_project_cli task docs > task-catalog.md
```

Those commands inspect the task registrations linked into `my_project_cli`.
Running the generic `genja` binary instead would inspect the registrations
linked into that generic binary.

## Development Workflow

During development, you can run the project-local binary through Cargo:

```bash
cargo run --bin my_project_cli -- task list
cargo run --bin my_project_cli -- task describe auto:my_project::tasks::BackupTask@0.1.0
cargo run --bin my_project_cli -- task docs
```

The `--` separator matters. Arguments before `--` are handled by Cargo.
Arguments after `--` are passed to the project-local binary.

If the binary belongs to another package in a workspace, select the package as
well:

```bash
cargo run -p my-project-cli --bin my_project_cli -- task list
```

## Runtime Deployment

For runtime deployments, build a project-local CLI binary during release or CI,
then deploy that executable to the target machine. The target machine does not
need Cargo if you deploy a prebuilt binary.

Build the binary:

```bash
cargo build --release --bin my_project_cli
```

The output path depends on the platform:

| Platform | Example output |
| --- | --- |
| Linux/macOS | `target/release/my_project_cli` |
| Windows | `target\release\my_project_cli.exe` |

Copy that executable to a location that matches your deployment policy. The
only requirement is that users or automation invoke the project-built
executable, either by full path or through `PATH`.

Common install locations include:

| Platform | User-local option | System-wide option |
| --- | --- | --- |
| Linux | `~/.local/bin` | `/usr/local/bin` |
| macOS | `/usr/local/bin` or `~/bin` | `/usr/local/bin` |
| Windows | `%USERPROFILE%\.cargo\bin` or another user directory on `PATH` | `C:\Program Files\<Project>\bin` |

On Linux/macOS:

```bash
./my_project_cli task list
./my_project_cli task docs > task-catalog.md
```

On Windows PowerShell:

```powershell
.\my_project_cli.exe task list
.\my_project_cli.exe task docs > task-catalog.md
```

On Windows cmd.exe:

```bat
my_project_cli.exe task list
my_project_cli.exe task docs > task-catalog.md
```

For local development, Cargo can build and install the binary for you:

```bash
cargo install --path . --bin my_project_cli
```

This is a development or build-machine convenience. Production servers do not
need Cargo when you deploy the compiled executable.

## Lower-Level Entrypoint

`genja_cli::run_main()` is the recommended helper for binary `main` functions.
It reads arguments from the current process and returns a
`std::process::ExitCode`.

For tests or embedded callers that need to provide their own arguments, use the
lower-level `genja_cli::run(args)` function.
