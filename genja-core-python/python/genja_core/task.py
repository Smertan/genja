"""Python task authoring API for Genja.

Import task-facing helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.task`` is the primary public surface for:

- ``@task(...)`` task metadata decoration
- task processor selection metadata
- ``TaskMessage``
- ``TaskSuccessResult``
- ``TaskFailureResult``
- ``TaskSkipResult``
- task ``options`` metadata

The canonical authoring shape is:

.. code-block:: python

    from genja_core.task import (
        Host,
        TaskExecutionContext,
        TaskInfo,
        TaskMessage,
        TaskSuccessResult,
        task,
    )

    @task(
        name="backup_config",
        connection_plugin_name="ssh",
        processors=["audit"],
        options={"backup_path": "/tmp/configs", "compress": True},
    )
    class BackupConfigTask:
        def run(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskExecutionContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(
                changed=True,
                summary=(
                    f"backed up {host.hostname} to "
                    f"{task.options['backup_path']}"
                ),
                messages=[
                    TaskMessage(
                        level="info",
                        text=(
                            f"task={task.name} "
                            f"depth={context.current_depth}/{context.max_depth}"
                        ),
                    )
                ],
                metadata={
                    "platform": host.platform,
                    "backup_path": task.options["backup_path"],
                    "compress": task.options["compress"],
                },
            )

``run(...)`` must return one of:

- ``TaskSuccessResult``
- ``TaskFailureResult``
- ``TaskSkipResult``

Task metadata comes from ``@task(...)``:

- ``name``: required and must be non-empty
- ``connection_plugin_name``: optional; when provided it must be non-empty
- ``sub_task``: optional decorated task class
- ``processors``: optional list of processor plugin names
- ``options``: optional JSON-serializable task options payload
"""

from __future__ import annotations

from datetime import datetime
from typing import Any, Protocol, TypeVar

from pydantic import BaseModel, Field

_TaskClassT = TypeVar("_TaskClassT", bound=type)


class _GenjaModel(BaseModel):
    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")

    def __getitem__(self, key: str) -> Any:
        return getattr(self, key)


class TaskInfo(_GenjaModel):
    """Task metadata passed into Python task ``run(...)`` methods."""

    name: str
    connection_plugin_name: str | None = None
    processors: list[str] = Field(default_factory=list)
    options: Any | None = None
    sub_task: TaskInfo | None = None


class Host(_GenjaModel):
    """Host payload passed into Python task ``run(...)`` methods."""

    hostname: str
    port: int | None = None
    username: str | None = None
    password: str | None = None
    platform: str | None = None
    data: Any | None = None


class TaskExecutionContext(_GenjaModel):
    """Execution context passed into Python task ``run(...)`` methods."""

    current_depth: int = 0
    max_depth: int | None = None


class GenjaTaskProtocol(Protocol):
    """Structural typing contract for Python-authored Genja task classes."""

    __genja_task_info__: dict[str, Any]

    def run(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskExecutionContext,
    ) -> TaskSuccessResult | TaskFailureResult | TaskSkipResult: ...


def task(
    name: str,
    connection_plugin_name: str | None = None,
    sub_task: type[GenjaTaskProtocol] | None = None,
    processors: list[str] | None = None,
    options: Any | None = None,
):
    """Attach Genja task metadata to a Python task class."""

    def wrap(cls: _TaskClassT) -> _TaskClassT:
        if not isinstance(cls, type):
            raise TypeError("@task can only decorate classes")

        run = getattr(cls, "run", None)
        if run is None:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' must define a 'run' method"
            )
        if not callable(run):
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' attribute 'run' must be callable"
            )

        if sub_task is not None:
            if not isinstance(sub_task, type):
                raise TypeError(
                    f"@task-decorated class '{cls.__name__}' sub_task must be a task class or None"
                )
            if not hasattr(sub_task, "__genja_task_info__"):
                raise TypeError(
                    f"@task-decorated class '{cls.__name__}' sub_task '{sub_task.__name__}' must also be decorated with @task"
                )
        if connection_plugin_name is not None:
            if (
                not isinstance(connection_plugin_name, str)
                or not connection_plugin_name.strip()
            ):
                raise TypeError(
                    f"@task-decorated class '{cls.__name__}' connection_plugin_name must be a non-empty string or None"
                )
        if processors is not None:
            if not isinstance(processors, list):
                raise TypeError(
                    f"@task-decorated class '{cls.__name__}' processors must be a list of processor names or None"
                )
            for processor_name in processors:
                if not isinstance(processor_name, str) or not processor_name.strip():
                    raise TypeError(
                        f"@task-decorated class '{cls.__name__}' processors must contain non-empty strings"
                    )

        cls.__genja_task_info__ = {
            "name": name,
            "connection_plugin_name": connection_plugin_name,
            "processors": list(processors or []),
            "options": options,
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
    "GenjaTaskProtocol",
    "TaskInfo",
    "Host",
    "TaskExecutionContext",
    "TaskMessage",
    "TaskSuccessResult",
    "TaskFailureResult",
    "TaskSkipResult",
]
