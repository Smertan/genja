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


def test_settings_can_be_constructed_programmatically():
    settings = genja.Settings(
        core=genja.CoreConfig(raise_on_error=True),
        inventory=genja.InventoryConfig(
            plugin="custom-inventory",
            options=genja.OptionsConfig(
                hosts_file="./hosts.yaml",
                groups_file="./groups.yaml",
            ),
            transform_function="normalize_inventory",
            transform_function_options={"suffix": "-lab"},
        ),
        ssh=genja.SSHConfig(config_file="./ssh_config"),
        runner=genja.RunnerConfig(
            plugin="serial",
            worker_count=2,
            max_task_depth=4,
            max_connection_attempts=5,
            retry=genja.RunnerRetryConfig(
                allow=True,
                max_attempts=3,
                delay_ms=250,
            ),
        ),
        logging=genja.LoggingConfig(
            enabled=False,
            level="debug",
            log_file="./genja.log",
            to_console=False,
            file_size=4096,
            max_file_count=3,
        ),
    )

    assert settings.core.raise_on_error is True
    assert settings.inventory.plugin == "custom-inventory"
    assert settings.inventory.options.hosts_file == "./hosts.yaml"
    assert settings.inventory.options.groups_file == "./groups.yaml"
    assert settings.inventory.options.defaults_file is None
    assert settings.inventory.transform_function == "normalize_inventory"
    assert settings.inventory.transform_function_options == {"suffix": "-lab"}
    assert settings.ssh.config_file == "./ssh_config"
    assert settings.runner.plugin == "serial"
    assert settings.runner.worker_count == 2
    assert settings.runner.max_task_depth == 4
    assert settings.runner.max_connection_attempts == 5
    assert settings.runner.retry.allow is True
    assert settings.runner.retry.max_attempts == 3
    assert settings.runner.retry.delay_ms == 250
    assert settings.logging.enabled is False
    assert settings.logging.level == "debug"
    assert settings.logging.log_file == "./genja.log"
    assert settings.logging.to_console is False
    assert settings.logging.file_size == 4096
    assert settings.logging.max_file_count == 3


def test_programmatic_settings_preserve_omitted_defaults():
    defaults = genja.Settings()
    settings = genja.Settings(
        runner=genja.RunnerConfig(plugin="serial"),
        logging=genja.LoggingConfig(level="debug"),
    )

    assert settings.core.raise_on_error == defaults.core.raise_on_error
    assert settings.inventory.plugin == defaults.inventory.plugin
    assert settings.runner.plugin == "serial"
    assert settings.runner.worker_count == defaults.runner.worker_count
    assert settings.runner.max_task_depth == defaults.runner.max_task_depth
    assert (
        settings.runner.max_connection_attempts
        == defaults.runner.max_connection_attempts
    )
    assert settings.runner.retry.allow == defaults.runner.retry.allow
    assert settings.runner.retry.max_attempts == defaults.runner.retry.max_attempts
    assert settings.runner.retry.delay_ms == defaults.runner.retry.delay_ms
    assert settings.logging.enabled == defaults.logging.enabled
    assert settings.logging.level == "debug"
    assert settings.logging.log_file == defaults.logging.log_file


def test_programmatic_runner_retry_omitted_fields_use_defaults():
    defaults = genja.Settings()
    settings = genja.Settings(
        runner=genja.RunnerConfig(
            retry=genja.RunnerRetryConfig(
                allow=True,
                delay_ms=500,
            ),
        )
    )

    assert settings.runner.retry.allow is True
    assert settings.runner.retry.max_attempts == defaults.runner.retry.max_attempts
    assert settings.runner.retry.delay_ms == 500


def test_programmatic_settings_are_accepted_by_runtime():
    settings = genja.Settings(runner=genja.RunnerConfig(plugin="serial"))

    runtime = genja.Genja.from_hosts({}, settings=settings)

    assert runtime.settings().runner.plugin == "serial"


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
