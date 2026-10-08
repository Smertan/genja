# Python discovery process fixtures

These scripts are fake Python executables used by the tests in
[`src/discovery/python.rs`](../../src/discovery/python.rs) and
[`src/discovery/combined.rs`](../../src/discovery/combined.rs). They test how
the Rust sources launch and read the discovery helper. They do not import tasks
or run Python, so the Rust tests do not require a Python installation or
`genja-py`.

The tests copy `fake_python_discovery.sh` on Unix (including macOS) or
`fake_python_discovery.cmd` on Windows into a temporary project directory. The
copy becomes the source's selected interpreter. When the source calls it, the
script checks for `-m genja._discovery_cli --pyproject <path>` and records the
working directory and project path in `observed_cwd.txt` and
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
