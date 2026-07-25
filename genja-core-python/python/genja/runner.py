"""Python runner plugin authoring API for Genja.

Import runner-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.runner`` is the primary public surface for:

- ``RunnerPluginBase``

Runner plugins are registered on ``PluginManager`` and selected through
``Genja.with_runner(...)`` or ``Settings.runner.plugin``. A runner receives a
task definition wrapper plus a host mapping and may orchestrate execution by
calling ``task.run_on_host(...)`` or ``task.run_on_hosts(...)``. Runners may
also implement ``run_tasks(...)`` for custom ordered task-list execution; when
omitted, the Rust bridge delegates each root task to ``run_task(...)`` in order.
Runner methods may be implemented as either ``def`` or ``async def``; Genja
will resolve either form. Runner callbacks receive ``run_options`` so they can
preserve operator-selected execution controls such as maximum task depth and
dry-run mode when delegating to task definitions.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import TYPE_CHECKING, Awaitable

from .plugin import PluginBase
from .settings import RunnerConfig

if TYPE_CHECKING:
    from . import TaskConnectionResolver, TaskDefinition, TaskResults, TaskRunOptions


class RunnerPluginBase(PluginBase):
    """Base class for Python-authored runner plugins."""

    group_name = "RunnerPlugin"
    _locked_group_name = "RunnerPlugin"

    @abstractmethod
    def run_task(
        self,
        task: TaskDefinition,
        hosts: dict[str, object],
        connection_resolver: TaskConnectionResolver | None,
        runner_config: RunnerConfig,
        run_options: TaskRunOptions,
    ) -> TaskResults | Awaitable[TaskResults]: ...


class BatchRunnerPluginBase(RunnerPluginBase, ABC):
    """Base class for runners with custom task-list execution."""

    @abstractmethod
    def run_tasks(
        self,
        tasks: list[TaskDefinition],
        hosts: dict[str, object],
        connection_resolver: TaskConnectionResolver | None,
        runner_config: RunnerConfig,
        run_options: TaskRunOptions,
    ) -> list[TaskResults] | Awaitable[list[TaskResults]]: ...


__all__ = ["RunnerPluginBase", "BatchRunnerPluginBase"]
