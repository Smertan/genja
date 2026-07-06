# Installation

Install the Genja package for the language you are using.

=== ":fontawesome-brands-rust: Rust"

    ```bash
    cargo add genja
    ```

    Or add it to `Cargo.toml`:

    ```toml
    [dependencies]
    genja = "0.2.0"
    ```

    The `genja` crate pulls in the Rust crates required for the public Genja
    API, including `genja-core`, `genja-core-derive`, and
    `genja-plugin-manager`.

=== ":fontawesome-brands-python: Python"

    ```bash
    pip install genja-py
    ```

    The Python distribution is named `genja-py`, but the import name is
    `genja`:

    ```python
    import genja as genja_lib
    ```

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
