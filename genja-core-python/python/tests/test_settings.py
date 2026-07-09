from pathlib import Path

import genja
import pytest


def test_settings_from_file_loads_yaml():
    settings_file = Path(__file__).parent / "fixtures" / "settings.yaml"
    settings = genja.Settings.from_file(str(settings_file))

    assert settings.core.raise_on_error is False
    assert settings.inventory.plugin == "FileInventoryPlugin"
    assert settings.inventory.options.hosts_file == "./inventory/hosts.yaml"
    assert settings.inventory.transform_function_options is None
    assert settings.ssh.config_file is None
    assert settings.runner.plugin == "threaded"
    assert settings.runner.worker_count == 10
    assert settings.runner.retry.allow is True
    assert settings.runner.retry.max_attempts == 4
    assert settings.runner.retry.delay_ms == 250
    assert settings.logging.level == "info"
    assert settings.logging.max_file_count == 10


def test_inventory_transform_function_options_are_exposed(tmp_path):
    settings_file = tmp_path / "settings.yaml"
    settings_file.write_text(
        "\n".join([
            "inventory:",
            "  plugin: FileInventoryPlugin",
            "  options:",
            "    hosts_file: ./hosts.yaml",
            "  transform_function: normalize_inventory",
            "  transform_function_options:",
            "    suffix: -lab",
            "    defaults:",
            "      platform: linux",
        ])
    )

    settings = genja.Settings.from_file(str(settings_file))

    assert settings.inventory.transform_function == "normalize_inventory"
    assert settings.inventory.transform_function_options == {
        "suffix": "-lab",
        "defaults": {"platform": "linux"},
    }


def test_settings_from_file_rejects_unknown_fields(tmp_path):
    settings_file = tmp_path / "settings.yaml"
    settings_file.write_text(
        "\n".join([
            "runner:",
            "  worker_counts: 10",
        ])
    )

    with pytest.raises(ValueError, match="worker_counts"):
        genja.Settings.from_file(str(settings_file))
