# Command Line Interface (Rust)

Genja provides a first-party CLI crate, `genja-cli`, for listing, describing,
and documenting registered task descriptors. Currently, CLI discovery supports
**compiled Rust tasks only**. **Python task discovery is planned separately.**

## Terminal User Interface (TUI)

Genja is developing an interactive task browser for the terminal. The current
TUI feature lets Rust applications embed a basic browser screen or run it in a
full-screen terminal session. A `genja tui` command is not available yet. See
the [Terminal UI guide](tui.md) for setup, usage, and current capabilities.

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

## Output Examples

These examples were generated from the registered task in
`genja/examples/task_discovery_cli.rs`. It is a documentation fixture, not a
backup implementation. The task has an explicit ID, retry metadata, and an
input schema; other tasks may have different or absent metadata.

Reproduce the output from this repository's root:

```bash
cargo run -p genja --features genja-cli --example task_discovery_cli -- task list
cargo run -p genja --features genja-cli --example task_discovery_cli -- task describe acme.examples.backup_config@1.0.0 --output json
cargo run -p genja --features genja-cli --example task_discovery_cli -- task docs
```

The examples below use `my_project_cli` as the executable name to match the
rest of this guide. Replace the arguments after `--` in the Cargo command to
reproduce each format. All displayed outputs are complete for this one-task
fixture. JSON and YAML list commands wrap the same descriptor in an array or
sequence.

/// details | Compact task list (table)
    type: example

```bash
my_project_cli task list
```

```text
 ID                          | VERSION | NAME          | SOURCE   | MODE  | CONSTRUCTIBLE
==========================================================================================
 acme.examples.backup_config | 1.0.0   | backup_config | explicit | async | yes
```
///

/// details | Compact task list (raw Markdown)
    type: example

```bash
my_project_cli task list --output markdown
```

```markdown
| ID                            | Version | Name            | Source     | Mode    | Constructible |
|-------------------------------|---------|-----------------|------------|---------|---------------|
| `acme.examples.backup_config` | `1.0.0` | `backup_config` | `explicit` | `async` | yes           |
```
///

/// details | Task details and input schema (table)
    type: example

```bash
my_project_cli task describe acme.examples.backup_config@1.0.0
```

```text
Task: acme.examples.backup_config@1.0.0

 Field             | Value
====================================================================
 ID                | acme.examples.backup_config
-------------------+------------------------------------------------
 Version           | 1.0.0
-------------------+------------------------------------------------
 Name              | backup_config
-------------------+------------------------------------------------
 Description       | Backs up selected paths from a network device
-------------------+------------------------------------------------
 ID source         | explicit
-------------------+------------------------------------------------
 Execution mode    | async
-------------------+------------------------------------------------
 Connection plugin | ssh
-------------------+------------------------------------------------
 Processors        | -
-------------------+------------------------------------------------
 Constructible     | yes
-------------------+------------------------------------------------
 Retry             | {"allow":true,"max_attempts":3,"delay_ms":250}
-------------------+------------------------------------------------
 Input schema      | available

Input schema:
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "backup_path": {
      "type": "string"
    },
    "compress": {
      "type": "boolean"
    }
  },
  "required": [
    "backup_path",
    "compress"
  ],
  "title": "BackupConfig",
  "type": "object"
}
```
///

/// details | Complete descriptor (JSON)
    type: example

```bash
my_project_cli task describe acme.examples.backup_config@1.0.0 --output json
```

```json
{
  "id": "acme.examples.backup_config",
  "id_source": "explicit",
  "name": "backup_config",
  "version": "1.0.0",
  "description": "Backs up selected paths from a network device",
  "execution_mode": "async",
  "connection_plugin_name": "ssh",
  "processor_names": [],
  "retry": {
    "allow": true,
    "max_attempts": 3,
    "delay_ms": 250
  },
  "input_schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "properties": {
      "backup_path": {
        "type": "string"
      },
      "compress": {
        "type": "boolean"
      }
    },
    "required": [
      "backup_path",
      "compress"
    ],
    "title": "BackupConfig",
    "type": "object"
  },
  "constructible": true
}
```
///

/// details | Complete descriptor (YAML)
    type: example

```bash
my_project_cli task describe acme.examples.backup_config@1.0.0 --output yaml
```

