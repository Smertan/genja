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

    Install the Python library alone:

    ```bash
    python -m pip install genja-py
    ```

    Starting with v0.5.0, install the optional CLI and TUI in the same
    environment:

    ```bash
    python -m pip install "genja-py[cli]>=0.5.0,<0.6.0"
    ```

    The extra depends on a compatible prebuilt `genja-cli` v0.5.x wheel. The
    base `genja-py` installation remains library-only.

    The Python distribution is named `genja-py`, but the import name is
    `genja`:

    ```python
    import genja as genja_lib
    ```

## Python Task Discovery From The CLI

`genja-py` supplies Python task registration and the discovery helper. The
`genja` executable comes from a separate `genja-cli` Python distribution that
packages the existing Rust binary with TUI support. The Rust crate and PyPI
distribution share a name, while the installed command is named `genja`.

For a Python-only project on v0.5.0 or later, activate the environment
containing your task modules, then install both packages with one command:

```bash
python -m pip install "genja-py[cli]>=0.5.0,<0.6.0"
```

The v0.5.0 wheel release covers CPython 3.10–3.14 on Linux x86_64,
macOS Apple Silicon and Intel, and Windows x64. On another platform, the CLI
extra has no compatible binary wheel and installation fails rather than
building the CLI from source. Base `genja-py` may still offer a source build,
which requires a Rust toolchain. The Intel macOS wheels target macOS 10.12;
CI installs them on macOS 15, so runtime compatibility with older releases
still needs verification.

Cargo-built CLI binaries use `python` from `PATH` by default. A CLI installed
from a Python wheel uses the interpreter beside its executable. Pass
`--python PATH` to select another environment containing `genja-py` and your
task modules.

For a first project, save this configuration as `pyproject.toml`:

```toml
[tool.genja.tasks]
modules = ["tasks"]
```

In the same directory, save this registered task as `tasks.py`:

```python
from genja.task import TaskRegistration, TaskSuccessResult, task


@task(
    name="hello",
    registration=TaskRegistration(
        id="example.hello",
        version="1.0.0",
        description="Greets a host",
    ),
)
class Hello:
    def start(self, task, host, context):
        return TaskSuccessResult(summary=f"Hello, {host.hostname}")
```

From that directory, Genja imports `tasks.py` because the module is declared
in `pyproject.toml`; the decorator registers the task during import:

```bash
genja --help
genja task list
genja task describe example.hello@1.0.0
genja tui
```

`genja tui` needs an interactive terminal. The wheel-installed binary includes
TUI support. A generic CLI binary is enough for Python-only projects. Mixed
projects use a project-local binary that links their Rust task crate and calls
`genja_cli::run_main()`; it also
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
