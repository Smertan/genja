"""Python processor authoring API for Genja.

Import processor-facing helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.processor`` is the primary public surface for:

- ``TaskProcessorContext``
- ``TaskProcessorProtocol``

Processor plugins are registered on ``PluginManager`` and selected by task
metadata:

.. code-block:: python

    import genja_core
    from genja_core.processor import TaskProcessorContext
    from genja_core.task import Host, TaskSuccessResult, task

    class AuditProcessor:
        def name(self) -> str:
            return "audit"

        def group(self) -> str:
            return "ProcessorPlugin"

        def on_instance_finish(self, context: TaskProcessorContext, result):
            data = result.to_dict()
            data["metadata"] = {
                **(data.get("metadata") or {}),
                "processed_by": context.task_name,
            }
            return data

    @task(name="backup_config", connection_plugin_name="ssh", processors=["audit"])
    class BackupConfigTask:
        def run(self, task, host, context):
            return TaskSuccessResult(summary=f"backed up {host.hostname}")

    plugins = genja_core.PluginManager()
    plugins.register_plugin(AuditProcessor())
"""

from __future__ import annotations

from typing import Any, Protocol

from pydantic import BaseModel


class _GenjaModel(BaseModel):
    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")

    def __getitem__(self, key: str) -> Any:
        return getattr(self, key)


class TaskProcessorContext(_GenjaModel):
    """Execution context passed into Python processor callbacks."""

    task_name: str
    parent_task_name: str | None = None
    depth: int = 0
    hostname: str | None = None

    @property
    def is_sub_task(self) -> bool:
        return self.parent_task_name is not None


class TaskProcessorProtocol(Protocol):
    """Structural typing contract for Python-authored processor plugins."""

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


__all__ = ["TaskProcessorContext", "TaskProcessorProtocol"]
