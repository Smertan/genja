import genja_core
from genja_core.task import TaskMessage, TaskSuccessResult, task


@task(name="verify_backup", plugin_name="ssh")
class VerifyBackupTask:
    def run(self, task, host, context):
        return TaskSuccessResult(
            summary=f"verified {host['hostname']}",
            messages=[TaskMessage(level="info", text=task["name"])],
        )


@task(name="backup_config", plugin_name="ssh", sub_task=VerifyBackupTask)
class BackupConfigTask:
    def run(self, task, host, context):
        return TaskSuccessResult(
            changed=True,
            summary=f"backed up {host['hostname']}",
            metadata={"sub_task_name": task["sub_task"]["name"]},
        )


def test_task_definition_from_python_class_extracts_metadata():
    task_definition = genja_core.TaskDefinition.from_python_class(BackupConfigTask)

    assert task_definition.name == "backup_config"
    assert task_definition.plugin_name == "ssh"
    assert len(task_definition.sub_tasks) == 1
    assert task_definition.sub_tasks[0].name == "verify_backup"


def test_task_definition_run_on_host_executes_python_body():
    task_definition = genja_core.TaskDefinition.from_python_class(BackupConfigTask)

    result = task_definition.run_on_host({"hostname": "router1", "platform": "ios"})
    data = result.to_dict()

    assert result.status == "passed"
    assert data["changed"] is True
    assert data["summary"] == "backed up router1"
    assert data["metadata"]["sub_task_name"] == "verify_backup"
