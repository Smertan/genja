"""Configuration checks for the internal Python task discovery helper."""

import pytest

from genja._discovery_cli import load_discovery_config


def test_load_discovery_config_reads_modules_and_project_root(tmp_path):
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text(
        '[tool.genja.tasks]\nmodules = ["my_project.tasks.network", '
        '"my_project.tasks.backups"]\n'
    )

    config = load_discovery_config(project_file)

    assert config.modules == (
        "my_project.tasks.network",
        "my_project.tasks.backups",
    )
    assert config.project_root == tmp_path


def test_load_discovery_config_accepts_empty_modules(tmp_path):
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text("[tool.genja.tasks]\nmodules = []\n")

    assert load_discovery_config(project_file).modules == ()


@pytest.mark.parametrize(
    ("contents", "message"),
    [
        ("[tool.genja.tasks]\n", r"\[tool.genja.tasks\].modules"),
        ('[tool.genja.tasks]\nmodules = "tasks"\n', "must be a list"),
        ('[tool.genja.tasks]\nmodules = ["tasks", 1]\n', r"modules\[1\]"),
        ('[tool.genja.tasks]\nmodules = ["tasks..network"]\n', r"modules\[0\]"),
        ("[tool.genja.tasks]\nmodules = [\n", "invalid TOML"),
    ],
)
def test_load_discovery_config_rejects_invalid_configuration(
    tmp_path, contents, message
):
    project_file = tmp_path / "pyproject.toml"
    project_file.write_text(contents)

    with pytest.raises(ValueError, match=message):
        load_discovery_config(project_file)


def test_load_discovery_config_reports_missing_file(tmp_path):
    with pytest.raises(ValueError, match="cannot read .*pyproject.toml"):
        load_discovery_config(tmp_path / "pyproject.toml")
