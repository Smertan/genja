import genja_core
from genja_core.task import Host, TaskMessage, TaskSuccessResult, task


@task(name="runtime_backup", connection_plugin_name="ssh")
class RuntimeBackupTask:
    def run(self, task, host, context):
        return TaskSuccessResult(
            changed=True,
            summary=f"runtime handled {host.hostname}",
            messages=[TaskMessage(level="info", text=task.name)],
            metadata={"platform": host.platform},
        )


@task(name="runtime_child", connection_plugin_name="ssh")
class RuntimeChildTask:
    def run(self, task, host, context):
        return TaskSuccessResult(
            summary=f"child handled {host.hostname}",
            metadata={
                "current_depth": context.current_depth,
                "max_depth": context.max_depth,
            },
        )


@task(
    name="runtime_parent",
    connection_plugin_name="ssh",
    sub_task=RuntimeChildTask,
)
class RuntimeParentTask:
    def run(self, task, host, context):
        return TaskSuccessResult(
            summary=f"parent handled {host.hostname}",
            metadata={
                "current_depth": context.current_depth,
                "max_depth": context.max_depth,
            },
        )


def test_genja_runtime_runs_python_task_definition():
    runtime = genja_core.Genja.from_hosts(
        {
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="ios"),
        }
    ).with_runner("serial")
    results = runtime.run_task(RuntimeBackupTask)
    data = results.to_dict()
    summary = results.host_summary()

    assert results.task_name == "runtime_backup"
    assert results.passed_hosts == ["router1", "router2"]
    assert results.failed_hosts == []
    assert results.skipped_hosts == []
    assert summary == {"passed": 2, "failed": 0, "skipped": 0, "total": 2}
    assert data["task_name"] == "runtime_backup"
    assert data["hosts"]["router1"]["Passed"]["summary"] == "runtime handled 10.0.0.1"
    assert data["hosts"]["router2"]["Passed"]["metadata"]["platform"] == "ios"


def test_genja_runtime_passes_real_task_context_depth():
    runtime = genja_core.Genja.from_hosts(
        {
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }
    ).with_runner("serial")
    results = runtime.run_task(RuntimeParentTask, max_depth=1)
    data = results.to_dict(raw=True)

    assert data["hosts"]["router1"]["Passed"]["metadata"] == {
        "current_depth": 0,
        "max_depth": 1,
    }

    child_results = data["sub_tasks"]["runtime_child"]
    assert child_results["hosts"]["router1"]["Passed"]["metadata"] == {
        "current_depth": 1,
        "max_depth": 1,
    }
