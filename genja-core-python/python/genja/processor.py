"""Python processor authoring API for Genja.

Import processor-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.processor`` is the primary public surface for:

- ``TaskProcessorContext``
- ``ProcessorPluginBase``

Processor plugins are registered on ``PluginManager`` and selected by task
metadata. A processor may implement one, multiple, or all lifecycle hooks.
Missing hooks are skipped.

.. code-block:: python

    import genja
    from genja.processor import ProcessorPluginBase, TaskProcessorContext
    from genja.task import Host, TaskSuccessResult, task

    class AuditProcessor(ProcessorPluginBase):
        name = "audit"

        def on_instance_finish(self, context: TaskProcessorContext, result):
            data = result.to_dict()
            data["metadata"] = {
                **(data.get("metadata") or {}),
                "processed_by": context.task_name,
            }
            return data

    @task(name="backup_config", connection_plugin_name="ssh", processors=["audit"])
    class BackupConfigTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary=f"backed up {host.hostname}")

    plugins = genja.PluginManager()
    plugins.register_plugin(AuditProcessor())
"""

from __future__ import annotations

from typing import Any

from pydantic import BaseModel
from .plugin import PluginBase


class _GenjaModel(BaseModel):
    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")

    def __getitem__(self, key: str) -> Any:
        return getattr(self, key)


class TaskProcessorContext(_GenjaModel):
    """Execution context passed into Python processor callbacks."""

    task_name: str
    """Name of the task currently being processed."""

    parent_task_name: str | None = None
    """Name of the parent task when processing a sub-task."""

    depth: int = 0
    """Task nesting depth, where top-level tasks have depth zero."""

    hostname: str | None = None
    """Hostname for per-host callbacks, if the callback is host-specific."""

    @property
    def is_sub_task(self) -> bool:
        """Return whether the current task is running as a sub-task."""
        return self.parent_task_name is not None


class ProcessorPluginBase(PluginBase):
    """Base class for Python-authored processor plugins."""

    group_name = "ProcessorPlugin"
    """Plugin group name used by Genja's plugin registry."""

    _locked_group_name = "ProcessorPlugin"
    """Internal locked plugin group name."""

    def on_task_start(self, context: TaskProcessorContext, results: Any) -> Any | None:
        """Handle the start of a task-level result collection.

        Return a replacement value to modify the results object, or ``None`` to
        leave it unchanged.
        """
        return None

    def on_task_finish(self, context: TaskProcessorContext, results: Any) -> Any | None:
        """Handle the completed task-level result collection.

        Return a replacement value to modify the results object, or ``None`` to
        leave it unchanged.
        """
        return None

    def on_instance_start(self, context: TaskProcessorContext) -> None:
        """Handle the start of a host task instance."""
        return None

    def on_instance_finish(
        self, context: TaskProcessorContext, result: Any
    ) -> Any | None:
        """Handle a completed host task instance result.

        Return a replacement value to modify the instance result, or ``None`` to
        leave it unchanged.
        """
        return None


__all__ = [
    "TaskProcessorContext",
    "ProcessorPluginBase",
]
