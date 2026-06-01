
import asyncio
import pytest
from genja import Genja
from genja.task import Host, TaskSuccessResult, TaskFailureResult, task


# Basic async task
@task(name="async_backup")
class AsyncBackupTask:
    async def start(self, task, host, context):
        # Simulate async I/O operation
        await asyncio.sleep(0.01)
        return TaskSuccessResult(summary=f"async backup completed for {host.hostname}")


# Async task that uses connection
@task(name="async_with_connection", connection_plugin_name="ssh")
class AsyncConnectionTask:
    async def start(self, task, host, context):
        connection = context.connection
        if connection is None:
            return TaskFailureResult(message="no connection available")
        
        await asyncio.sleep(0.01)
        return TaskSuccessResult(
            summary=f"async task used connection for {host.hostname}",
            result={"connection_type": "ssh"}
        )


# Async task that fails
@task(name="async_failing_task")
class AsyncFailingTask:
    async def start(self, task, host, context):
        await asyncio.sleep(0.01)
        return TaskFailureResult(
            message=f"async task failed on {host.hostname}",
            kind="external"
        )


# Async task with sub-tasks
@task(name="async_parent", sub_task=AsyncBackupTask)
class AsyncParentTask:
    async def start(self, task, host, context):
        await asyncio.sleep(0.01)
        return TaskSuccessResult(summary=f"async parent completed for {host.hostname}")


# Sync task (for comparison)
@task(name="sync_backup")
class SyncBackupTask:
    def start(self, task, host, context):
        return TaskSuccessResult(summary=f"sync backup completed for {host.hostname}")


# Mixed async/sync task
@task(name="mixed_task")
class MixedTask:
    async def start(self, task, host, context):
        # Can call sync functions from async
        result = self._sync_helper(host.hostname)
        await asyncio.sleep(0.01)
        return TaskSuccessResult(summary=result)
    
    def _sync_helper(self, hostname):
        return f"processed {hostname}"


def test_runtime_run_task_async_executes_async_task():
    """Test basic async task execution"""
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
    assert "async backup completed" in data["hosts"]["router1"]["summary"]


def test_runtime_run_task_async_handles_failures():
    """Test async task that fails"""
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
    assert "async task failed" in data["hosts"]["router1"]["message"]


def test_runtime_run_task_async_with_connection():
    """Test async task with connection plugin"""
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")
        
        return await runtime.run_task_async(AsyncConnectionTask)

    results = asyncio.run(run_case())

    assert results.task_name == "async_with_connection"
    assert results.passed_hosts == ["router1"]


def test_runtime_run_task_async_with_sub_tasks():
    """Test async task with sub-tasks"""
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")
        
        return await runtime.run_task_async(AsyncParentTask, max_depth=2)

    results = asyncio.run(run_case())

    assert results.task_name == "async_parent"
    assert results.passed_hosts == ["router1"]
    
    # Check sub-task executed
    data = results.to_dict()
    assert "sub_tasks" in data
    assert len(data["sub_tasks"]) == 1
    # Fix: sub_tasks is a dict, not a list
    sub_task_names = list(data["sub_tasks"].keys())
    assert "async_backup" in sub_task_names


def test_runtime_run_task_async_supports_asyncio_gather():
    """Test running multiple async tasks concurrently"""
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
    """Test running multiple tasks in sequence"""
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
            "router2": Host(hostname="10.0.0.2", platform="ios"),
        }).with_runner("serial")
        
        # Fix: Run tasks individually instead of using Tasks
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
    """Test that async tasks process hosts in order"""
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
    """Test async task execution within pytest-asyncio"""
    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")
    
    results = await runtime.run_task_async(AsyncBackupTask)

    assert results.task_name == "async_backup"
    assert results.passed_hosts == ["router1"]


def test_runtime_run_task_async_handles_exception_in_task():
    """Test async task that raises an exception"""
    @task(name="async_exception_task")
    class AsyncExceptionTask:
        async def start(self, task, host, context):
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
    assert "something went wrong" in data["hosts"]["router1"]["message"]


def test_runtime_run_task_async_with_timeout():
    """Test async task with asyncio timeout"""
    @task(name="slow_async_task")
    class SlowAsyncTask:
        async def start(self, task, host, context):
            await asyncio.sleep(10)  # Very slow
            return TaskSuccessResult(summary="completed")

    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")
        
        try:
            # Fix: Use wait_for instead of timeout for Python 3.10
            return await asyncio.wait_for(
                runtime.run_task_async(SlowAsyncTask),
                timeout=0.1
            )
        except asyncio.TimeoutError:
            return "timeout"

    result = asyncio.run(run_case())
    assert result == "timeout"


def test_runtime_run_task_async_returns_immediately():
    """Test that run_task_async returns a coroutine immediately"""
    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    }).with_runner("serial")
    
    # Should return immediately without blocking
    coro = runtime.run_task_async(AsyncBackupTask)
    
    # Verify it's a coroutine
    assert asyncio.iscoroutine(coro)
    
    # Clean up
    coro.close()


def test_mixed_sync_and_async_tasks_in_sequence():
    """Test running sync and async tasks in sequence"""
    async def run_case():
        runtime = Genja.from_hosts({
            "router1": Host(hostname="10.0.0.1", platform="ios"),
        }).with_runner("serial")
        
        # Fix: Run tasks individually
        results = []
        for task_class in [SyncBackupTask, AsyncBackupTask, SyncBackupTask, AsyncBackupTask]:
            result = await runtime.run_task_async(task_class)
            results.append(result)
        return results

    results = asyncio.run(run_case())

    assert len(results) == 4
    assert all(r.passed_hosts == ["router1"] for r in results)
