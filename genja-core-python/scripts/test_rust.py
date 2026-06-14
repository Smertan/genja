import os
import subprocess
import sys


def main() -> int:
    env = os.environ.copy()
    env["PYO3_PYTHON"] = sys.executable
    command = [
        "cargo",
        "test",
        "-p",
        "genja-core-python",
        "--lib",
        "--",
        "--test-threads=1",
        *sys.argv[1:],
    ]
    return subprocess.run(command, env=env).returncode


if __name__ == "__main__":
    raise SystemExit(main())
