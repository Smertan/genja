"""Python task authoring API for Genja.

Import task-facing helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.task`` is the primary public surface for:

- ``@task(...)`` task metadata decoration
- ``TaskMessage``
- ``TaskSuccessResult``
- ``TaskFailureResult``
- ``TaskSkipResult``
"""

from datetime import datetime
from typing import Any

from pydantic import BaseModel, Field


def task(name: str, plugin_name: str, sub_task=None):
    def wrap(cls):
        cls.__genja_task_info__ = {
            "name": name,
            "plugin_name": plugin_name,
            "sub_task": sub_task,
        }
        return cls

    return wrap


class TaskMessage(BaseModel):
    level: str
    text: str
    code: str | None = None
    timestamp: datetime | None = None

    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")


class TaskSuccessResult(BaseModel):
    status: str = "passed"
    result: Any | None = None
    changed: bool = False
    diff: str | None = None
    summary: str | None = None
    warnings: list[str] = Field(default_factory=list)
    messages: list[TaskMessage] = Field(default_factory=list)
    metadata: Any | None = None

    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")


class TaskFailureResult(BaseModel):
    message: str
    status: str = "failed"
    kind: str = "external"
    retryable: bool = False
    details: Any | None = None
    warnings: list[str] = Field(default_factory=list)
    messages: list[TaskMessage] = Field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")


class TaskSkipResult(BaseModel):
    status: str = "skipped"
    reason: str | None = None
    message: str | None = None

    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")


__all__ = [
    "task",
    "TaskMessage",
    "TaskSuccessResult",
    "TaskFailureResult",
    "TaskSkipResult",
]
