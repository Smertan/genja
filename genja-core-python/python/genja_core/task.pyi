from __future__ import annotations

from datetime import datetime
from typing import Any, Callable, TypeVar

_T = TypeVar("_T", bound=type)


class TaskInfo:
    name: str
    plugin_name: str
    sub_task: TaskInfo | None

    def to_dict(self) -> dict[str, Any]: ...
    def __getitem__(self, key: str) -> Any: ...


class Host:
    hostname: str
    port: int | None
    username: str | None
    password: str | None
    platform: str | None
    data: Any | None

    def to_dict(self) -> dict[str, Any]: ...
    def __getitem__(self, key: str) -> Any: ...


class TaskContext:
    current_depth: int
    max_depth: int | None

    def to_dict(self) -> dict[str, Any]: ...
    def __getitem__(self, key: str) -> Any: ...


def task(
    name: str,
    plugin_name: str,
    sub_task: type | None = None,
) -> Callable[[_T], _T]: ...


class TaskMessage:
    level: str
    text: str
    code: str | None
    timestamp: datetime | None

    def __init__(
        self,
        *,
        level: str,
        text: str,
        code: str | None = None,
        timestamp: datetime | None = None,
    ) -> None: ...

    def to_dict(self) -> dict[str, Any]: ...
    def __getitem__(self, key: str) -> Any: ...


class TaskSuccessResult:
    status: str
    result: Any | None
    changed: bool
    diff: str | None
    summary: str | None
    warnings: list[str]
    messages: list[TaskMessage]
    metadata: Any | None

    def __init__(
        self,
        *,
        status: str = "passed",
        result: Any | None = None,
        changed: bool = False,
        diff: str | None = None,
        summary: str | None = None,
        warnings: list[str] | None = None,
        messages: list[TaskMessage] | None = None,
        metadata: Any | None = None,
    ) -> None: ...

    def to_dict(self) -> dict[str, Any]: ...
    def __getitem__(self, key: str) -> Any: ...


class TaskFailureResult:
    message: str
    status: str
    kind: str
    retryable: bool
    details: Any | None
    warnings: list[str]
    messages: list[TaskMessage]

    def __init__(
        self,
        *,
        message: str,
        status: str = "failed",
        kind: str = "external",
        retryable: bool = False,
        details: Any | None = None,
        warnings: list[str] | None = None,
        messages: list[TaskMessage] | None = None,
    ) -> None: ...

    def to_dict(self) -> dict[str, Any]: ...
    def __getitem__(self, key: str) -> Any: ...


class TaskSkipResult:
    status: str
    reason: str | None
    message: str | None

    def __init__(
        self,
        *,
        status: str = "skipped",
        reason: str | None = None,
        message: str | None = None,
    ) -> None: ...

    def to_dict(self) -> dict[str, Any]: ...
    def __getitem__(self, key: str) -> Any: ...


__all__: list[str]
