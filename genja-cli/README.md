# Genja CLI

First-party command-line interface for Genja automation workflows.

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

For tests or embedded callers that provide their own arguments, use
`genja_cli::run(args)` instead of `genja_cli::run_main()`.
