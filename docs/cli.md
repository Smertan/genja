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

### Through The Genja Feature

For projects already using `genja`, enable the optional `genja-cli` feature.
This feature is disabled by default and exposes the CLI library as `genja::cli`.

The feature is currently unreleased and planned for v0.5.0. Once a release
containing it is published, add it with:

```bash
cargo add genja --features genja-cli
```

To use the current checkout before publication, use a path dependency instead
(replace the path with the location of the `genja` crate in your checkout):

```toml
[dependencies]
genja = { path = "../genja/genja", features = ["genja-cli"] }
```

Add the entry point to your binary:

```rust
use my_project_tasks as _;

fn main() -> std::process::ExitCode {
    genja::cli::run_main()
}
```

### Direct CLI Dependency

A dedicated CLI wrapper package can depend directly on `genja-cli` without
depending on the full `genja` runtime. Once `genja-cli` is published:

```bash
cargo add genja-cli
```

Before publication, use a path dependency on the checkout's `genja-cli` crate:

```toml
[dependencies]
genja-cli = { path = "../genja/genja-cli" }
```

Then add a binary target that references the task crate and delegates to
`genja_cli::run_main()`:

```rust
use my_project_tasks as _;

fn main() -> std::process::ExitCode {
    genja_cli::run_main()
}
```

### Link Your Tasks

In either example, replace `my_project_tasks` with your task crate and add it as
a dependency of the binary package. The `use my_project_tasks as _;` line
intentionally links the crate that contains `#[genja_task]` registrations.
If your tasks live in the binary crate, declare the task modules there. If they
live in the same package's library target, reference that library from the binary.

Both dependency options expose the same CLI library and discover only tasks
linked into the running executable. Adding a dependency or enabling the feature
does not create a binary entry point or install the generic `genja` command.
End users receive your project-built executable; they do not need to install
`genja-cli` separately.

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

`genja::cli::run_main()` (with the feature) and `genja_cli::run_main()` (with a
direct dependency) are the same recommended helper for binary `main` functions.
It reads arguments from the current process and returns a
`std::process::ExitCode`.

For tests or embedded callers that need to provide their own arguments, use the
lower-level `genja::cli::run(args)` or `genja_cli::run(args)` function.
