"""Python inventory plugin authoring API for Genja.

Import inventory-facing helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.inventory`` is the primary public surface for:

- ``InventoryPluginProtocol``

Inventory plugins are registered on ``PluginManager`` and selected through
``Settings.inventory.plugin``. The plugin ``load()`` method receives the current
``Settings`` object plus a read-only plugin-registry view exposing
``plugin_names()`` and ``plugin_names_and_groups()``.

Inventory plugins may be implemented as either ``def`` or ``async def``; Genja
will resolve either form. They should return a host mapping in the same shape
accepted by ``Genja.from_hosts(...)``:

.. code-block:: python

    import genja_core
    from genja_core.inventory import InventoryPluginProtocol

    class StaticInventoryPlugin:
        def name(self) -> str:
            return "python_inventory"

        def group(self) -> str:
            return "InventoryPlugin"

        def load(self, settings, plugins):
            return {
                "router1": {
                    "hostname": "10.0.0.1",
                    "platform": "ios",
                }
            }

    manager = genja_core.PluginManager()
    manager.register_plugin(StaticInventoryPlugin())
"""

from __future__ import annotations

from typing import Any, Awaitable, Protocol

from .settings import Settings


class InventoryPluginProtocol(Protocol):
    """Structural typing contract for Python-authored inventory plugins."""

    def name(self) -> str: ...

    def group(self) -> str: ...

    def load(
        self,
        settings: Settings,
        plugins: Any,
    ) -> dict[str, Any] | Awaitable[dict[str, Any]]: ...


__all__ = ["InventoryPluginProtocol"]
