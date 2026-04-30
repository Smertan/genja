"""Python task authoring API for Genja.

Import task-facing helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.task`` is the primary public surface for:

- ``@task(...)`` task metadata decoration
- ``TaskMessage``
- ``TaskSuccessResult``
- ``TaskFailureResult``
- ``TaskSkipResult``

The canonical authoring shape is:

.. code-block:: python

    from genja_core.task import (
        Host,
        TaskContext,
        TaskInfo,
        TaskMessage,
        TaskSuccessResult,
        task,
    )

    @task(name="backup_config", plugin_name="ssh")
    class BackupConfigTask:
        def run(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(
                changed=True,
                summary=f"backed up {host.hostname}",
                messages=[
                    TaskMessage(level="info", text=f"task={task.name}")
                ],
                metadata={"platform": host.platform},
            )

``run(...)`` must return one of:

- ``TaskSuccessResult``
- ``TaskFailureResult``
- ``TaskSkipResult``

Task metadata comes from ``@task(...)``:

- ``name``: required and must be non-empty
- ``plugin_name``: required and must be non-empty
- ``sub_task``: optional decorated task class
"""

from __future__ import annotations

from datetime import datetime
from typing import Any

from pydantic import BaseModel, Field


class _GenjaModel(BaseModel):
    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")

    def __getitem__(self, key: str) -> Any:
        return getattr(self, key)


class TaskInfo(_GenjaModel):
    """Task metadata passed into Python task ``run(...)`` methods."""

    name: str
    plugin_name: str
    sub_task: TaskInfo | None = None


class Host(_GenjaModel):
    """Host payload passed into Python task ``run(...)`` methods."""

    hostname: str
    port: int | None = None
    username: str | None = None
    password: str | None = None
    platform: str | None = None
    data: Any | None = None


class TaskContext(_GenjaModel):
    """Execution context passed into Python task ``run(...)`` methods."""

    current_depth: int = 0
    max_depth: int | None = None


def task(name: str, plugin_name: str, sub_task: type | None = None):
    """Attach Genja task metadata to a Python task class."""

    def wrap(cls):
        cls.__genja_task_info__ = {
            "name": name,
            "plugin_name": plugin_name,
            "sub_task": sub_task,
        }
        return cls

    return wrap


class TaskMessage(_GenjaModel):
    """A structured message attached to a task result."""

    level: str
    text: str
    code: str | None = None
    timestamp: datetime | None = None


class TaskSuccessResult(_GenjaModel):
    """Successful task outcome returned from ``run(...)``."""

    status: str = "passed"
    result: Any | None = None
    changed: bool = False
    diff: str | None = None
    summary: str | None = None
    warnings: list[str] = Field(default_factory=list)
    messages: list[TaskMessage] = Field(default_factory=list)
    metadata: Any | None = None


class TaskFailureResult(_GenjaModel):
    """Failed task outcome returned from ``run(...)``."""

    message: str
    status: str = "failed"
    kind: str = "external"
    retryable: bool = False
    details: Any | None = None
    warnings: list[str] = Field(default_factory=list)
    messages: list[TaskMessage] = Field(default_factory=list)


class TaskSkipResult(_GenjaModel):
    """Skipped task outcome returned from ``run(...)``."""

    status: str = "skipped"
    reason: str | None = None
    message: str | None = None


__all__ = [
    "task",
    "TaskInfo",
    "Host",
    "TaskContext",
    "TaskMessage",
    "TaskSuccessResult",
    "TaskFailureResult",
    "TaskSkipResult",
]
