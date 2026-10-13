# Genja CLI

`genja-cli` distributes the existing Rust `genja` command as a prebuilt Python
wheel. The command includes the terminal UI (`genja tui`) and is installed into
the selected Python environment's scripts directory.

For Python task projects, install `genja-py` in the same environment to provide
the task API and discovery helper. The `genja-py[cli]` extra installs both
packages once released.

The `genja-cli` PyPI distribution, the Rust crate of the same name, and the
installed `genja` command are distinct. Source builds and Rust library usage
are documented in the [repository](https://github.com/Smertan/genja).
