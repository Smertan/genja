import genja_core
import pytest
from genja_core.task import Host, TaskContext, TaskInfo, TaskMessage, TaskSuccessResult, task


@task(name="verify_backup", plugin_name="ssh")
class VerifyBackupTask:
    def run(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskContext)
        return TaskSuccessResult(
            summary=f"verified {host.hostname}",
            messages=[TaskMessage(level="info", text=task.name)],
        )


@task(name="backup_config", plugin_name="ssh", sub_task=VerifyBackupTask)
class BackupConfigTask:
    def run(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskContext)
        return TaskSuccessResult(
            changed=True,
            summary=f"backed up {host.hostname}",
            metadata={"sub_task_name": task.sub_task.name},
        )


def test_task_definition_from_python_class_extracts_metadata():
    task_definition = genja_core.TaskDefinition.from_python_class(BackupConfigTask)

    assert task_definition.name == "backup_config"
    assert task_definition.plugin_name == "ssh"
    assert len(task_definition.sub_tasks) == 1
    assert task_definition.sub_tasks[0].name == "verify_backup"


def test_task_definition_run_on_host_executes_python_body():
    task_definition = genja_core.TaskDefinition.from_python_class(BackupConfigTask)

    result = task_definition.run_on_host(Host(hostname="router1", platform="ios"))
    data = result.to_dict()

    assert result.status == "passed"
    assert data["changed"] is True
    assert data["summary"] == "backed up router1"
    assert data["metadata"]["sub_task_name"] == "verify_backup"


def test_task_definition_from_python_class_requires_decorator_metadata():
    class MissingMetadataTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    with pytest.raises(ValueError, match="missing __genja_task_info__"):
        genja_core.TaskDefinition.from_python_class(MissingMetadataTask)


def test_task_definition_from_python_class_rejects_empty_plugin_name():
    @task(name="backup_config", plugin_name="")
    class InvalidTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    with pytest.raises(ValueError, match="plugin_name.*must not be empty"):
        genja_core.TaskDefinition.from_python_class(InvalidTask)
