# genja-core-python scripts

Utility scripts for local development and CI checks.

## `check_python_api_docs.py`

Checks documented Python API stubs and selected Rust/PyO3 bindings.

Run through PDM from `genja-core-python`:

```bash
pdm run check-stubs
```

The check verifies:

- classification of every top-level `python/genja/*.pyi` stub;
- public class and method docstrings in selected `.pyi` files;
- structural parity for Rust-backed classes duplicated between `genja.pyi` and
  the top-level `__init__.pyi` re-export surface;
- Rust doc comments on selected documented PyO3 bindings.

Every top-level `python/genja/*.pyi` stub must be listed in
`STUBS_REQUIRING_DOCSTRINGS` in `check_python_api_docs.py`. When adding a stub,
document its public API and add it to that list during the same change. When
bringing another duplicated top-level Rust-backed class under the standard, add
it to `DUPLICATED_TOP_LEVEL_CLASSES`. When documenting another Rust/PyO3 binding
source file, extend the script so CI prevents regressions for that file too.

## `test_rust.py`

Runs the Rust-backed PyO3 test suite through the active Python environment.

Run through PDM from `genja-core-python`:

```bash
pdm run test-rust
```

The script sets `PYO3_PYTHON` to the current Python interpreter before invoking
Cargo so Rust tests embed the same Python environment used by the package.
