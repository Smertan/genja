from __future__ import annotations

from collections.abc import Awaitable

from genja import Genja, TaskResults, Tasks
from genja.task import Host, TaskSuccessResult, task


@task(name="typecheck_runtime_async")
class RuntimeAsyncTypecheckTask:
    async def start_async(self, task, host: Host, context) -> TaskSuccessResult:
        return TaskSuccessResult(summary=f"handled {host.hostname}")


def check_runtime_async_types() -> None:
    runtime = Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1", platform="ios"),
    })
    tasks = Tasks()
    tasks.add_task(RuntimeAsyncTypecheckTask)

    one: Awaitable[TaskResults] = runtime.run_task_async(RuntimeAsyncTypecheckTask)
    many: Awaitable[list[TaskResults]] = runtime.run_tasks_async(tasks)

    _ = one
    _ = many
