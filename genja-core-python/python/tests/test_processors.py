import sys
import textwrap

import genja
from tests.fixtures.processor_plugins import AuditProcessor
from genja.task import Host, TaskSuccessResult, task


@task(name="processor_backup", connection_plugin_name="ssh", processors=["audit"])
class ProcessorBackupTask:
    def start(self, task, host, context):
        return TaskSuccessResult(
            summary=f"backed up {host.hostname}",
            metadata={"platform": host.platform},
        )


def test_plugin_manager_register_plugin_executes_python_processor():
    plugins = genja.PluginManager()
    processor = AuditProcessor()
    plugins.register_plugin(processor)

    runtime = genja.Genja.from_hosts(
        {"router1": Host(hostname="10.0.0.1", platform="ios")},
        plugin_manager=plugins,
    ).with_runner("serial")
    results = runtime.run_task(ProcessorBackupTask)
    data = results.to_dict()

    assert data["summary"] == "processed by audit"
    assert data["hosts"]["router1"]["status"] == "passed"
    assert data["hosts"]["router1"]["metadata"] == {
        "platform": "ios",
        "processor": "audit",
        "hostname": "router1",
    }
    assert processor.events == [
        ("task_start", "processor_backup", None),
        ("instance_start", "processor_backup", "router1"),
        ("instance_finish", "processor_backup", "router1"),
        ("task_finish", "processor_backup", None),
    ]


def test_plugin_manager_loads_python_processors_from_pyproject(tmp_path):
    module_path = tmp_path / "processor_plugins.py"
    module_path.write_text(
        textwrap.dedent(
            """
            from tests.fixtures.processor_plugins import AuditProcessor as BaseAuditProcessor

            class AuditProcessor(BaseAuditProcessor):
                def on_instance_finish(self, context, result):
                    data = super().on_instance_finish(context, result)
                    data["metadata"]["loaded_from"] = "pyproject"
                    return data
            """
        )
    )
    pyproject_path = tmp_path / "pyproject.toml"
    pyproject_path.write_text(
        textwrap.dedent(
            """
            [tool.genja.plugins.processor]
            audit = "processor_plugins:AuditProcessor"
            """
        )
    )

    sys.path.insert(0, str(tmp_path))
    try:
        plugins = genja.PluginManager()
        plugins.load_python_plugins_from_pyproject(str(pyproject_path))

        runtime = genja.Genja.from_hosts(
            {"router1": Host(hostname="10.0.0.1", platform="ios")},
            plugin_manager=plugins,
        ).with_runner("serial")
        results = runtime.run_task(ProcessorBackupTask)
    finally:
        sys.path.remove(str(tmp_path))
        sys.modules.pop("processor_plugins", None)

    data = results.to_dict()
    assert data["hosts"]["router1"]["status"] == "passed"
    assert data["hosts"]["router1"]["metadata"]["loaded_from"] == "pyproject"
