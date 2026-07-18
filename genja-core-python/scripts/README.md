# genja-core-python scripts

Utility scripts for local development and CI checks.

## `check_python_api_docs.py`

Checks documented Python API stubs and selected Rust/PyO3 bindings.

Run through PDM from `genja-core-python`:

```bash
pdm run check-stubs
```

The check is intentionally scoped to API surfaces that have already been brought
up to the repository's stub/docstring standard. It currently verifies:

- public class and method docstrings in selected `.pyi` files;
- structural parity for Rust-backed classes duplicated between `genja.pyi` and
  the top-level `__init__.pyi` re-export surface;
- Rust doc comments on documented PyO3 settings bindings.

When improving another stub file, add it to `STUBS_REQUIRING_DOCSTRINGS` in
`check_python_api_docs.py`. When bringing another duplicated top-level
Rust-backed class under the standard, add it to `DUPLICATED_TOP_LEVEL_CLASSES`.
When documenting another Rust/PyO3 binding source file, extend the script so CI
prevents regressions for that file too.

## `test_rust.py`

Runs the Rust-backed PyO3 test suite through the active Python environment.

Run through PDM from `genja-core-python`:

```bash
pdm run test-rust
```

The script sets `PYO3_PYTHON` to the current Python interpreter before invoking
Cargo so Rust tests embed the same Python environment used by the package.
