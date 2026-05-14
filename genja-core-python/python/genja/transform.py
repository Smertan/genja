"""Python transform-function plugin authoring API for Genja.

Import transform-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.transform`` is the primary public surface for:

- ``TransformFunctionPluginProtocol``

Transform-function plugins are registered on ``PluginManager`` and selected
through ``Settings.inventory.transform_function``. The transform callbacks may
be implemented as either ``def`` or ``async def``; Genja will resolve either
form.

.. code-block:: python

    import genja
    from genja.transform import TransformFunctionPluginProtocol

    class HostnameSuffixTransform:
        def name(self) -> str:
            return "python_transform"

        def group(self) -> str:
            return "TransformFunctionPlugin"

        def transform_host(self, host, options):
            suffix = (options or {}).get("suffix", "")
            return {
                **host,
                "hostname": f"{host['hostname']}{suffix}",
            }

    plugins = genja.PluginManager()
    plugins.register_plugin(HostnameSuffixTransform())
"""

from __future__ import annotations

from typing import Any, Awaitable, Protocol


class TransformFunctionPluginProtocol(Protocol):
    """Structural typing contract for Python-authored transform plugins."""

    def name(self) -> str: ...

    def group(self) -> str: ...

    def transform_host(
        self,
        host: dict[str, Any],
        options: Any | None,
    ) -> dict[str, Any] | Awaitable[dict[str, Any]]: ...

    def transform_group(
        self,
        group: dict[str, Any],
        options: Any | None,
    ) -> dict[str, Any] | Awaitable[dict[str, Any]]: ...

    def transform_defaults(
        self,
        defaults: dict[str, Any],
        options: Any | None,
    ) -> dict[str, Any] | Awaitable[dict[str, Any]]: ...


__all__ = ["TransformFunctionPluginProtocol"]
