import asyncio

import pytest
import genja
from genja import Genja
from genja.task import (
    Host,
    IdempotencyCheckResult,
    IdempotencyMode,
    TaskFailureKind,
    TaskFailureResult,
    TaskSuccessResult,
    task,
)
from tests.fixtures.connection_plugins import ConnectionPlugin


@task(name="async_backup")
class AsyncBackupTask:
    async def start_async(self, task, host, context):
        await asyncio.sleep(0.01)
        return TaskSuccessResult(summary=f"async backup completed for {host.hostname}")


@task(name="async_with_connection", connection_plugin_name="ssh")
class AsyncConnectionTask:
    async def start_async(self, task, host, context):
        connection = context.connection()
        if connection is None:
            return TaskFailureResult(message="no connection available")

        await asyncio.sleep(0.01)
        return TaskSuccessResult(
            summary=f"async task used connection for {host.hostname}",
            result={"connection_type": "ssh"},
        )


@task(name="async_failing_task")
class AsyncFailingTask:
    async def start_async(self, task, host, context):
        await asyncio.sleep(0.01)
        return TaskFailureResult(
            message=f"async task failed on {host.hostname}",
            kind=TaskFailureKind.EXTERNAL,
        )


@task(name="async_parent", sub_tasks=[AsyncBackupTask])
class AsyncParentTask:
    async def start_async(self, task, host, context):
        await asyncio.sleep(0.01)
        return TaskSuccessResult(summary=f"async parent completed for {host.hostname}")


@task(name="sync_backup")
class SyncBackupTask:
    def start(self, task, host, context):
        return TaskSuccessResult(summary=f"sync backup completed for {host.hostname}")


@task(name="mixed_task")
class MixedTask:
    async def start_async(self, task, host, context):
        result = self._sync_helper(host.hostname)
        await asyncio.sleep(0.01)
        return TaskSuccessResult(summary=result)

    def _sync_helper(self, hostname):
        return f"processed {hostname}"


def test_runtime_run_task_async_executes_async_task():
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="ios"),
        }).with_runner("serial")

        return await runtime.run_task_async(AsyncBackupTask)

    results = asyncio.run(run_case())

    assert results.task_name == "async_backup"
    assert results.passed_hosts == ["router1", "router2"]
    assert results.failed_hosts == []

    data = results.to_dict()
    assert (
        "async backup completed"
        in data["hosts"]["router1"]["outcome"]["Passed"]["summary"]
    )


def test_runtime_run_task_async_handles_failures():
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")

        return await runtime.run_task_async(AsyncFailingTask)

    results = asyncio.run(run_case())

    assert results.task_name == "async_failing_task"
    assert results.passed_hosts == []
    assert results.failed_hosts == ["router1"]

    data = results.to_dict()
    assert (
        "async task failed" in data["hosts"]["router1"]["outcome"]["Failed"]["message"]
    )


def test_runtime_run_task_async_with_connection():
    async def run_case():
        plugins = genja.PluginManager()
        plugins.register_plugin(ConnectionPlugin())
        runtime = Genja.from_hosts(
            {
                "router1": Host(hostname="10.0.0.1", platform="ios"),
            },
            plugin_manager=plugins,
        ).with_runner("serial")

        return await runtime.run_task_async(AsyncConnectionTask)

    results = asyncio.run(run_case())

    assert results.task_name == "async_with_connection"
    assert results.passed_hosts == ["router1"]


def test_runtime_run_task_async_with_sub_tasks():
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")

        return await runtime.run_task_async(
            AsyncParentTask,
            run_options=genja.TaskRunOptions(max_depth=2),
        )

    results = asyncio.run(run_case())

    assert results.task_name == "async_parent"
    assert results.passed_hosts == ["router1"]

    data = results.to_dict()
    assert "sub_tasks" in data
    assert len(data["sub_tasks"]) == 1
    sub_task_names = list(data["sub_tasks"].keys())
    assert "async_backup" in sub_task_names


