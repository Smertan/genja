# Command Line Interface

Genja provides a first-party CLI crate, `genja-cli`, for listing, describing,
and documenting registered task descriptors. It discovers compiled Rust tasks
and configured Python task modules from the selected project file.

## Choose Your Project Setup

Select the workflow that matches your project. The task commands, output
formats, identity rules, and errors in the [command reference](#command-reference-all-projects)
apply to all three.

=== ":fontawesome-brands-python: Python"

    Starting with v0.5.0, install both `genja-py` and the prebuilt `genja`
    command in the Python environment containing your task modules:

    ```bash
    python -m pip install "genja-py[cli]>=0.5.0,<0.6.0"
    ```

    Python discovery requires v0.5.0 or later of both packages. See
    [Installation](installation.md#python-task-discovery-from-the-cli) for
    supported wheel platforms.

    Declare the modules containing your `@task` classes in `pyproject.toml`:

    ```toml
    [tool.genja.tasks]
    modules = ["my_project.tasks.network", "my_project.tasks.backups"]
    ```

    Genja imports these modules during discovery, so task authors do not need
    an application import or a separate registry call. From the project
    directory, run:

    ```bash
    genja task list
    genja task describe acme.examples.backup_config@1.0.0
    genja task docs
    ```

    The identity above comes from the
    [Python example](examples.md#mixed-rust-and-python-discovery); use an
    identity from your own `task list` output. `genja --help` lists commands,
    and `genja version` prints the CLI version.

    The CLI reads `pyproject.toml` in the current directory. A `genja` binary
    installed from a Python wheel uses the interpreter beside it; a Cargo-built
    binary uses `python` from `PATH`. Pass a path to `--pyproject` to select
    another project file, or to `--python` to select another interpreter. See
    [Task Registration](task-registration.md#python-import-boundary) for the
    import boundary.

=== ":fontawesome-brands-rust: Rust"

    Build a [project-local CLI binary](#project-local-cli-binary-rust-and-mixed-projects)
    that links your Rust task registrations. The generic installed `genja`
    binary cannot see tasks linked only into your project binary.

    ```bash
    my_project_cli task list
    my_project_cli task describe acme.examples.backup_config@1.0.0
    my_project_cli task docs
    ```

    Without `[tool.genja.tasks]`, discovery uses the linked Rust tasks and
    does not start Python.

=== "Mixed Rust and Python"

    Build a [project-local CLI binary](#project-local-cli-binary-rust-and-mixed-projects)
    that links your Rust task crate. Install `genja-py` alongside your Python
    task modules in the selected interpreter and declare those modules in the
    project's `pyproject.toml`:

    ```toml
    [tool.genja.tasks]
    modules = ["my_project.tasks.network", "my_project.tasks.backups"]
    ```

    The same binary combines linked Rust tasks with descriptors from the
    imported Python modules:

    ```bash
    my_project_cli task list
    my_project_cli task describe python:acme.examples.backup_config@1.0.0
    my_project_cli task docs
    ```

    The qualified identity is from the
    [mixed example](examples.md#mixed-rust-and-python-discovery); use an
    identity from your own list. The CLI uses `pyproject.toml` in the current
    directory and `python` from `PATH` by default. Pass a path to
    `--pyproject` or `--python` to override either choice. Configured Python
    failures stop discovery without printing partial Rust results.

## Terminal User Interface (TUI)

Genja is developing an interactive task browser for the terminal. A binary
built with the `tui` feature can browse configured Python tasks and linked Rust
tasks in one screen. Rust applications can also embed the browser. Press `q` or
Escape to exit. Builds without the feature omit the command. See
the [Terminal UI guide](tui.md) for setup, usage, and current capabilities.

## Command Reference (All Projects)

The tabs below use `genja` for Python-only projects and `my_project_cli` for
Rust or mixed projects. Commands and flags are otherwise the same. See
[Project-Local CLI Binary](#project-local-cli-binary-rust-and-mixed-projects)
for Rust and mixed setup instructions.
The example task identities are illustrative; use the ID and version returned
by your binary's `task list` command.

### Help And Version

=== ":fontawesome-brands-python: Python"

    ```bash
    genja --help
    genja version
    genja task --help
    genja task list --help
    genja task describe --help
    genja task docs --help
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli --help
    my_project_cli version
    my_project_cli task --help
    my_project_cli task list --help
    my_project_cli task describe --help
    my_project_cli task docs --help
    ```

=== "Mixed Rust and Python"

    ```bash
    my_project_cli --help
    my_project_cli version
    my_project_cli task --help
    my_project_cli task list --help
    my_project_cli task describe --help
    my_project_cli task docs --help
    ```

`version` prints the linked Genja CLI version, not your project's package version.

### Terminal Task Browser

With the `tui` feature on `genja-cli`, or `genja-tui` on `genja`, the generic
`genja` binary or a project-local binary can launch the terminal browser:

=== ":fontawesome-brands-python: Python"

    ```bash
    genja tui --help
    genja tui
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli tui --help
    my_project_cli tui
    ```

=== "Mixed Rust and Python"

    ```bash
    my_project_cli tui --help
    my_project_cli tui
    ```

Run it in an interactive terminal and press `q` or Escape to exit. The screen
shows one searchable task table with Rust and Python language icons. Enter opens
Details and Tab switches to Schema inspection. Task execution is not implemented.
Builds without TUI support omit this command. See the
[Terminal UI guide](tui.md) for setup and terminal behaviour.

### Try The Sample Tasks (Rust Checkout)

**These commands require a local checkout of the Genja GitHub repository.**
Run them from that checkout's root, where its workspace `Cargo.toml` lives.
Running `cargo add genja` in your own project does not make Genja's example
executables available through `cargo run --example`. See
[Examples](examples.md) for checkout instructions.

In the checkout, run `task_browser` to explore two sample tasks without
creating your own project-local binary:

```bash
cargo run -p genja --features genja-cli --example task_browser -- task list
cargo run -p genja --features genja-cli --example task_browser -- task describe acme.examples.backup_config@1.0.0
cargo run -p genja --features genja-tui --example task_browser -- tui
```

The example links a blocking, constructible `backup_config` task and an async,
descriptor-only `collect_facts` task with a generated local ID. To describe the
second task, use its ID and version from `task list` in `<id>@<version>` form.
Add `--output json` to a list or describe command to inspect the descriptor data.

The sample implementations do not perform backups or connect to hosts.
Registrations belong only to the example executable. The TUI currently loads
both descriptors and displays them in a navigable, searchable table. Press `/`
to search and Enter to return to navigation. See the
[Rust tab of the TUI search walkthrough](tui.md#try-search) for matching and
clearing examples, or [CLI And TUI Task Browser](examples.md#cli-and-tui-task-browser)
for the sample binary's CLI commands.

### List Tasks

=== ":fontawesome-brands-python: Python"

    ```bash
    genja task list
    genja task list --output table
    genja task list --output json
    genja task list --output yaml
    genja task list --output markdown
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli task list
    my_project_cli task list --output table
    my_project_cli task list --output json
    my_project_cli task list --output yaml
    my_project_cli task list --output markdown
    ```

=== "Mixed Rust and Python"

    ```bash
    my_project_cli task list
    my_project_cli task list --output table
    my_project_cli task list --output json
    my_project_cli task list --output yaml
    my_project_cli task list --output markdown
    ```

The default format is `table`. Table and Markdown lists are compact summaries
containing task ID, version, name, ID source, implementation language, execution mode, and
constructibility. They omit descriptions, connection plugins, processors,
retry metadata, and input schemas.

**Language** is assigned by discovery (`Rust` or `Python`) and is separate from
**Source**, which says whether the descriptor ID was generated or explicit.
Custom descriptor sources without language metadata show `Unknown`.

JSON and YAML lists include one `{ descriptor, language }` record per task
implementation. The nested descriptor contains all canonical fields. Use
`task describe` to inspect one implementation or `task docs` to document all tasks.

#### What Does Constructible Mean?

`CONSTRUCTIBLE` reports whether the task registration includes a factory that
can create the task from JSON input in its source language. A factory turns
input into a task instance; it does not run the task.

- `yes` means a construction factory is registered.
- `no` means the task can be discovered and inspected, but has no registered
  factory. Creation by task identity and JSON input is unavailable until the
  author registers one. Application code may still create the task directly.

JSON and YAML output express the same field as `descriptor.constructible: true`
or `descriptor.constructible: false`. A `yes` value does not guarantee valid input or successful
execution. See [Constructible Descriptor Field](task-registration.md#constructible-descriptor-field)
for how Rust task structs and registration determine this value.

### Describe A Task

Pass an identity in `<task-id>@<task-version>` form:

=== ":fontawesome-brands-python: Python"

    ```bash
    genja task describe acme.examples.backup_config@1.0.0
    genja task describe acme.examples.backup_config@1.0.0 --output table
    genja task describe acme.examples.backup_config@1.0.0 --output json
    genja task describe acme.examples.backup_config@1.0.0 --output yaml
    genja task describe acme.examples.backup_config@1.0.0 --output markdown
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli task describe acme.examples.backup_config@1.0.0
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output table
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output json
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output yaml
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output markdown
    ```

=== "Mixed Rust and Python"

    ```bash
    my_project_cli task describe rust:acme.examples.backup_config@1.0.0
    my_project_cli task describe python:acme.examples.backup_config@1.0.0
    my_project_cli task describe python:acme.examples.backup_config@1.0.0 --output json
    my_project_cli task describe python:acme.examples.backup_config@1.0.0 --output yaml
    my_project_cli task describe python:acme.examples.backup_config@1.0.0 --output markdown
    ```

Use the exact ID and version shown in the list. Explicit IDs, such as
`acme.examples.backup_config`, and generated IDs, such as
`auto:my_project::tasks::BackupTask`, can both be described. Preserve the
`auto:` prefix, `::` separators, and capitalization in generated Rust IDs:

```bash
my_project_cli task describe auto:my_project::tasks::BackupTask@0.1.0
```

If Rust and Python provide the same ID and version, prefix the identity with
`rust:` or `python:` to select one implementation, as shown in the Mixed tab.

An unqualified identity works when only one implementation matches. For a
generated Rust ID, the language prefix comes before `auto:`, as in
`rust:auto:my_project::tasks::BackupTask@0.1.0`. Explicit task IDs cannot
contain `:`, and custom sources should avoid IDs beginning `rust:` or `python:`
because those prefixes are reserved for CLI selection.

The default table view displays ID, version, name, description, ID source, language,
execution mode, connection plugin, processors, constructibility, retry metadata,
and input schema availability. Missing optional metadata is shown as `-`.
Retry metadata is shown as compact JSON when present.

When an input schema is available, the table view includes pretty-printed JSON
below the field summary. Markdown includes the same metadata and a fenced JSON
schema block. JSON and YAML include the schema within their nested descriptor.

### Output Formats

`task list` and `task describe` accept these lowercase `--output` values:

| Format | `task list` | `task describe` |
| --- | --- | --- |
| `table` (default) | Compact task summary rows | Field/value summary with input schema below when available |
| `json` | Array of implementation records | One implementation record |
| `yaml` | Sequence of implementation records | One implementation record |
| `markdown` | Compact task summary table | Task heading, metadata table, and input schema block when available |

JSON and YAML wrap the existing `TaskDescriptor` serialization contract as
`{ descriptor, language }`. The descriptor retains optional metadata and input
schemas without a language field. The outer `language` is `rust`, `python`, or
`null` when a custom source cannot identify it. Read descriptor fields through
the `descriptor` key, for example `descriptor.id`. Markdown output is raw
Markdown text, not a rendered terminal view.

### Generate A Task Catalogue

=== ":fontawesome-brands-python: Python"

    ```bash
    genja task docs
    genja task docs --output markdown
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli task docs
    my_project_cli task docs --output markdown
    ```

=== "Mixed Rust and Python"

    ```bash
    my_project_cli task docs
    my_project_cli task docs --output markdown
    ```

`task docs` supports Markdown only, and uses it by default. For registered tasks,
it generates a document with a `Task Catalogue` heading, an index linking to the
summary and individual tasks, a summary table, and detailed per-task sections.
Each task section includes descriptor metadata and a fenced JSON input schema
when available. Index links include the language qualifier and point to distinct
sections when Rust and Python share an ID and version.

Unlike the compact `task list --output markdown` table, the catalogue summary
also includes descriptions. Use this command for complete task documentation.

## Output Examples

Select a project setup to see output from the checkout fixtures. The Python and
Rust tabs cover every output format. The mixed tab shows the combined list and
language-qualified lookup when both implementations share an ID and version.

=== ":fontawesome-brands-python: Python"

    These examples use the [configured Python task module](examples.md#mixed-rust-and-python-discovery)
    in `genja/examples/python`. Install `genja-py` in the selected Python
    environment and the `genja` binary from `genja-cli`, then run from the
    repository root. If that environment's interpreter is not `python` on `PATH`,
    pass its path with `--python`.

    /// details | Task list (table)
        type: example

    ```bash
    genja --pyproject genja/examples/python/pyproject.toml task list
    ```

    ```text
     ID                          | VERSION | NAME          | SOURCE   | LANGUAGE | MODE     | CONSTRUCTIBLE
    ========================================================================================================
     acme.examples.backup_config | 1.0.0   | backup_config | explicit | Python   | blocking | yes
    -----------------------------+---------+---------------+----------+----------+----------+---------------
     acme.examples.collect_facts | 1.0.0   | collect_facts | explicit | Python   | blocking | yes
    ```
    ///

    /// details | Task list (raw Markdown)
        type: example

    ```bash
    genja --pyproject genja/examples/python/pyproject.toml task list --output markdown
    ```

    ```markdown
    | ID                            | Version | Name            | Source     | Language | Mode       | Constructible |
    |-------------------------------|---------|-----------------|------------|----------|------------|---------------|
    | `acme.examples.backup_config` | `1.0.0` | `backup_config` | `explicit` | `Python` | `blocking` | yes           |
    | `acme.examples.collect_facts` | `1.0.0` | `collect_facts` | `explicit` | `Python` | `blocking` | yes           |
    ```
    ///

    /// details | Task details and input schema (table)
        type: example

    ```bash
    genja --pyproject genja/examples/python/pyproject.toml task describe acme.examples.backup_config@1.0.0
    ```

    ```text
    Task: python:acme.examples.backup_config@1.0.0

     Field             | Value
    ===================================================================
     ID                | acme.examples.backup_config
    -------------------+-----------------------------------------------
     Version           | 1.0.0
    -------------------+-----------------------------------------------
     Name              | backup_config
    -------------------+-----------------------------------------------
     Description       | Backs up selected paths from a network device
    -------------------+-----------------------------------------------
     ID source         | explicit
    -------------------+-----------------------------------------------
     Language          | Python
    -------------------+-----------------------------------------------
     Execution mode    | blocking
    -------------------+-----------------------------------------------
     Connection plugin | ssh
    -------------------+-----------------------------------------------
     Processors        | -
    -------------------+-----------------------------------------------
     Constructible     | yes
    -------------------+-----------------------------------------------
     Retry             | -
    -------------------+-----------------------------------------------
     Input schema      | available

    Input schema:
    {
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
      "type": "object"
    }
    ```
    ///

    /// details | Implementation record (JSON)
        type: example

    ```bash
    genja --pyproject genja/examples/python/pyproject.toml task describe acme.examples.backup_config@1.0.0 --output json
    ```

    ```json
    {
      "descriptor": {
        "id": "acme.examples.backup_config",
        "id_source": "explicit",
        "name": "backup_config",
        "version": "1.0.0",
        "description": "Backs up selected paths from a network device",
        "execution_mode": "blocking",
        "connection_plugin_name": "ssh",
        "processor_names": [],
        "retry": null,
        "input_schema": {
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
          "type": "object"
        },
        "constructible": true
      },
      "language": "python"
    }
    ```
    ///

    /// details | Implementation record (YAML)
        type: example

    ```bash
    genja --pyproject genja/examples/python/pyproject.toml task describe acme.examples.backup_config@1.0.0 --output yaml
    ```

    ```yaml
    descriptor:
      id: acme.examples.backup_config
      id_source: explicit
      name: backup_config
      version: 1.0.0
      description: Backs up selected paths from a network device
      execution_mode: blocking
      connection_plugin_name: ssh
      processor_names: []
      retry: null
      input_schema:
        properties:
          backup_path:
            type: string
          compress:
            type: boolean
        required:
        - backup_path
        - compress
        type: object
      constructible: true
    language: python
    ```
    ///

    /// details | Complete two-task catalogue (raw Markdown)
        type: example

    ```bash
    genja --pyproject genja/examples/python/pyproject.toml task docs
    ```

    ````markdown
    # Task Catalogue

    ## Index

    - [Summary](#summary)
    - [Tasks](#tasks)
      - [`python:acme.examples.backup_config@1.0.0`](#task-acme-examples-backup-config-1-0-0-python-1)
      - [`python:acme.examples.collect_facts@1.0.0`](#task-acme-examples-collect-facts-1-0-0-python-2)

    ## Summary

    | ID                            | Version | Name            | Source     | Language | Mode       | Constructible | Description                                   |
    |-------------------------------|---------|-----------------|------------|----------|------------|---------------|-----------------------------------------------|
    | `acme.examples.backup_config` | `1.0.0` | `backup_config` | `explicit` | `Python` | `blocking` | yes           | Backs up selected paths from a network device |
    | `acme.examples.collect_facts` | `1.0.0` | `collect_facts` | `explicit` | `Python` | `blocking` | yes           | Collects basic host facts                     |

    ## Tasks

    <a id="task-acme-examples-backup-config-1-0-0-python-1"></a>

    ### python:acme.examples.backup_config@1.0.0

    | Field             | Value                                         |
    |-------------------|-----------------------------------------------|
    | ID                | `acme.examples.backup_config`                 |
    | Version           | `1.0.0`                                       |
    | Name              | `backup_config`                               |
    | Description       | Backs up selected paths from a network device |
    | ID source         | `explicit`                                    |
    | Language          | `Python`                                      |
    | Execution mode    | `blocking`                                    |
    | Connection plugin | `ssh`                                         |
    | Processors        | -                                             |
    | Constructible     | yes                                           |
    | Retry             | -                                             |
    | Input schema      | available                                     |

    #### Input Schema

    ```json
    {
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
      "type": "object"
    }
    ```

    <a id="task-acme-examples-collect-facts-1-0-0-python-2"></a>

    ### python:acme.examples.collect_facts@1.0.0

    | Field             | Value                         |
    |-------------------|-------------------------------|
    | ID                | `acme.examples.collect_facts` |
    | Version           | `1.0.0`                       |
    | Name              | `collect_facts`               |
    | Description       | Collects basic host facts     |
    | ID source         | `explicit`                    |
    | Language          | `Python`                      |
    | Execution mode    | `blocking`                    |
    | Connection plugin | -                             |
    | Processors        | -                             |
    | Constructible     | yes                           |
    | Retry             | -                             |
    | Input schema      | -                             |
    ````
    ///

=== ":fontawesome-brands-rust: Rust"

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
    fixture. JSON and YAML list commands wrap the implementation record in an array
    or sequence.

    /// details | Compact task list (table)
        type: example

    ```bash
    my_project_cli task list
    ```

    ```text
     ID                          | VERSION | NAME          | SOURCE   | LANGUAGE | MODE  | CONSTRUCTIBLE
    =====================================================================================================
     acme.examples.backup_config | 1.0.0   | backup_config | explicit | Rust     | async | yes
    ```
    ///

    /// details | Compact task list (raw Markdown)
        type: example

    ```bash
    my_project_cli task list --output markdown
    ```

    ```markdown
    | ID                            | Version | Name            | Source     | Language | Mode    | Constructible |
    |-------------------------------|---------|-----------------|------------|----------|---------|---------------|
    | `acme.examples.backup_config` | `1.0.0` | `backup_config` | `explicit` | `Rust`   | `async` | yes           |
    ```
    ///

    /// details | Task details and input schema (table)
        type: example

    ```bash
    my_project_cli task describe acme.examples.backup_config@1.0.0
    ```

    ```text
    Task: rust:acme.examples.backup_config@1.0.0

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
     Language          | Rust
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

    /// details | Implementation record (JSON)
        type: example

    ```bash
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output json
    ```

    ```json
    {
      "descriptor": {
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
      },
      "language": "rust"
    }
    ```
    ///

    /// details | Implementation record (YAML)
        type: example

    ```bash
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output yaml
    ```

    ```yaml
    descriptor:
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
    language: rust
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
      - [`rust:acme.examples.backup_config@1.0.0`](#task-acme-examples-backup-config-1-0-0-rust-1)

    ## Summary

    | ID                            | Version | Name            | Source     | Language | Mode    | Constructible | Description                                   |
    |-------------------------------|---------|-----------------|------------|----------|---------|---------------|-----------------------------------------------|
    | `acme.examples.backup_config` | `1.0.0` | `backup_config` | `explicit` | `Rust`   | `async` | yes           | Backs up selected paths from a network device |

    ## Tasks

    <a id="task-acme-examples-backup-config-1-0-0-rust-1"></a>

    ### rust:acme.examples.backup_config@1.0.0

    | Field             | Value                                            |
    |-------------------|--------------------------------------------------|
    | ID                | `acme.examples.backup_config`                    |
    | Version           | `1.0.0`                                          |
    | Name              | `backup_config`                                  |
    | Description       | Backs up selected paths from a network device    |
    | ID source         | `explicit`                                       |
    | Language          | `Rust`                                           |
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

=== "Mixed Rust and Python"

    The [task browser example](examples.md#mixed-rust-and-python-discovery)
    links Rust tasks and loads the configured Python module. The command below
    shows an in-project PDM virtual environment on Linux/macOS. Install
    `genja-py` in the selected Python environment first. Replace the
    `--python` path with that environment's interpreter if you use another
    package manager, a different environment name such as `venv`, or a
    different operating system. For PDM, run `pdm info --python` from
    `genja-core-python` to find the path.

    The Rust `CollectFacts` task in `genja/examples/task_browser.rs` has no
    explicit registration. Its generated task version comes from the `version`
    field in `genja/Cargo.toml`. The output below shows `0.5.0` for the
    v0.5.0 release.

    /// details | Combined task list (table)
        type: example

    ```bash
    cargo run -p genja --features genja-cli --example task_browser -- \
      --python genja-core-python/.venv/bin/python \
      --pyproject genja/examples/python/pyproject.toml task list
    ```

    ```text
     ID                              | VERSION | NAME          | SOURCE    | LANGUAGE | MODE     | CONSTRUCTIBLE
    =============================================================================================================
     acme.examples.backup_config     | 1.0.0   | backup_config | explicit  | Rust     | blocking | yes
    ---------------------------------+---------+---------------+-----------+----------+----------+---------------
     acme.examples.backup_config     | 1.0.0   | backup_config | explicit  | Python   | blocking | yes
    ---------------------------------+---------+---------------+-----------+----------+----------+---------------
     acme.examples.collect_facts     | 1.0.0   | collect_facts | explicit  | Python   | blocking | yes
    ---------------------------------+---------+---------------+-----------+----------+----------+---------------
     auto:task_browser::CollectFacts | 0.5.0   | collect_facts | generated | Rust     | async    | no
    ```
    ///

    /// details | Combined task list (raw Markdown)
        type: example

    ```bash
    cargo run -p genja --features genja-cli --example task_browser -- \
      --python genja-core-python/.venv/bin/python \
      --pyproject genja/examples/python/pyproject.toml task list --output markdown
    ```

    ```markdown
    | ID                                | Version | Name            | Source      | Language | Mode       | Constructible |
    |-----------------------------------|---------|-----------------|-------------|----------|------------|---------------|
    | `acme.examples.backup_config`     | `1.0.0` | `backup_config` | `explicit`  | `Rust`   | `blocking` | yes           |
    | `acme.examples.backup_config`     | `1.0.0` | `backup_config` | `explicit`  | `Python` | `blocking` | yes           |
    | `acme.examples.collect_facts`     | `1.0.0` | `collect_facts` | `explicit`  | `Python` | `blocking` | yes           |
    | `auto:task_browser::CollectFacts` | `0.5.0` | `collect_facts` | `generated` | `Rust`   | `async`    | no            |
    ```
    ///

    /// details | Selecting a shared identity
        type: example

    An unqualified lookup fails because both languages provide the identity:

    ```bash
    my_project_cli task describe acme.examples.backup_config@1.0.0
    ```

    ```text
    error: task descriptor `acme.examples.backup_config@1.0.0` has implementations in multiple languages: Rust, Python; use `rust:acme.examples.backup_config@1.0.0` or `python:acme.examples.backup_config@1.0.0`
    ```

    Use the language prefix to select one implementation:

    ```bash
    my_project_cli task describe rust:acme.examples.backup_config@1.0.0
    my_project_cli task describe python:acme.examples.backup_config@1.0.0
    ```
    ///


## Exporting Task Descriptors (All Projects)

Commands write results to stdout. Use your shell's `>` redirection to save them
to a file; Genja does not provide an output-file option. Select the tab matching
your project; each example assumes its executable is on `PATH`.

### JSON And YAML

Export implementation records, each with a complete descriptor and its
language, for automation or later processing:

=== ":fontawesome-brands-python: Python"

    ```bash
    genja task list --output json > tasks.json
    genja task list --output yaml > tasks.yaml
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli task list --output json > tasks.json
    my_project_cli task list --output yaml > tasks.yaml
    ```

=== "Mixed Rust and Python"

    ```bash
    my_project_cli task list --output json > tasks.json
    my_project_cli task list --output yaml > tasks.yaml
    ```

Export one implementation record, including any input schema in its descriptor,
using its listed identity:

=== ":fontawesome-brands-python: Python"

    ```bash
    genja task describe acme.examples.backup_config@1.0.0 --output json > backup-config.json
    genja task describe acme.examples.backup_config@1.0.0 --output yaml > backup-config.yaml
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output json > backup-config.json
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output yaml > backup-config.yaml
    ```

=== "Mixed Rust and Python"

    ```bash
    my_project_cli task describe python:acme.examples.backup_config@1.0.0 --output json > backup-config.json
    my_project_cli task describe python:acme.examples.backup_config@1.0.0 --output yaml > backup-config.yaml
    ```

### Markdown Documents

Choose the command according to how much detail the document needs:

| Command | Document contents |
| --- | --- |
| `task list --output markdown` | Compact summary table without descriptions or input schemas |
| `task describe <identity> --output markdown` | One task's metadata and input schema, when available |
| `task docs` | Catalogue index, summary including descriptions, and detailed sections for all tasks |

=== ":fontawesome-brands-python: Python"

    ```bash
    genja task list --output markdown > task-summary.md
    genja task describe acme.examples.backup_config@1.0.0 --output markdown > backup-config.md
    genja task docs --output markdown > task-catalog.md
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli task list --output markdown > task-summary.md
    my_project_cli task describe acme.examples.backup_config@1.0.0 --output markdown > backup-config.md
    my_project_cli task docs --output markdown > task-catalog.md
    ```

=== "Mixed Rust and Python"

    ```bash
    my_project_cli task list --output markdown > task-summary.md
    my_project_cli task describe python:acme.examples.backup_config@1.0.0 --output markdown > backup-config.md
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

For compiled Rust tasks, rebuild the project binary after changing task
registrations, then regenerate the files. Python task modules are imported on
each discovery command.

## Discovery Scope And Architecture (All Projects)

The CLI reads Rust registrations linked into the running executable and Python
registrations from modules declared in `[tool.genja.tasks]` in the selected
`pyproject.toml`. It does not scan directories, build another project, or load
another executable's Rust registry. If expected Rust tasks are missing, check
that the project-specific binary links the task crate or modules.

Commands consume the shared `TaskDescriptorSource` abstraction from
`genja_cli::discovery`. It returns `TaskImplementation` records that pair each
canonical `TaskDescriptor` with source-assigned language metadata, or a common
`DiscoveryError`. The compiled source is backed by the registry in
`genja-core`; the combined source adds configured Python descriptors through
the `genja-py` helper. Existing custom sources that only return descriptors
remain compatible and have unknown language.

The command handlers pass implementations to separate output renderers. The
combined source orders them deterministically by ID, version, and language.
This boundary allows future descriptor sources to reuse the command and
rendering layers.

These commands inspect metadata only. A descriptor's `constructible` value
reports its registration capability; listing or describing it does not construct
or execute the task. Task execution is outside the initial CLI discovery scope.

The CLI selects combined discovery when `[tool.genja.tasks]` is present and
compiled Rust discovery otherwise. The Python helper imports the declared
modules and returns descriptor JSON to the combined source. For Python or mixed
projects, `--pyproject` and `--python` work before or after a task subcommand.
Provider manifests and MCP-backed discovery are future work.

## Empty Results And Errors (All Projects)

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

=== ":fontawesome-brands-python: Python"

    ```bash
    genja task describe acme.examples.backup_config
    ```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    my_project_cli task describe acme.examples.backup_config
    ```

=== "Mixed Rust and Python"

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

Use `task list` with your project's executable to check the available IDs and
versions. An ID-only lookup is not supported by this command, even though the
shared error model can represent ambiguous versions for other discovery consumers.

### Output Streams And Exit Status

Successful command output, including help and version information, goes to
stdout. Errors go to stderr as human-readable text, even when `--output json`
or `--output yaml` is selected; errors are not serialized descriptors.

| Outcome | Exit status |
| --- | --- |
| Successful discovery, including empty lists, or requested help/version | `0` |
| Invalid identity, missing descriptor, discovery source failure, or serialization failure | `1` |
| Invalid command arguments, such as an omitted identity argument or unsupported output format | `2` |

For example, `task list --output xml` is rejected with an argument error listing
the supported formats. `task docs` accepts only `markdown`.
Discovery source failures include the backend's reason, such as a registry
registration conflict. Command handlers complete discovery and rendering before
printing results, so these discovery and serialization failures do not emit a
partial descriptor document on stdout.

## Project-Local CLI Binary (Rust And Mixed Projects)

For Rust or mixed projects with compiled task registrations, build a project-specific
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

Both dependency options expose the same CLI library. Rust discovery sees tasks
linked into the running executable; configured Python modules are imported at
runtime. Adding a dependency or enabling the feature does not create a binary
entry point or install the generic `genja` command.
End users receive your project-built executable; they do not need to install
`genja-cli` separately.

## Running Project-Local Discovery (Rust And Mixed Projects)

Run task discovery commands through the project-specific binary:

```bash
my_project_cli task list
my_project_cli task describe auto:my_project::tasks::BackupTask@0.1.0
my_project_cli task docs > task-catalog.md
```

Those commands inspect the Rust registrations linked into `my_project_cli` and,
when `[tool.genja.tasks]` is configured, the declared Python modules. Running
the generic `genja` binary instead would not see your project-linked Rust tasks.

## Development Workflow (Rust And Mixed Projects)

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

## Runtime Deployment (Rust And Mixed Projects)

For runtime deployments, build a project-local CLI binary during release or CI,
then deploy that executable to the target machine. The target machine does not
need Cargo if you deploy a prebuilt binary.
For mixed projects, also deploy the Python task modules and provide a selected
Python environment with `genja-py` installed; the binary imports those modules
when discovery runs.

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

## Lower-Level Entrypoint (Rust API)

`genja::cli::run_main()` (with the feature) and `genja_cli::run_main()` (with a
direct dependency) are the same recommended helper for binary `main` functions.
It reads arguments from the current process and returns a
`std::process::ExitCode`.

For tests or embedded callers that need to provide their own arguments, use the
lower-level `genja::cli::run(args)` or `genja_cli::run(args)` function.
