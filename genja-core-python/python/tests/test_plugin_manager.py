import sys
import textwrap

import pytest

import genja
from genja.task import Host, TaskSuccessResult, task
from tests.fixtures.connection_plugins import ConnectionPlugin
from tests.fixtures.inventory_plugins import StaticInventoryPlugin
from tests.fixtures.processor_plugins import (
    MinimalAuditProcessor,
    UnsupportedGroupPlugin,
)
from tests.fixtures.runner_plugins import BatchRunnerPlugin
from tests.fixtures.transform_plugins import (
    AsyncHostTransformPlugin,
    HostOnlyTransformPlugin,
)


@task(name="plugin_manager_runner_task")
class PluginManagerRunnerTask:
    def start(self, task, host, context):
        return TaskSuccessResult(summary=f"ran on {host.hostname}")


@task(name="plugin_manager_runner_task_two")
class PluginManagerRunnerTaskTwo:
    def start(self, task, host, context):
        return TaskSuccessResult(summary=f"ran second on {host.hostname}")


def test_plugin_manager_registers_all_supported_python_plugin_groups():
    manager = genja.PluginManager()
    manager.register_plugin(ConnectionPlugin())
    manager.register_plugin(MinimalAuditProcessor())
    manager.register_plugin(StaticInventoryPlugin())
    manager.register_plugin(BatchRunnerPlugin())
    manager.register_plugin(HostOnlyTransformPlugin())

    names = manager.plugin_names()
    names_and_groups = manager.plugin_names_and_groups()

    assert "ssh" in names
    assert "audit" in names
    assert "python_inventory" in names
    assert "python_batch_runner" in names
    assert "python_host_only_transform" in names
    assert ("ssh", "Connection") in names_and_groups
    assert ("audit", "Processor") in names_and_groups
    assert ("python_inventory", "Inventory") in names_and_groups
    assert ("python_batch_runner", "Runner") in names_and_groups
    assert ("python_host_only_transform", "TransformFunction") in names_and_groups


def test_plugin_manager_deregister_plugin_removes_plugin():
    manager = genja.PluginManager()
    manager.register_plugin(StaticInventoryPlugin())

    assert manager.deregister_plugin("python_inventory") == "python_inventory"
    assert "python_inventory" not in manager.plugin_names()
    assert manager.deregister_plugin("python_inventory") is None


def test_plugin_manager_rejects_unsupported_group_from_python():
    manager = genja.PluginManager()

    with pytest.raises(
        ValueError, match="unsupported python plugin group 'UnknownPlugin'"
    ):
        manager.register_plugin(UnsupportedGroupPlugin())


def test_plugin_manager_is_consumed_after_runtime_build():
    manager = genja.PluginManager()
    runtime = genja.Genja.from_hosts(
        {"router1": Host(hostname="10.0.0.1", platform="ios")},
        plugin_manager=manager,
    )

    assert runtime is not None
    with pytest.raises(ValueError, match="already been consumed"):
        manager.plugin_names()
    with pytest.raises(ValueError, match="already been consumed"):
        manager.register_plugin(MinimalAuditProcessor())


def test_runner_plugin_run_tasks_executes_from_python_side():
    manager = genja.PluginManager()
    manager.register_plugin(BatchRunnerPlugin())
    runtime = genja.Genja.from_hosts(
        {
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="nxos"),
        },
        plugin_manager=manager,
    ).with_runner("python_batch_runner")

    first = runtime.run_task(PluginManagerRunnerTask)
    second = runtime.run_task(PluginManagerRunnerTaskTwo)

    assert first.passed_hosts == ["router1", "router2"]
    assert second.passed_hosts == ["router1", "router2"]

    tasks = genja.Tasks()
    tasks.add_task(PluginManagerRunnerTask)
    tasks.add_task(PluginManagerRunnerTaskTwo)

    batch = runtime.run_tasks(tasks)

    assert [result.task_name for result in batch] == [
        "plugin_manager_runner_task",
        "plugin_manager_runner_task_two",
    ]
    assert batch[0].passed_hosts == ["router1", "router2"]
    assert batch[1].passed_hosts == ["router1", "router2"]


def test_transform_plugin_with_host_only_method_still_builds_runtime():
    manager = genja.PluginManager()
    manager.register_plugin(HostOnlyTransformPlugin())
    runtime = genja.Genja.from_hosts(
        {"router1": Host(hostname="10.0.0.1", platform="ios")},
        plugin_manager=manager,
    )

    assert runtime.inventory()["router1"]["hostname"] == "10.0.0.1"


def test_transform_plugin_resolves_async_hooks(tmp_path):
    hosts_path = tmp_path / "hosts.yaml"
    hosts_path.write_text("router1:\n  hostname: 10.0.0.1\n  platform: ios\n")
    settings_path = tmp_path / "settings.yaml"
    settings_path.write_text(
        textwrap.dedent(
            f"""
            inventory:
              plugin: FileInventoryPlugin
              options:
                hosts_file: {hosts_path}
              transform_function: python_async_transform
              transform_function_options:
                suffix: -lab
            runner:
              plugin: serial
            """
        )
    )

    manager = genja.PluginManager()
    manager.register_plugin(AsyncHostTransformPlugin())
    runtime = genja.Genja.from_settings_file(str(settings_path), plugin_manager=manager)

    transformed_hosts = runtime.iter_inventory_hosts()
    router1 = transformed_hosts[0][1]

    assert router1["hostname"] == "10.0.0.1-lab"


def test_plugin_manager_load_python_plugins_from_pyproject_rejects_name_mismatch(
    tmp_path,
):
    module_path = tmp_path / "processor_plugins.py"
    module_path.write_text(
        textwrap.dedent(
            """
            from tests.fixtures.processor_plugins import MinimalAuditProcessor
            """
        )
    )
    pyproject_path = tmp_path / "pyproject.toml"
    pyproject_path.write_text(
        textwrap.dedent(
            """
            [tool.genja.plugins.processor]
            wrong_name = "processor_plugins:MinimalAuditProcessor"
            """
        )
    )

    sys.path.insert(0, str(tmp_path))
    try:
        manager = genja.PluginManager()
        with pytest.raises(ValueError, match="plugin name mismatch"):
            manager.load_python_plugins_from_pyproject(str(pyproject_path))
    finally:
        sys.path.remove(str(tmp_path))
        sys.modules.pop("processor_plugins", None)


def test_plugin_manager_load_python_plugins_from_pyproject_rejects_non_string_entry(
    tmp_path,
):
    pyproject_path = tmp_path / "pyproject.toml"
    pyproject_path.write_text(
        textwrap.dedent(
            """
            [tool.genja.plugins.processor]
            audit = { path = "processor_plugins:MinimalAuditProcessor" }
            """
        )
    )

    manager = genja.PluginManager()
    with pytest.raises(ValueError, match="must be a string import path"):
        manager.load_python_plugins_from_pyproject(str(pyproject_path))