def test_runtime_run_task_async_supports_asyncio_gather():
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")

        first, second = await asyncio.gather(
            runtime.run_task_async(AsyncBackupTask),
            runtime.run_task_async(SyncBackupTask),
        )
        return first, second

    first, second = asyncio.run(run_case())

    assert first.task_name == "async_backup"
    assert second.task_name == "sync_backup"
    assert first.passed_hosts == ["router1"]
    assert second.passed_hosts == ["router1"]


def test_runtime_run_tasks_async_executes_multiple_tasks():
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="ios"),
        }).with_runner("serial")

        results = []
        for task_class in [AsyncBackupTask, SyncBackupTask, MixedTask]:
            result = await runtime.run_task_async(task_class)
            results.append(result)
        return results

    results = asyncio.run(run_case())

    assert len(results) == 3
    assert results[0].task_name == "async_backup"
    assert results[1].task_name == "sync_backup"
    assert results[2].task_name == "mixed_task"

    for result in results:
        assert result.passed_hosts == ["router1", "router2"]


def test_runtime_run_task_async_preserves_order_with_multiple_hosts():
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="ios"),
            "router3": Host(hostname="10.0.0.3", platform="ios"),
        }).with_runner("serial")

        return await runtime.run_task_async(AsyncBackupTask)

    results = asyncio.run(run_case())

    assert results.passed_hosts == ["router1", "router2", "router3"]


@pytest.mark.asyncio
async def test_runtime_run_task_async_works_in_pytest_asyncio():
    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")

    results = await runtime.run_task_async(AsyncBackupTask)

    assert results.task_name == "async_backup"
    assert results.passed_hosts == ["router1"]


@pytest.mark.asyncio
async def test_runtime_run_task_async_supports_dry_run():
    calls: list[str] = []

    @task(name="async_preview", supports_dry_run=True)
    class AsyncPreviewTask:
        async def start_async(self, task, host, context):
            calls.append("start_async")
            return TaskSuccessResult(summary="started")

        async def dry_run_async(self, task, host, context):
            calls.append("dry_run_async")
            assert task.supports_dry_run is True
            assert context.dry_run is True
            return TaskSuccessResult(
                changed=True,
                summary=f"would update {host.hostname}",
            )

    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")

    results = await runtime.run_task_async(
        AsyncPreviewTask,
        run_options=genja.TaskRunOptions(dry_run=True),
    )

    assert calls == ["dry_run_async"]
    assert results.passed_hosts == ["router1"]
    host_result = results.to_dict()["hosts"]["router1"]
    assert host_result["outcome"]["Passed"]["changed"] is True
    assert host_result["execution_metadata"]["dry_run"] is True


@pytest.mark.asyncio
async def test_runtime_run_task_async_idempotent_converged_check_skips_start():
    calls: list[str] = []

    @task(name="async_idempotent_converged", idempotency=IdempotencyMode.CHECK)
    class AsyncIdempotentConvergedTask:
        async def check_async(self, task, host, context):
            calls.append("check_async")
            assert task.idempotency == IdempotencyMode.CHECK
            await asyncio.sleep(0.01)
            return IdempotencyCheckResult.converged(
                summary=f"{host.hostname} already configured",
            )

        async def start_async(self, task, host, context):
            calls.append("start_async")
            return TaskSuccessResult(changed=True, summary="started")

    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")

    results = await runtime.run_task_async(AsyncIdempotentConvergedTask)
    host_result = results.to_dict()["hosts"]["router1"]

    assert calls == ["check_async"]
    assert results.passed_hosts == ["router1"]
    assert host_result["outcome"]["Passed"]["changed"] is False
    assert host_result["outcome"]["Passed"]["summary"] == "10.0.0.1 already configured"


