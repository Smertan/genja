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
pdm run check-stubs
pdm run test
pdm run test-rust
```

## Changelog And Compatibility

- This repository uses a single top-level `CHANGELOG.md`.
- This repository follows Keep a Changelog style for `CHANGELOG.md`.
- Add user-visible changes under `Unreleased`.
- Use release-note headings such as:
  - `Added`
  - `Changed`
  - `Fixed`
  - `Removed`
- Mark breaking entries inline with `**Breaking:**` and include migration guidance in the same bullet.
- Update the changelog during the branch, not only at merge time.
- For linked work items, use `Refs: #<issue>` when an issue exists.
