"""Python runner plugin authoring API for Genja.

Import runner-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.runner`` is the primary public surface for:

- ``RunnerPluginProtocol``

Runner plugins are registered on ``PluginManager`` and selected through
``Genja.with_runner(...)`` or ``Settings.runner.plugin``. A runner receives a
task definition wrapper plus a host mapping and may orchestrate execution by
calling ``task.run_on_host(...)`` or ``task.run_on_hosts(...)``. Runners may
also implement ``run_tasks(...)`` for custom ordered task-list execution; when
omitted, the Rust bridge delegates each root task to ``run_task(...)`` in order.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Awaitable, Protocol

from .settings import RunnerConfig

if TYPE_CHECKING:
    from . import TaskConnectionResolver, TaskDefinition, TaskResults


class RunnerPluginProtocol(Protocol):
    """Structural typing contract for Python-authored runner plugins."""

    def name(self) -> str: ...

    def group(self) -> str: ...

    def run_task(
        self,
        task: TaskDefinition,
        hosts: dict[str, object],
        connection_resolver: TaskConnectionResolver | None,
        runner_config: RunnerConfig,
        max_depth: int,
    ) -> TaskResults | Awaitable[TaskResults]: ...


class BatchRunnerPluginProtocol(RunnerPluginProtocol, Protocol):
    """Optional extension for runners with custom task-list execution."""

    def run_tasks(
        self,
        tasks: list[TaskDefinition],
        hosts: dict[str, object],
        connection_resolver: TaskConnectionResolver | None,
        runner_config: RunnerConfig,
        max_depth: int,
    ) -> list[TaskResults] | Awaitable[list[TaskResults]]: ...


__all__ = ["RunnerPluginProtocol", "BatchRunnerPluginProtocol"]
