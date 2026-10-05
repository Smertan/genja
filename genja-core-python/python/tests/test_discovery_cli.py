"""Subprocess checks for the Python task discovery module command."""

import json
from pathlib import Path
import shutil
import subprocess
import sys


def _copy_fixture_module(project_root, module_name):
    source = Path(__file__).parent / "fixtures" / f"{module_name}.py"
    shutil.copyfile(source, project_root / source.name)


def _run_discovery(pyproject_path):
    return subprocess.run(
        [
            sys.executable,
            "-m",
            "genja._discovery_cli",
            "--pyproject",
            str(pyproject_path),
        ],
        cwd=pyproject_path.parent.parent,
        capture_output=True,
        text=True,
        check=False,
    )


def test_discovery_command_imports_declared_project_module(tmp_path):
    _copy_fixture_module(tmp_path, "discovery_tasks")
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text('[tool.genja.tasks]\nmodules = ["discovery_tasks"]\n')

    result = _run_discovery(project_file)

    assert result.returncode == 0
    assert "import message" in result.stderr
    assert "native import message" in result.stderr
    assert "import message" not in result.stdout
    descriptors = json.loads(result.stdout)
    assert len(descriptors) == 1
    assert descriptors[0]["id"] == "acme.discovered"
    assert descriptors[0]["version"] == "1.0.0"


def test_discovery_command_imports_multiple_declared_modules(tmp_path):
    _copy_fixture_module(tmp_path, "discovery_tasks")
    _copy_fixture_module(tmp_path, "discovery_other")
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text(
        '[tool.genja.tasks]\nmodules = ["discovery_tasks", "discovery_other"]\n'
    )

    result = _run_discovery(project_file)

    assert result.returncode == 0
    descriptors = json.loads(result.stdout)
    assert len(descriptors) == 2
    assert {descriptor["id"] for descriptor in descriptors} == {
        "acme.discovered",
        "acme.other",
    }


def test_discovery_command_accepts_empty_module_list(tmp_path):
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text("[tool.genja.tasks]\nmodules = []\n")

    result = _run_discovery(project_file)

    assert result.returncode == 0
    assert json.loads(result.stdout) == []


def test_discovery_command_ignores_tasks_without_registration(tmp_path):
    _copy_fixture_module(tmp_path, "discovery_empty")
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text('[tool.genja.tasks]\nmodules = ["discovery_empty"]\n')

    result = _run_discovery(project_file)

    assert result.returncode == 0
    assert json.loads(result.stdout) == []


def test_discovery_command_reports_failed_import_without_json(tmp_path):
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text('[tool.genja.tasks]\nmodules = ["missing_tasks"]\n')

    result = _run_discovery(project_file)

    assert result.returncode != 0
    assert result.stdout == ""
    assert "missing_tasks" in result.stderr


def test_discovery_command_discards_descriptors_after_import_exception(tmp_path):
    _copy_fixture_module(tmp_path, "discovery_tasks")
    _copy_fixture_module(tmp_path, "discovery_broken")
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text(
        '[tool.genja.tasks]\nmodules = ["discovery_tasks", "discovery_broken"]\n'
    )

    result = _run_discovery(project_file)

    assert result.returncode != 0
    assert result.stdout == ""
    assert "discovery_broken" in result.stderr
    assert "RuntimeError: import exploded" in result.stderr


def test_discovery_command_reports_invalid_module_field_without_json(tmp_path):
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text('[tool.genja.tasks]\nmodules = ["valid", 3]\n')

    result = _run_discovery(project_file)

    assert result.returncode != 0
    assert result.stdout == ""
    assert "[tool.genja.tasks].modules[1]" in result.stderr
