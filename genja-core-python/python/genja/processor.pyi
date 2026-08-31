"""Python processor authoring API for Genja.

Import processor-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.processor`` is the primary public surface for:

- ``TaskProcessorContext``
- ``ProcessorPluginBase``

Processor plugins are registered on ``PluginManager`` and selected by task
metadata. A processor may implement one, multiple, or all lifecycle hooks.
Missing hooks are skipped.
"""

from __future__ import annotations

from typing import Any, ClassVar

from .plugin import PluginBase

class TaskProcessorContext:
    """Execution context passed into Python processor callbacks."""

    task_name: str
    """Name of the task currently being processed."""

    parent_task_name: str | None
    """Name of the parent task when processing a sub-task."""

    depth: int
    """Task nesting depth, where top-level tasks have depth zero."""

    hostname: str | None
    """Hostname for per-host callbacks, if the callback is host-specific."""

    @property
    def is_sub_task(self) -> bool:
        """Return whether the current task is running as a sub-task."""
        ...

    def to_dict(self) -> dict[str, Any]:
        """Return the processor context as a dictionary."""
        ...

class ProcessorPluginBase(PluginBase):
    """Base class for Python-authored processor plugins."""

    group_name: ClassVar[str]
    """Plugin group name used by Genja's plugin registry."""

    _locked_group_name: ClassVar[str]
    """Internal locked plugin group name."""

    def on_task_start(self, context: TaskProcessorContext, results: Any) -> Any | None:
        """Handle the start of a task-level result collection.

        Return a replacement value to modify the results object, or ``None`` to
        leave it unchanged.
        """
        ...

    def on_task_finish(self, context: TaskProcessorContext, results: Any) -> Any | None:
        """Handle the completed task-level result collection.

        Return a replacement value to modify the results object, or ``None`` to
        leave it unchanged.
        """
        ...

    def on_instance_start(self, context: TaskProcessorContext) -> None:
        """Handle the start of a host task instance."""
        ...

    def on_instance_finish(
        self, context: TaskProcessorContext, result: Any
    ) -> Any | None:
        """Handle a completed host task instance result.

        Return a replacement value to modify the instance result, or ``None`` to
        leave it unchanged.
        """
        ...

__all__: list[str]
