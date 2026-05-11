"""Python runner plugin authoring API for Genja.

Import runner-facing helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.runner`` is the primary public surface for:

- ``RunnerPluginProtocol``

Runner plugins are registered on ``PluginManager`` and selected through
``Genja.with_runner(...)`` or ``Settings.runner.plugin``. A runner receives a
task definition wrapper plus a host mapping and may orchestrate execution by
calling ``task.run_on_host(...)`` or ``task.run_on_hosts(...)``.
"""

from __future__ import annotations

from typing import Any, Awaitable, Protocol

from .settings import RunnerConfig


class RunnerPluginProtocol(Protocol):
    """Structural typing contract for Python-authored runner plugins."""

    def name(self) -> str: ...

    def group(self) -> str: ...

    def run(
        self,
        task: Any,
        hosts: dict[str, Any],
        connection_resolver: Any | None,
        runner_config: RunnerConfig,
        max_depth: int,
    ) -> Any | Awaitable[Any]: ...


__all__ = ["RunnerPluginProtocol"]
