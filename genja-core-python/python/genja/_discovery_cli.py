"""Import explicitly configured Python task modules and export descriptors."""

from __future__ import annotations

import argparse
from contextlib import redirect_stdout
from dataclasses import dataclass
import importlib
import json
from pathlib import Path
import sys

from .task import list_registered_tasks

if sys.version_info >= (3, 11):
    import tomllib
else:
    import tomli as tomllib


@dataclass(frozen=True)
class DiscoveryConfig:
    """Validated task modules and the directory containing their project file."""

    modules: tuple[str, ...]
    project_root: Path


def load_discovery_config(pyproject_path: Path) -> DiscoveryConfig:
    """Read explicit task module names from a project's ``pyproject.toml``."""
    path = pyproject_path.resolve()
    try:
        with path.open("rb") as project_file:
            project = tomllib.load(project_file)
    except OSError as error:
        raise ValueError(f"cannot read {path}: {error}") from error
    except tomllib.TOMLDecodeError as error:
        raise ValueError(f"invalid TOML in {path}: {error}") from error

    tool = project.get("tool")
    genja = tool.get("genja") if isinstance(tool, dict) else None
    tasks = genja.get("tasks") if isinstance(genja, dict) else None
    if not isinstance(tasks, dict) or "modules" not in tasks:
        raise ValueError(f"{path}: missing [tool.genja.tasks].modules")

    modules = tasks["modules"]
    if not isinstance(modules, list):
        raise ValueError(f"{path}: [tool.genja.tasks].modules must be a list")

    for index, module in enumerate(modules):
        if not isinstance(module, str) or not all(
            part.isidentifier() for part in module.split(".")
        ):
            raise ValueError(
                f"{path}: [tool.genja.tasks].modules[{index}] "
                "must be a nonempty dotted Python module name"
            )

    return DiscoveryConfig(modules=tuple(modules), project_root=path.parent)


def main(argv: list[str] | None = None) -> int:
    """Export registered task descriptors from a declared project module list."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--pyproject",
        type=Path,
        required=True,
        help="path to the project's pyproject.toml",
    )
    args = parser.parse_args(argv)

    try:
        config = load_discovery_config(args.pyproject)
    except ValueError as error:
        print(error, file=sys.stderr)
        return 1

    project_root = str(config.project_root)
    sys.path.insert(0, project_root)
    try:
        for module_name in config.modules:
            try:
                with redirect_stdout(sys.stderr):
                    importlib.import_module(module_name)
            except (Exception, SystemExit) as error:
                print(
                    f"failed to import task module {module_name!r}: "
                    f"{type(error).__name__}: {error}",
                    file=sys.stderr,
                )
                return 1

        try:
            result = json.dumps([
                descriptor.to_dict() for descriptor in list_registered_tasks()
            ])
        except (TypeError, ValueError) as error:
            print(f"failed to serialize task descriptors: {error}", file=sys.stderr)
            return 1
    finally:
        sys.path.remove(project_root)

    print(result)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
