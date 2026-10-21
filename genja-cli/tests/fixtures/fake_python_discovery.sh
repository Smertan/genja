#!/bin/sh

# Test double for the Python executable, not a Python module. Rust tests copy
# this file as "fake-python" and run it through PythonTaskDescriptorSource.
# It checks the helper command without requiring Python or genja-py.
if [ "$#" -ne 4 ] || [ "$1" != "-m" ] || [ "$2" != "genja._discovery_cli" ] || [ "$3" != "--pyproject" ]; then
    printf 'unexpected discovery helper arguments\n' >&2
    exit 2
fi

# Record where the source ran the command and which project file it supplied.
printf '%s' "$PWD" > observed_cwd.txt
printf '%s' "$4" > observed_pyproject.txt

# The Rust test writes these optional files in the project directory to choose
# the simulated helper's stderr, stdout JSON, and nonzero exit code.
if [ -f helper_stderr.txt ]; then
    cat helper_stderr.txt >&2
fi
if [ -f helper_stdout.txt ]; then
    cat helper_stdout.txt
fi
if [ -f helper_exit_code.txt ]; then
    exit "$(cat helper_exit_code.txt)"
fi
