"""Subprocess checks for the Python task discovery module command."""

import json
import subprocess
import sys


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
    (tmp_path / "discovered_tasks.py").write_text(
        'print("import message")\n'
        "import os\n"
        'os.write(1, b"native import message\\n")\n'
        "from genja.task import TaskRegistration, TaskSuccessResult, task\n"
        '@task(name="discovered", registration=TaskRegistration('
        'id="acme.discovered", version="1.0.0"))\n'
        "class DiscoveredTask:\n"
        "    def start(self, task, host, context):\n"
        '        return TaskSuccessResult(summary="done")\n'
    )
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text('[tool.genja.tasks]\nmodules = ["discovered_tasks"]\n')

    result = _run_discovery(project_file)

    assert result.returncode == 0
    assert "import message" in result.stderr
    assert "native import message" in result.stderr
    assert "import message" not in result.stdout
    descriptors = json.loads(result.stdout)
    assert len(descriptors) == 1
    assert descriptors[0]["id"] == "acme.discovered"
    assert descriptors[0]["version"] == "1.0.0"


def test_discovery_command_emits_empty_json_list(tmp_path):
    (tmp_path / "empty_tasks.py").write_text("")
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text('[tool.genja.tasks]\nmodules = ["empty_tasks"]\n')

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
