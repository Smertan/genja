# Genja CLI

`genja-cli` distributes the existing Rust `genja` command as a prebuilt Python
wheel. The command includes the terminal UI (`genja tui`) and is installed into
the selected Python environment's scripts directory.

For Python task projects, install `genja-py` in the same environment to provide
the task API and discovery helper. Starting with v0.5.0, install both packages
with `python -m pip install "genja-py[cli]>=0.5.0,<0.6.0"`.

For configured Python tasks, the command uses that environment's Python
interpreter beside the installed executable. Use `--python PATH` to select
another interpreter.

The `genja-cli` PyPI distribution, the Rust crate of the same name, and the
installed `genja` command are distinct. Source builds and Rust library usage
are documented in the [repository](https://github.com/Smertan/genja).
