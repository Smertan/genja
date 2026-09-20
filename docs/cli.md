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
