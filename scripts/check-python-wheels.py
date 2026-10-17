#!/usr/bin/env python3
"""Validate Genja CLI wheels and wheel-only Python installations."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import venv
from email.parser import Parser
from zipfile import ZipFile


def one_wheel(directory: Path) -> Path:
    wheels = sorted(directory.glob("*.whl"))
    if len(wheels) != 1 or list(directory.glob("*.tar.gz")):
        raise ValueError(f"expected one wheel and no sdist in {directory}: {wheels}")
    return wheels[0]


def metadata(wheel: Path):
    with ZipFile(wheel) as archive:
        path = next(name for name in archive.namelist() if name.endswith("/METADATA"))
        return Parser().parsestr(archive.read(path).decode("utf-8"))


def inspect_cli(directory: Path, version: str, platform: str, glibc: str) -> None:
    wheel = one_wheel(directory)
    fields = wheel.stem.split("-", 4)
    if len(fields) != 5 or fields[0] not in {"genja_cli", "genja_py_cli"}:
        raise ValueError(f"unexpected CLI wheel name: {wheel.name}")
    if fields[1:4] != [version, "py3", "none"]:
        raise ValueError(f"unexpected CLI wheel name or Python ABI: {wheel.name}")
    tag = {
        "linux": f"manylinux_{glibc}_x86_64",
        "macos": "macosx_11_0_arm64",
        "macos-intel": "macosx_10_12_x86_64",
        "windows": "win_amd64",
    }[platform]
    if tag not in fields[4].split("."):
        raise ValueError(f"{wheel.name} lacks required platform tag {tag}")
    info = metadata(wheel)
    if info["Name"].replace("-", "_") != fields[0]:
        raise ValueError(f"CLI wheel name and metadata disagree in {wheel.name}")
    if info["Version"] != version or info["Requires-Python"] != ">=3.10":
        raise ValueError(f"incorrect CLI wheel metadata in {wheel.name}")
    executable = f"{fields[0]}-{version}.data/scripts/genja"
    if platform == "windows":
        executable += ".exe"
    with ZipFile(wheel) as archive:
        if (
            executable not in archive.namelist()
            or archive.getinfo(executable).file_size == 0
        ):
            raise ValueError(f"{wheel.name} lacks the {executable} executable")
    print(f"validated {wheel.name}")


def inspect_intel_python(directory: Path, version: str, python_version: str) -> None:
    wheel = one_wheel(directory)
    fields = wheel.stem.split("-", 4)
    python_tag = "cp" + python_version.replace(".", "")
    if (
        len(fields) != 5
        or fields[:4] != ["genja_py", version, python_tag, python_tag]
        or "macosx_10_12_x86_64" not in fields[4].split(".")
    ):
        raise ValueError(f"unexpected Intel Python wheel tags: {wheel.name}")
    info = metadata(wheel)
    if info["Name"] != "genja-py" or info["Version"] != version:
        raise ValueError(f"incorrect Python wheel metadata in {wheel.name}")
    print(f"validated {wheel.name}")


def execute(*arguments: object, cwd: Path | None = None) -> str:
    command = [str(argument) for argument in arguments]
    result = subprocess.run(command, cwd=cwd, capture_output=True, text=True)
    if result.returncode:
        raise ValueError(f"{command} failed ({result.returncode}): {result.stderr}")
    return result.stdout


def scripts_directory(environment: Path) -> Path:
    return environment / ("Scripts" if os.name == "nt" else "bin")


def new_environment(
    environment: Path, wheel: Path, cli_directory: Path, extra: bool
) -> None:
    venv.EnvBuilder(with_pip=True).create(environment)
    python = scripts_directory(environment) / (
        "python.exe" if os.name == "nt" else "python"
    )
    execute(
        python,
        "-m",
        "pip",
        "install",
        "--only-binary=:all:",
        "pydantic>=2.11.0",
        "tomli>=2.0; python_version < '3.11'",
    )
    requirement = f"{wheel}[cli]" if extra else str(wheel)
    execute(
        python,
        "-m",
        "pip",
        "install",
        "--no-index",
        "--only-binary=:all:",
        "--find-links",
        cli_directory,
        requirement,
    )


def install_check(
    python_directory: Path, cli_directory: Path, version: str, project: Path
) -> None:
    python_wheel = one_wheel(python_directory).resolve()
    cli_wheel = one_wheel(cli_directory)
    if (
        metadata(python_wheel)["Version"] != version
        or metadata(cli_wheel)["Version"] != version
    ):
        raise ValueError(f"wheel versions do not match {version}")
    with tempfile.TemporaryDirectory(prefix="genja-wheel-check-") as temporary:
        root = Path(temporary)
        base = root / "base"
        new_environment(base, python_wheel, cli_directory.resolve(), extra=False)
        command_name = "genja.exe" if os.name == "nt" else "genja"
        if (scripts_directory(base) / command_name).exists():
            raise ValueError("base genja-py installation unexpectedly contains genja")

        combined = root / "combined"
        new_environment(combined, python_wheel, cli_directory.resolve(), extra=True)
        genja = scripts_directory(combined) / command_name
        if not genja.is_file():
            raise ValueError(f"CLI wheel did not install {genja}")
        if "tui" not in execute(genja, "--help"):
            raise ValueError("installed CLI does not offer the TUI command")
        execute(genja, "tui", "--help")
        tasks = json.loads(
            execute(genja, "task", "list", "--output", "json", cwd=project.resolve())
        )
        if not tasks or any(task["language"] != "python" for task in tasks):
            raise ValueError(f"installed CLI did not discover Python tasks: {tasks}")
    print(f"wheel-only installation passed with Python {sys.version.split()[0]}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    inspect = commands.add_parser("inspect-cli")
    inspect.add_argument("--wheel-dir", type=Path, required=True)
    inspect.add_argument("--version", required=True)
    inspect.add_argument(
        "--platform",
        choices=["linux", "macos", "macos-intel", "windows"],
        required=True,
    )
    inspect.add_argument("--minimum-glibc", default="2_17")
    inspect_python = commands.add_parser("inspect-intel-python")
    inspect_python.add_argument("--wheel-dir", type=Path, required=True)
    inspect_python.add_argument("--version", required=True)
    inspect_python.add_argument("--python-version", required=True)
    install = commands.add_parser("install")
    install.add_argument("--python-wheels", type=Path, required=True)
    install.add_argument("--cli-wheels", type=Path, required=True)
    install.add_argument("--version", required=True)
    install.add_argument("--project", type=Path, required=True)
    args = parser.parse_args()
    if args.command == "inspect-cli":
        inspect_cli(args.wheel_dir, args.version, args.platform, args.minimum_glibc)
    elif args.command == "inspect-intel-python":
        inspect_intel_python(args.wheel_dir, args.version, args.python_version)
    else:
        install_check(args.python_wheels, args.cli_wheels, args.version, args.project)


if __name__ == "__main__":
    main()
