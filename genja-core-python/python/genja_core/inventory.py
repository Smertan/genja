"""Python inventory models and plugin authoring API for Genja.

Import inventory-facing helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.inventory`` is the primary public surface for:

- ``ConnectionOptions``
- ``Host``
- ``Group``
- ``Defaults``
- ``Inventory``
- ``InventoryPluginProtocol``

Inventory plugins are registered on ``PluginManager`` and selected through
``Settings.inventory.plugin``. The plugin ``load()`` method receives the current
``Settings`` object plus a read-only plugin-registry view exposing
``plugin_names()`` and ``plugin_names_and_groups()``.

Inventory plugins may be implemented as either ``def`` or ``async def``; Genja
will resolve either form. They may return either a host mapping in the same
shape accepted by ``Genja.from_hosts(...)`` or a full ``Inventory`` payload:

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

from pydantic import BaseModel
from .settings import Settings


class _GenjaModel(BaseModel):
    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json", exclude_none=True)

    def __getitem__(self, key: str) -> Any:
        return getattr(self, key)


class ConnectionOptions(_GenjaModel):
    hostname: str | None = None
    port: int | None = None
    username: str | None = None
    password: str | None = None
    platform: str | None = None
    extras: Any | None = None


class Host(_GenjaModel):
    hostname: str | None = None
    port: int | None = None
    username: str | None = None
    password: str | None = None
    platform: str | None = None
    groups: list[str] | None = None
    data: Any | None = None
    connection_options: dict[str, ConnectionOptions | dict[str, Any]] | None = None


class Group(_GenjaModel):
    hostname: str | None = None
    port: int | None = None
    username: str | None = None
    password: str | None = None
    platform: str | None = None
    groups: list[str] | None = None
    data: Any | None = None
    connection_options: dict[str, ConnectionOptions | dict[str, Any]] | None = None


class Defaults(_GenjaModel):
    hostname: str | None = None
    port: int | None = None
    username: str | None = None
    password: str | None = None
    platform: str | None = None
    data: Any | None = None
    connection_options: dict[str, ConnectionOptions | dict[str, Any]] | None = None


class Inventory(_GenjaModel):
    hosts: dict[str, Host | dict[str, Any]]
    groups: dict[str, Group | dict[str, Any]] | None = None
    defaults: Defaults | dict[str, Any] | None = None


class InventoryPluginProtocol(Protocol):
    """Structural typing contract for Python-authored inventory plugins."""

    def name(self) -> str: ...

    def group(self) -> str: ...

    def load(
        self,
        settings: Settings,
        plugins: Any,
    ) -> Inventory | dict[str, Any] | Awaitable[Inventory | dict[str, Any]]: ...


__all__ = [
    "ConnectionOptions",
    "Host",
    "Group",
    "Defaults",
    "Inventory",
    "InventoryPluginProtocol",
]
