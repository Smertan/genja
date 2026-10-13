# Python Examples

These examples use the shared host inventory in `genja/examples/inventory`.

Install the Python package from the repository before running them locally:

```bash
cd genja-core-python
maturin develop
cd ..
```

Then run examples from any working directory:

```bash
python /path/to/genja/genja/examples/python/basic_runtime.py
python /path/to/genja/genja/examples/python/filter_hosts.py
python /path/to/genja/genja/examples/python/run_task.py
python /path/to/genja/genja/examples/python/run_task_tree.py
python /path/to/genja/genja/examples/python/task_registration.py
```

The adjacent `pyproject.toml` declares `task_registration_tasks` for CLI
discovery. With `genja-py` installed in the selected Python environment and
the separate `genja-cli` binary installed, run from this directory:

```bash
genja task list
genja task describe python:acme.examples.backup_config@1.0.0
genja tui
```

The `tui` command requires a CLI binary built with its optional `tui` feature.
The Rust `task_browser` example can use this same project file to show a mixed
Rust/Python list. See [Mixed Rust And Python Discovery](../../../docs/examples.md#mixed-rust-and-python-discovery).
