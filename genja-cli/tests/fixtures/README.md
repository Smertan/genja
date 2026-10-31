# Python discovery process fixtures

These scripts are fake Python executables used by the tests in
[`src/discovery/python.rs`](../../src/discovery/python.rs) and
[`src/discovery/combined.rs`](../../src/discovery/combined.rs), and by the
wheel interpreter test in [`tests/cli.rs`](../cli.rs). They test how
the Rust sources launch and read the discovery helper. They do not import tasks
or run Python, so the Rust tests do not require a Python installation or
`genja-py`.

On Unix (including macOS), the tests link the executable
`fake_python_discovery.sh` into a temporary project directory. On Windows,
they copy `fake_python_discovery.cmd`. The temporary path becomes the source's
selected interpreter. When the source calls it, the script checks for
`-m genja._discovery_cli --pyproject <path>` and records the working directory
and project path in `observed_cwd.txt` and
`observed_pyproject.txt`.

The tests create these optional files in the temporary project directory to
choose the script's response:

| File | Effect |
| --- | --- |
| `helper_stdout.txt` | Contents sent to stdout, normally descriptor JSON. |
| `helper_stderr.txt` | Contents sent to stderr, such as import diagnostics. |
| `helper_exit_code.txt` | Exit code to return; the default is success. |

The real Python helper's module imports and JSON export are tested separately
in [`test_discovery_cli.py`](../../../genja-core-python/python/tests/test_discovery_cli.py).

`wrong_python.sh` is a Unix-only `PATH` decoy. The wheel interpreter test links
it ahead of the system Python while invoking a copied `genja` binary directly.
Its distinctive error shows if the CLI picks `PATH` instead of its companion
interpreter.
