#!/bin/sh

# PATH decoy for wheel interpreter tests. If this script runs, the CLI did not
# choose the Python interpreter beside its installed executable.
printf 'wrong interpreter selected\n' >&2
exit 73
