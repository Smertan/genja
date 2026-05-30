"""Python transform-function plugin authoring API for Genja.

Import transform-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.transform`` is the primary public surface for:

- ``TransformFunctionPluginBase``

Transform-function plugins are registered on ``PluginManager`` and selected
through ``Settings.inventory.transform_function``. The transform callbacks may
be implemented as either ``def`` or ``async def``; Genja will resolve either
form. A plugin may implement one, multiple, or all transform callbacks. Missing
callbacks pass the original inventory value through unchanged.

.. code-block:: python

    import genja
    from genja.transform import TransformFunctionPluginBase

    class HostnameSuffixTransform(TransformFunctionPluginBase):
        name = "python_transform"

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

from typing import Any

from .plugin import PluginBase


class TransformFunctionPluginBase(PluginBase):
    """Base class for Python-authored transform-function plugins."""

    group_name = "TransformFunctionPlugin"
    _locked_group_name = "TransformFunctionPlugin"

    def transform_host(self, host: dict[str, Any], options: Any | None) -> dict[str, Any]:
        return host

    def transform_group(self, group: dict[str, Any], options: Any | None) -> dict[str, Any]:
        return group

    def transform_defaults(self, defaults: dict[str, Any], options: Any | None) -> dict[str, Any]:
        return defaults


__all__ = [
    "TransformFunctionPluginBase",
]
