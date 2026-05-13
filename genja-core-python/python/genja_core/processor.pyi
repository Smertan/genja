from __future__ import annotations

from typing import Any, Protocol

class TaskProcessorContext:
    task_name: str
    parent_task_name: str | None
    depth: int
    hostname: str | None

    @property
    def is_sub_task(self) -> bool: ...
    def to_dict(self) -> dict[str, Any]: ...

class TaskProcessorProtocol(Protocol):
    def name(self) -> str: ...
    def group(self) -> str: ...
    def on_task_start(
        self, context: TaskProcessorContext, results: Any
    ) -> Any | None: ...
    def on_task_finish(
        self, context: TaskProcessorContext, results: Any
    ) -> Any | None: ...
    def on_instance_start(self, context: TaskProcessorContext) -> None: ...
    def on_instance_finish(
        self, context: TaskProcessorContext, result: Any
    ) -> Any | None: ...

__all__: list[str]
