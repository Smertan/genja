import genja_core
import pytest
from genja_core.task import (
    Host,
    TaskRuntimeContext,
    TaskInfo,
    TaskMessage,
    TaskSuccessResult,
    task,
)


@task(
    name="verify_backup",
    connection_plugin_name="ssh",
    processors=["audit"],
    options={"mode": "strict"},
)
class VerifyBackupTask:
    def run(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskRuntimeContext)
        assert task.processors == ["audit"]
        assert task.options == {"mode": "strict"}
        return TaskSuccessResult(
            summary=f"verified {host.hostname}",
            messages=[TaskMessage(level="info", text=task.name)],
        )


@task(
    name="backup_config",
    connection_plugin_name="ssh",
    sub_task=VerifyBackupTask,
    options={"backup_path": "/tmp/configs", "compress": True},
)
class BackupConfigTask:
    def run(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskRuntimeContext)
        assert task.options == {"backup_path": "/tmp/configs", "compress": True}
        return TaskSuccessResult(
            changed=True,
            summary=f"backed up {host.hostname}",
            metadata={
                "sub_task_name": task.sub_task.name,
                "backup_path": task.options["backup_path"],
            },
        )


def test_task_definition_from_python_class_extracts_metadata():
    task_definition = genja_core.TaskDefinition.from_python_class(BackupConfigTask)

    assert task_definition.name == "backup_config"
    assert task_definition.connection_plugin_name == "ssh"
    assert len(task_definition.sub_tasks) == 1
    assert task_definition.sub_tasks[0].name == "verify_backup"
    assert task_definition.to_dict()["options"] == {
        "backup_path": "/tmp/configs",
        "compress": True,
    }
    assert task_definition.sub_tasks[0].to_dict()["processors"] == ["audit"]
    assert task_definition.sub_tasks[0].to_dict()["options"] == {"mode": "strict"}


def test_task_definition_run_on_host_executes_python_body():
    task_definition = genja_core.TaskDefinition.from_python_class(BackupConfigTask)

    result = task_definition.run_on_host(Host(hostname="router1", platform="ios"))
    data = result.to_dict()

    assert result.status == "passed"
    assert data["changed"] is True
    assert data["summary"] == "backed up router1"
    assert data["metadata"]["sub_task_name"] == "verify_backup"
    assert data["metadata"]["backup_path"] == "/tmp/configs"


def test_task_definition_from_python_class_requires_decorator_metadata():
    class MissingMetadataTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    with pytest.raises(ValueError, match="missing __genja_task_info__"):
        genja_core.TaskDefinition.from_python_class(MissingMetadataTask)


def test_task_definition_from_python_class_allows_missing_connection_plugin_name():
    @task(name="backup_config")
    class NoConnectionTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    task_definition = genja_core.TaskDefinition.from_python_class(NoConnectionTask)

    assert task_definition.connection_plugin_name is None
    assert task_definition.to_dict()["connection_plugin_name"] is None


def test_task_decorator_rejects_empty_connection_plugin_name():
    with pytest.raises(TypeError, match="connection_plugin_name must be a non-empty string or None"):
        @task(name="backup_config", connection_plugin_name="")
        class InvalidTask:
            def run(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_definition_from_python_class_rejects_empty_connection_plugin_name_in_metadata():
    @task(name="backup_config", connection_plugin_name="ssh")
    class InvalidTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    InvalidTask.__genja_task_info__["connection_plugin_name"] = ""

    with pytest.raises(ValueError, match="connection_plugin_name.*must not be empty"):
        genja_core.TaskDefinition.from_python_class(InvalidTask)


def test_task_decorator_requires_callable_run_method():
    with pytest.raises(TypeError, match="must define a 'run' method"):
        @task(name="backup_config", connection_plugin_name="ssh")
        class InvalidTask:
            pass


def test_task_decorator_rejects_undecorated_sub_task():
    class PlainSubTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    with pytest.raises(TypeError, match="must also be decorated with @task"):
        @task(
            name="backup_config",
            connection_plugin_name="ssh",
            sub_task=PlainSubTask,
        )
        class InvalidTask:
            def run(self, task, host, context):
                return TaskSuccessResult(summary="noop")
