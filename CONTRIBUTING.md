# Contributing

The canonical contributor guide lives in [docs/contributing.md](docs/contributing.md).

Before opening a pull request, run the checks that match your change:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --exclude genja-core-python
```

For Python binding changes:

```bash
cd genja-core-python
pdm install -d
pdm run lint
pdm run typecheck
pdm run test
pdm run test-rust
```
