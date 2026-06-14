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
    parent_task_name: str | None = None
    depth: int = 0
    hostname: str | None = None

    @property
    def is_sub_task(self) -> bool:
        return self.parent_task_name is not None


class ProcessorPluginBase(PluginBase):
    """Base class for Python-authored processor plugins."""

    group_name = "ProcessorPlugin"
    _locked_group_name = "ProcessorPlugin"

    def on_task_start(self, context: TaskProcessorContext, results: Any) -> Any | None:
        return None

    def on_task_finish(self, context: TaskProcessorContext, results: Any) -> Any | None:
        return None

    def on_instance_start(self, context: TaskProcessorContext) -> None:
        return None

    def on_instance_finish(
        self, context: TaskProcessorContext, result: Any
    ) -> Any | None:
        return None


__all__ = [
    "TaskProcessorContext",
    "ProcessorPluginBase",
]
