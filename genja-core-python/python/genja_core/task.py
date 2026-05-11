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
        TaskRuntimeContext,
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
            context: TaskRuntimeContext,
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

``run(...)`` may be implemented as ``def`` or ``async def`` and must resolve to
one of:

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
from enum import Enum
from typing import Any, Awaitable, Literal, Protocol, TypeVar, cast

from pydantic import BaseModel, Field

_TaskClassT = TypeVar("_TaskClassT", bound=type)


class _GenjaModel(BaseModel):
    """Base model class for Genja data structures with dictionary-like access.
    
    Extends Pydantic's BaseModel to provide convenient dictionary conversion
    and attribute access via subscript notation for all Genja model classes.
    """

    def to_dict(self) -> dict[str, Any]:
        """Convert the model instance to a JSON-serializable dictionary.
        
        Returns:
            dict[str, Any]: A dictionary representation of the model with all
                fields serialized in JSON-compatible format.
        """
        return self.model_dump(mode="json")

    def __getitem__(self, key: str) -> Any:
        """Enable dictionary-style attribute access using subscript notation.
        
        Args:
            key (str): The name of the attribute to retrieve.
        
        Returns:
            Any: The value of the requested attribute.
        
        Raises:
            AttributeError: If the specified attribute does not exist on the model.
        """
        return getattr(self, key)


class TaskInfo(_GenjaModel):
    """Task metadata passed into Python task ``run(...)`` methods."""

    name: str = Field(description="Unique task name.")
    connection_plugin_name: str | None = Field(
        default=None,
        description="Connection plugin name used to execute this task.",
    )
    processors: list[str] = Field(
        default_factory=list,
        description="Processor plugin names applied to this task.",
    )
    options: Any | None = Field(
        default=None,
        description="JSON-serializable task options payload.",
    )
    sub_task: TaskInfo | None = Field(
        default=None,
        description="Nested task metadata for an optional sub-task.",
    )


class Host(_GenjaModel):
    """Host payload passed into Python task ``run(...)`` methods."""

    hostname: str = Field(description="Inventory hostname for the current target.")
    port: int | None = Field(
        default=None,
        description="Network port used for the current host connection.",
    )
    username: str | None = Field(
        default=None,
        description="Username used for the current host connection.",
    )
    password: str | None = Field(
        default=None,
        description="Password used for the current host connection.",
    )
    platform: str | None = Field(
        default=None,
        description="Platform identifier associated with the current host.",
    )
    data: Any | None = Field(
        default=None,
        description="Additional inventory data attached to the host.",
    )


class TaskRuntimeContext(_GenjaModel):
    """Runtime context passed into Python task ``run(...)`` methods."""

    current_depth: int = Field(
        default=0,
        description="Current task execution depth.",
    )
    max_depth: int | None = Field(
        default=None,
        description="Maximum allowed execution depth, if configured.",
    )
    connection: Any | None = Field(
        default=None,
        description="Resolved connection object available to the task.",
    )


class GenjaTaskProtocol(Protocol):
    """Structural typing contract for Python-authored Genja task classes."""

    __genja_task_info__: dict[str, Any]

    def run(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult | TaskFailureResult | TaskSkipResult | Awaitable[
        TaskSuccessResult | TaskFailureResult | TaskSkipResult
    ]: ...


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

        task_cls = cast(type[GenjaTaskProtocol], cls)
        task_cls.__genja_task_info__ = {
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

    level: TaskMessageLevel = Field(description="Message severity level.")
    text: str = Field(description="Human-readable message text.")
    code: str | None = Field(
        default=None,
        description="Optional machine-readable message code.",
    )
    timestamp: datetime | None = Field(
        default=None,
        description="Timestamp associated with the message.",
    )


class TaskStatus(str, Enum):
    """Canonical task status values returned by Genja task results."""

    PASSED = "passed"
    FAILED = "failed"
    SKIPPED = "skipped"


class TaskFailureKind(str, Enum):
    """Canonical task failure categories returned by Genja task results."""

    CONNECTION = "connection"
    AUTHENTICATION = "authentication"
    VALIDATION = "validation"
    TIMEOUT = "timeout"
    COMMAND = "command"
    UNSUPPORTED = "unsupported"
    INTERNAL = "internal"
    EXTERNAL = "external"


class TaskMessageLevel(str, Enum):
    """Canonical task message severity levels returned by Genja task results."""

    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    DEBUG = "debug"


class TaskSuccessResult(_GenjaModel):
    """Successful task outcome returned from ``run(...)``."""

    status: Literal[TaskStatus.PASSED] = Field(
        default=TaskStatus.PASSED,
        description="Task status for a successful result.",
    )
    result: Any | None = Field(
        default=None,
        description="Primary task result payload.",
    )
    changed: bool = Field(
        default=False,
        description="Whether the task changed remote or managed state.",
    )
    diff: str | None = Field(
        default=None,
        description="Optional human-readable diff for the applied change.",
    )
    summary: str | None = Field(
        default=None,
        description="Short summary of the successful task outcome.",
    )
    warnings: list[str] = Field(
        default_factory=list,
        description="Non-fatal warnings produced during task execution.",
    )
    messages: list[TaskMessage] = Field(
        default_factory=list,
        description="Structured messages emitted during task execution.",
    )
    metadata: Any | None = Field(
        default=None,
        description="Additional JSON-serializable metadata for the result.",
    )


class TaskFailureResult(_GenjaModel):
    """Failed task outcome returned from ``run(...)``."""

    message: str = Field(description="Human-readable failure message.")
    status: Literal[TaskStatus.FAILED] = Field(
        default=TaskStatus.FAILED,
        description="Task status for a failed result.",
    )
    kind: TaskFailureKind = Field(
        default=TaskFailureKind.EXTERNAL,
        description="Failure category identifier.",
    )
    retryable: bool = Field(
        default=False,
        description="Whether the failure may succeed on retry.",
    )
    details: Any | None = Field(
        default=None,
        description="Additional JSON-serializable failure details.",
    )
    warnings: list[str] = Field(
        default_factory=list,
        description="Non-fatal warnings produced before the failure occurred.",
    )
    messages: list[TaskMessage] = Field(
        default_factory=list,
        description="Structured messages emitted before the failure occurred.",
    )


class TaskSkipResult(_GenjaModel):
    """Skipped task outcome returned from ``run(...)``."""

    status: Literal[TaskStatus.SKIPPED] = Field(
        default=TaskStatus.SKIPPED,
        description="Task status for a skipped result.",
    )
    reason: str | None = Field(
        default=None,
        description="Machine-readable reason the task was skipped.",
    )
    message: str | None = Field(
        default=None,
        description="Human-readable explanation for the skipped task.",
    )


__all__ = [
    "task",
    "GenjaTaskProtocol",
    "TaskInfo",
    "Host",
    "TaskRuntimeContext",
    "TaskMessage",
    "TaskMessageLevel",
    "TaskStatus",
    "TaskFailureKind",
    "TaskSuccessResult",
    "TaskFailureResult",
    "TaskSkipResult",
]
