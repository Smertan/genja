"""Public Python async wrappers for the native runtime awaitables.

These wrappers preserve normal Python coroutine semantics: callers can create
the coroutine object first and await it later, while the Rust-native async
bridge is only entered once the coroutine is actually awaited.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, cast

if TYPE_CHECKING:
    from .genja import Genja, TaskResults, Tasks
    from .task import GenjaTaskProtocol

async def run_task_async(
    runtime: "Genja",
    task_class: type["GenjaTaskProtocol"],
    max_depth: int | None = None,
) -> "TaskResults":
    runtime_any = cast(Any, runtime)
    return await runtime_any._run_task_async_native(task_class, max_depth=max_depth)


async def run_tasks_async(
    runtime: "Genja",
    tasks: "Tasks",
    max_depth: int | None = None,
) -> list["TaskResults"]:
    runtime_any = cast(Any, runtime)
    return await runtime_any._run_tasks_async_native(tasks, max_depth=max_depth)
