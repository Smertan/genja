from pathlib import Path

import genja_core


def test_settings_from_file_loads_yaml():
    settings_file = Path(__file__).parent / "fixtures" / "settings.yaml"
    settings = genja_core.Settings.from_file(str(settings_file))

    assert settings.core.raise_on_error is False
    assert settings.inventory.plugin == "FileInventoryPlugin"
    assert settings.inventory.options.hosts_file == "./inventory/hosts.yaml"
    assert settings.ssh.config_file is None
    assert settings.runner.plugin == "threaded"
    assert settings.runner.worker_count == 10
    assert settings.logging.level == "info"
    assert settings.logging.max_file_count == 10