```yaml
id: acme.examples.backup_config
id_source: explicit
name: backup_config
version: 1.0.0
description: Backs up selected paths from a network device
execution_mode: async
connection_plugin_name: ssh
processor_names: []
retry:
  allow: true
  max_attempts: 3
  delay_ms: 250
input_schema:
  $schema: https://json-schema.org/draft/2020-12/schema
  properties:
    backup_path:
      type: string
    compress:
      type: boolean
  required:
  - backup_path
  - compress
  title: BackupConfig
  type: object
constructible: true
```
///

/// details | Complete one-task catalogue (raw Markdown)
    type: example

```bash
my_project_cli task docs
```

````markdown
# Task Catalogue

## Index

- [Summary](#summary)
- [Tasks](#tasks)
  - [`acme.examples.backup_config@1.0.0`](#task-acme-examples-backup-config-1-0-0)

## Summary

| ID                            | Version | Name            | Source     | Mode    | Constructible | Description                                   |
|-------------------------------|---------|-----------------|------------|---------|---------------|-----------------------------------------------|
| `acme.examples.backup_config` | `1.0.0` | `backup_config` | `explicit` | `async` | yes           | Backs up selected paths from a network device |

## Tasks

<a id="task-acme-examples-backup-config-1-0-0"></a>

### acme.examples.backup_config@1.0.0

| Field             | Value                                            |
|-------------------|--------------------------------------------------|
| ID                | `acme.examples.backup_config`                    |
| Version           | `1.0.0`                                          |
| Name              | `backup_config`                                  |
| Description       | Backs up selected paths from a network device    |
| ID source         | `explicit`                                       |
| Execution mode    | `async`                                          |
| Connection plugin | `ssh`                                            |
| Processors        | -                                                |
| Constructible     | yes                                              |
| Retry             | `{"allow":true,"max_attempts":3,"delay_ms":250}` |
| Input schema      | available                                        |

#### Input Schema

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "properties": {
    "backup_path": {
      "type": "string"
    },
    "compress": {
      "type": "boolean"
    }
  },
  "required": [
    "backup_path",
    "compress"
  ],
  "title": "BackupConfig",
  "type": "object"
}
```
````
///


## Exporting Task Descriptors

Commands write results to stdout. Use your shell's `>` redirection to save them
to a file; Genja does not provide an output-file option. These examples assume
your project-specific executable is on `PATH`.

### JSON And YAML

Export complete descriptor lists for automation or later processing:

```bash
my_project_cli task list --output json > tasks.json
my_project_cli task list --output yaml > tasks.yaml
```

Export one descriptor, including any input schema, using its listed identity:

```bash
my_project_cli task describe acme.examples.backup_config@1.0.0 --output json > backup-config.json
my_project_cli task describe acme.examples.backup_config@1.0.0 --output yaml > backup-config.yaml
```

### Markdown Documents

Choose the command according to how much detail the document needs:

| Command | Document contents |
| --- | --- |
| `task list --output markdown` | Compact summary table without descriptions or input schemas |
| `task describe <identity> --output markdown` | One task's metadata and input schema, when available |
| `task docs` | Catalogue index, summary including descriptions, and detailed sections for all tasks |

```bash
my_project_cli task list --output markdown > task-summary.md
my_project_cli task describe acme.examples.backup_config@1.0.0 --output markdown > backup-config.md
my_project_cli task docs --output markdown > task-catalog.md
```

The files contain raw Markdown. Use a Markdown renderer with table and fenced
code-block support, such as GitHub or a suitably configured Zensical/MkDocs site,
to display them. Catalogue navigation also depends on the renderer preserving
heading links and the generated HTML anchors.

For Confluence, check the import or conversion workflow supported by your editor
and installed apps. Markdown formatting shortcuts are not a guarantee that a
complete pasted Markdown catalogue will be converted correctly. Atlassian
documents [Markdown shortcuts](https://support.atlassian.com/confluence-cloud/docs/keyboard-shortcuts-markdown-and-autocomplete/)
separately from [legacy-editor markup insertion](https://support.atlassian.com/confluence-cloud/docs/insert-confluence-wiki-markup/).
Check tables, code blocks, and navigation links after importing. Genja generates
the document; it does not publish it to Confluence or another platform.

### Redirection Behaviour

The shell creates or overwrites the target file when using `>`, even if the
command later fails. Errors remain on stderr, so check the command's exit status
before treating a generated file as a successful export. Avoid combining stderr
with stdout when producing JSON or YAML for a parser.

These redirection examples also work with an executable on `PATH` in PowerShell
or cmd.exe. For an executable in the current directory, use `./my_project_cli`
on Linux/macOS, `.\my_project_cli.exe` in PowerShell, or `my_project_cli.exe` in
cmd.exe. Output-file encoding is controlled by your shell; ensure UTF-8 when
passing files between tools or platforms.

Rebuild the project binary after changing task registrations, then regenerate
the files. Discovery reflects the tasks linked into that binary, not edits to
source files that have not been compiled.

## Discovery Scope And Architecture

The initial CLI discovery implementation supports compiled Rust task descriptors
only. It reads registrations linked into the running executable; it does not
scan the current directory, build another project, or load another executable's
registry. If expected tasks are missing, check that you are running the
project-specific binary and that it links the task crate or modules.

Commands consume the shared `TaskDescriptorSource` abstraction from
`genja_cli::discovery`. It provides list and describe operations returning
`TaskDescriptor` values or a common `DiscoveryError`.
`discovery::rust::CompiledTaskDescriptorSource` is the current implementation,
backed by the compiled registry in `genja-core`.

The command handlers obtain descriptors through this abstraction and pass them
to separate output renderers. The compiled source sorts descriptors by ID and
then version string for deterministic output. This boundary allows future
descriptor sources to reuse the command and rendering layers.

These commands inspect metadata only. A descriptor's `constructible` value
reports its registration capability; listing or describing it does not construct
or execute the task. Task execution is outside the initial CLI discovery scope.

### Future Descriptor Sources

Python CLI discovery is planned separately. Python task registration happens
when the module defining a decorated task class is imported. Installing a
Python package or having its files on disk does not populate that process's
registry; the task-defining modules must be imported first.

A future Python descriptor source could import explicitly declared modules,
possibly identified by provider manifests, before reading the Python registry.
The current CLI does not import Python task modules. Python, provider-manifest,
and MCP-backed descriptor sources are not implemented. See
[Task Registration](task-registration.md) for the existing registration APIs.

## Empty Results And Errors

### Empty Registries

An empty registry is a successful result, with exit status `0`:

| Command | Output when no tasks are registered |
| --- | --- |
| `task list` or `task list --output table` | `No registered tasks found.` |
| `task list --output json` | An empty array: `[]` |
| `task list --output yaml` | An empty sequence: `[]` |
| `task list --output markdown` | The summary table header and separator, with no task rows |
| `task docs` | A `Task Catalogue` heading followed by `No registered tasks found.` |

The generic installed `genja` binary may have no project task registrations.
Changing directories does not change which tasks are linked into it.

### Invalid And Missing Identities

`task describe` requires exactly one `@` separator, a non-empty task ID without
leading or trailing whitespace, and a semantic version. A missing version does
not select the latest version. Lookup matches the listed ID and version exactly,
including generated IDs with `auto:` prefixes, `::` separators, and uppercase
type names.

For example, omitting `@<version>`:

```bash
my_project_cli task describe acme.examples.backup_config
```

produces this error on stderr and exits with status `1`:

```text
error: invalid task identity `acme.examples.backup_config`: identity must contain exactly one `@` separator
```

A valid identity with no matching descriptor also exits with status `1`:

```text
error: task descriptor `acme.examples.missing@1.0.0` was not found
```

Use `my_project_cli task list` to check the available IDs and versions. An ID-only
lookup is not supported by this command, even though the shared error model can
represent ambiguous versions for other discovery consumers.

### Output Streams And Exit Status

Successful command output, including help and version information, goes to
stdout. Errors go to stderr as human-readable text, even when `--output json`
or `--output yaml` is selected; errors are not serialized descriptors.

| Outcome | Exit status |
| --- | --- |
| Successful discovery, including empty lists, or requested help/version | `0` |
| Invalid identity, missing descriptor, discovery source failure, or serialization failure | `1` |
| Invalid command arguments, such as an omitted identity argument or unsupported output format | `2` |

For example, `my_project_cli task list --output xml` is rejected with an argument
error listing the supported formats. `task docs` accepts only `markdown`.
Discovery source failures include the backend's reason, such as a registry
registration conflict. Command handlers complete discovery and rendering before
printing results, so these discovery and serialization failures do not emit a
partial descriptor document on stdout.

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