@pytest.mark.asyncio
async def test_runtime_run_task_async_check_and_verify_reuses_check_hook():
    calls: list[str] = []

    @task(
        name="async_idempotent_verified",
        idempotency=IdempotencyMode.CHECK_AND_VERIFY,
    )
    class AsyncIdempotentVerifiedTask:
        async def check_async(self, task, host, context):
            calls.append("check_async")
            await asyncio.sleep(0.01)
            if calls.count("check_async") == 1:
                return IdempotencyCheckResult.change_required(diff="+configured")
            return IdempotencyCheckResult.converged(summary="now converged")

        async def start_async(self, task, host, context):
            calls.append("start_async")
            return TaskSuccessResult(changed=True, summary="applied")

    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")

    results = await runtime.run_task_async(AsyncIdempotentVerifiedTask)
    host_result = results.to_dict()["hosts"]["router1"]

    assert calls == ["check_async", "start_async", "check_async"]
    assert results.passed_hosts == ["router1"]
    assert host_result["outcome"]["Passed"]["changed"] is True
    assert host_result["outcome"]["Passed"]["summary"] == "applied"


@pytest.mark.asyncio
async def test_runtime_run_task_async_dry_run_bypasses_idempotency_check():
    calls: list[str] = []

    @task(
        name="async_idempotent_dry_run",
        idempotency=IdempotencyMode.CHECK,
        supports_dry_run=True,
    )
    class AsyncIdempotentDryRunTask:
        async def check_async(self, task, host, context):
            calls.append("check_async")
            return IdempotencyCheckResult.converged()

        async def start_async(self, task, host, context):
            calls.append("start_async")
            return TaskSuccessResult(changed=True, summary="started")

        async def dry_run_async(self, task, host, context):
            calls.append("dry_run_async")
            return TaskSuccessResult(changed=True, summary="would change")

    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")

    results = await runtime.run_task_async(
        AsyncIdempotentDryRunTask,
        run_options=genja.TaskRunOptions(dry_run=True),
    )
    host_result = results.to_dict()["hosts"]["router1"]

    assert calls == ["dry_run_async"]
    assert results.passed_hosts == ["router1"]
    assert host_result["outcome"]["Passed"]["changed"] is True
    assert host_result["execution_metadata"]["dry_run"] is True


def test_runtime_run_task_async_handles_exception_in_task():
    @task(name="async_exception_task")
    class AsyncExceptionTask:
        async def start_async(self, task, host, context):
            await asyncio.sleep(0.01)
            raise ValueError("something went wrong")

    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")

        return await runtime.run_task_async(AsyncExceptionTask)

    results = asyncio.run(run_case())

    assert results.failed_hosts == ["router1"]
    data = results.to_dict()
    assert (
        "something went wrong"
        in data["hosts"]["router1"]["outcome"]["Failed"]["message"]
    )


def test_runtime_run_task_async_with_timeout():
    @task(name="slow_async_task")
    class SlowAsyncTask:
        async def start_async(self, task, host, context):
            await asyncio.sleep(10)
            return TaskSuccessResult(summary="completed")

    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")

        try:
            return await asyncio.wait_for(
                runtime.run_task_async(SlowAsyncTask),
                timeout=0.1,
            )
        except asyncio.TimeoutError:
            return "timeout"

    result = asyncio.run(run_case())
    assert result == "timeout"


def test_runtime_run_task_async_returns_immediately():
    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")

    coro = runtime.run_task_async(AsyncBackupTask)

    assert asyncio.iscoroutine(coro)

    coro.close()


def test_mixed_sync_and_async_tasks_in_sequence():
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")

        results = []
        for task_class in [
            SyncBackupTask,
            AsyncBackupTask,
            SyncBackupTask,
            AsyncBackupTask,
        ]:
            result = await runtime.run_task_async(task_class)
            results.append(result)
        return results

    results = asyncio.run(run_case())

    assert len(results) == 4
    assert all(r.passed_hosts == ["router1"] for r in results)
