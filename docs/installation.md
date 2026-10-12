# Installation

Install the Genja package for the language you are using.

=== ":fontawesome-brands-rust: Rust"

    ```bash
    cargo add genja
    ```

    Or add it to `Cargo.toml`:

    ```toml
    [dependencies]
    genja = "0.3.0"
    ```

    The `genja` crate pulls in the Rust crates required for the public Genja
    API, including `genja-core`, `genja-core-derive`, and
    `genja-plugin-manager`.

    ### Optional Features

    | Feature | Enabled by default | Provides |
    | --- | --- | --- |
    | `genja-cli` | No | CLI entry points, task discovery, and rendering through `genja::cli`. |

    The `genja-cli` feature is currently unreleased and planned for v0.5.0.
    Once a release containing it is published, enable it with:

    ```bash
    cargo add genja@0.5.0 --features genja-cli
    ```

    Or, once v0.5.0 is published, configure the dependency in `Cargo.toml`:

    ```toml
    [dependencies]
    genja = { version = "0.5.0", features = ["genja-cli"] }
    ```

    If you already depend on `genja`, update its existing entry rather than
    adding a second one.

    Cargo automatically resolves `genja-cli` using the dependency requirement
    declared by your selected `genja` version; you do not need to select a CLI
    version separately. The unified release train aligns these requirements.
    A `0.5.0` requirement allows compatible `0.5.x` patch releases, so the exact
    patch versions may differ. `Cargo.lock` records the resolved versions;
    exact version matching is not currently enforced.

    Enabling the feature adds CLI library code to your dependencies. It does
    not install an executable; provide a project-specific binary that links
    your tasks and calls `genja::cli::run_main()`.

    See the [CLI guide](cli.md#through-the-genja-feature) for setup from a
    checkout before publication, binary wiring, the direct `genja-cli`
    dependency option, and deployment instructions.

=== ":fontawesome-brands-python: Python"

    ```bash
    pip install genja-py
    ```

    The Python distribution is named `genja-py`, but the import name is
    `genja`:

    ```python
    import genja as genja_lib
    ```

## Python Task Discovery From The CLI

`genja-py` supplies Python task registration and the discovery helper. The
`genja` executable comes from the separate Rust `genja-cli` crate; installing
`genja-py` alone does not install that executable. There is currently no
`genja-py[cli]` extra that bundles it.

Install `genja-py` in the Python environment that contains your task modules:

```bash
python -m pip install genja-py
```

Until `genja-cli` is published, install its binary from a Genja checkout:

```bash
cargo install --path genja-cli --features tui
```

Omit `--features tui` if you only need `task list`, `task describe`, and
`task docs`. The `tui` feature enables `genja tui` in the installed binary.
The CLI uses `python` from `PATH` by default; pass `--python PATH` to select
the environment where `genja-py` and your task modules are installed.

Declare Python task modules in your project's `pyproject.toml`:

```toml
[tool.genja.tasks]
modules = ["my_project.tasks"]
```

From that project directory, `genja task list`, `genja task describe`, and
`genja task docs` discover those modules. A binary built with `tui` also
supports `genja tui`. A generic CLI binary is enough for Python-only projects.
Mixed projects use a project-local binary
that links their Rust task crate and calls `genja_cli::run_main()`; it also
imports configured Python modules through the selected interpreter. See the
[CLI guide](cli.md#project-local-cli-binary-rust-and-mixed-projects) and
[mixed discovery example](examples.md#mixed-rust-and-python-discovery).

## Verify

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::Settings;

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let settings = Settings::default();
        println!("Runner plugin: {}", settings.runner().plugin());
        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    settings = genja_lib.Settings()
    print(f"Runner plugin: {settings.runner.plugin}")
    ```

## Examples

Cargo and pip installations include the library package, not the repository's
example source files. To run the examples, clone the repository and run them
from the checkout:

```bash
git clone https://github.com/Smertan/genja.git
cd genja
```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    cargo run -p genja --example basic_runtime
    cargo run -p genja --example filter_hosts
    cargo run -p genja --example run_task
    cargo run -p genja --example run_task_tree
    ```

=== ":fontawesome-brands-python: Python"

    Python examples are available under `genja/examples/python`.
