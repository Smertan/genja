import genja
import pytest
from genja.task import (
    Host,
    GenjaTaskProtocol,
    TaskRuntimeContext,
    TaskMessageLevel,
    TaskInfo,
    TaskMessage,
    TaskSuccessResult,
    task,
)
from typing import cast


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
            messages=[TaskMessage(level=TaskMessageLevel.INFO, text=task.name)],
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


@task(
    name="verify_backup_plain",
    connection_plugin_name="ssh",
    options={"mode": "strict"},
)
class VerifyBackupPlainTask:
    def run(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskRuntimeContext)
        assert task.options == {"mode": "strict"}
        return TaskSuccessResult(
            summary=f"verified {host.hostname}",
            messages=[TaskMessage(level=TaskMessageLevel.INFO, text=task.name)],
        )


@task(
    name="backup_config_plain",
    connection_plugin_name="ssh",
    sub_task=VerifyBackupPlainTask,
    options={"backup_path": "/tmp/configs", "compress": True},
)
class BackupConfigPlainTask:
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
    task_definition = genja.TaskDefinition.from_python_class(BackupConfigTask)

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
    task_definition = genja.TaskDefinition.from_python_class(BackupConfigPlainTask)

    result = task_definition.run_on_host(Host(hostname="router1", platform="ios"))
    data = result.to_dict()

    assert result.passed_hosts == ["router1"]
    assert data["hosts"]["router1"]["status"] == "passed"
    assert data["hosts"]["router1"]["summary"] == "backed up router1"
    assert (
        data["hosts"]["router1"]["metadata"]["sub_task_name"] == "verify_backup_plain"
    )
    assert data["hosts"]["router1"]["metadata"]["backup_path"] == "/tmp/configs"


def test_task_definition_from_python_class_requires_decorator_metadata():
    class MissingMetadataTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    with pytest.raises(ValueError, match="missing __genja_task_info__"):
        genja.TaskDefinition.from_python_class(MissingMetadataTask)


def test_task_definition_from_python_class_allows_missing_connection_plugin_name():
    @task(name="backup_config")
    class NoConnectionTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    task_definition = genja.TaskDefinition.from_python_class(NoConnectionTask)

    assert task_definition.connection_plugin_name is None
    assert task_definition.to_dict()["connection_plugin_name"] is None


def test_task_decorator_rejects_empty_connection_plugin_name():
    with pytest.raises(
        TypeError, match="connection_plugin_name must be a non-empty string or None"
    ):

        @task(name="backup_config", connection_plugin_name="")
        class InvalidTask:
            def run(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_decorator_rejects_empty_name():
    with pytest.raises(TypeError, match="name must be a non-empty string"):

        @task(name="   ", connection_plugin_name="ssh")
        class InvalidTask:
            def run(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_decorator_rejects_non_json_serializable_options():
    with pytest.raises(TypeError, match="options must be JSON-serializable"):

        @task(
            name="backup_config",
            connection_plugin_name="ssh",
            options={"callback": lambda: None},
        )
        class InvalidTask:
            def run(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_definition_from_python_class_rejects_empty_connection_plugin_name_in_metadata():
    @task(name="backup_config", connection_plugin_name="ssh")
    class InvalidTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__[
        "connection_plugin_name"
    ] = ""

    with pytest.raises(ValueError, match="connection_plugin_name.*must not be empty"):
        genja.TaskDefinition.from_python_class(InvalidTask)


def test_task_definition_from_python_class_rejects_empty_name_in_metadata():
    @task(name="backup_config", connection_plugin_name="ssh")
    class InvalidTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__["name"] = "   "

    with pytest.raises(ValueError, match="field 'name' must not be empty"):
        genja.TaskDefinition.from_python_class(InvalidTask)


def test_task_definition_from_python_class_rejects_non_json_serializable_options_in_metadata():
    @task(name="backup_config", connection_plugin_name="ssh")
    class InvalidTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__["options"] = {
        "callback": lambda: None
    }

    with pytest.raises(TypeError, match="not JSON serializable"):
        genja.TaskDefinition.from_python_class(InvalidTask)


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
