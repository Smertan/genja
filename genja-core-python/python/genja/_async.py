"""Async wrapper helpers for sync extension methods."""

from __future__ import annotations

import asyncio
import threading
from typing import TYPE_CHECKING, TypeVar

if TYPE_CHECKING:
    from .genja import Genja, TaskResults, Tasks
    from .task import GenjaTaskProtocol

T = TypeVar("T")


async def _run_in_thread(fn, /, *args, **kwargs):
    state: dict[str, T | BaseException | bool | None] = {
        "done": False,
        "result": None,
        "exception": None,
    }

    def worker() -> None:
        try:
            state["result"] = fn(*args, **kwargs)
        except BaseException as exc:
            state["exception"] = exc
        else:
            state["done"] = True
            return
        state["done"] = True

    threading.Thread(target=worker, daemon=True).start()
    while not state["done"]:
        await asyncio.sleep(0.01)

    if state["exception"] is not None:
        raise state["exception"]

    return state["result"]


async def run_task_async(
    runtime: "Genja",
    task_class: type["GenjaTaskProtocol"],
    max_depth: int | None = None,
) -> "TaskResults":
    return await _run_in_thread(runtime.run_task, task_class, max_depth=max_depth)


async def run_tasks_async(
    runtime: "Genja",
    tasks: "Tasks",
    max_depth: int | None = None,
) -> list["TaskResults"]:
    return await _run_in_thread(runtime.run_tasks, tasks, max_depth=max_depth)
