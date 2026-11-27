from __future__ import annotations

from collections.abc import Awaitable

from genja import (
    Genja,
    TaskListFailureControl,
    TaskListRunOptions,
    TaskListTimeoutControl,
    TaskResults,
    TaskRunOptions,
    Tasks,
    SubtaskContinuation,
)
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
    tasks.add_recovery_task(RuntimeAsyncTypecheckTask)
    recovery_root: bool = tasks[1].is_recovery_root

    one: Awaitable[TaskResults] = runtime.run_task_async(RuntimeAsyncTypecheckTask)
    many: Awaitable[list[TaskResults]] = runtime.run_tasks_async(tasks)
    list_options = TaskListRunOptions(
        TaskRunOptions(
            task_timeout=0.5,
            subtask_continuation=SubtaskContinuation.ON_PARENT_SUCCESS,
        ),
        TaskListTimeoutControl.SKIP_REMAINING_FOR_HOST,
        TaskListFailureControl.SKIP_REMAINING_FOR_HOST,
    )
    list_options = list_options.with_failure_control(TaskListFailureControl.CONTINUE)
    controlled: Awaitable[list[TaskResults]] = runtime.run_tasks_async(
        tasks, run_options=list_options
    )

    _ = one
    _ = many
    _ = controlled
    _ = recovery_root
