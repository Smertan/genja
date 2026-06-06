"""Python connection plugin authoring API for Genja.

Import connection-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.connection`` is the primary public surface for:

- ``ConnectionKey``
- ``ResolvedConnectionParams``
- ``ConnectionPluginBase``
- ``ConnectionBase``

Connection plugins are registered on ``PluginManager`` and selected by task
metadata:

Connection factories and connection methods may be implemented as either
``def`` or ``async def``; Genja will resolve either form.

.. code-block:: python

    import genja
    from genja.connection import ConnectionBase, ConnectionKey, ConnectionPluginBase, ResolvedConnectionParams

    class NetmikoConnection(ConnectionBase):
        def __init__(self, key: ConnectionKey):
            self.key = key
            self.alive = False

        def open(self, params: ResolvedConnectionParams) -> None:
            self.alive = True

        def close(self) -> ConnectionKey:
            self.alive = False
            return self.key

        def is_alive(self) -> bool:
            return self.alive

    class NetmikoPlugin(ConnectionPluginBase):
        name = "ssh"

        def create(self, key: ConnectionKey) -> NetmikoConnection:
            return NetmikoConnection(key)

    plugins = genja.PluginManager()
    plugins.register_plugin(NetmikoPlugin())
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any, Awaitable

from pydantic import BaseModel
from .plugin import PluginBase


class _GenjaModel(BaseModel):
    def to_dict(self) -> dict[str, Any]:
        return self.model_dump(mode="json")

    def __getitem__(self, key: str) -> Any:
        return getattr(self, key)


class ConnectionKey(_GenjaModel):
    hostname: str
    plugin_name: str


class ResolvedConnectionParams(_GenjaModel):
    hostname: str
    port: int | None = None
    username: str | None = None
    password: str | None = None
    platform: str | None = None
    extras: Any | None = None


class ConnectionBase(ABC):
    """Base class for Python-authored connection instances."""

    @abstractmethod
    def open(
        self,
        params: ResolvedConnectionParams,
    ) -> None | Awaitable[None]:
        ...

    def execute_command(self, command: str) -> str | Awaitable[str]:
        raise NotImplementedError("connection does not implement execute_command")

    @abstractmethod
    def close(
        self,
    ) -> ConnectionKey | dict[str, Any] | None | Awaitable[ConnectionKey | dict[str, Any] | None]:
        ...

    @abstractmethod
    def is_alive(self) -> bool | Awaitable[bool]:
        ...


class ConnectionPluginBase(PluginBase):
    """Base class for Python-authored connection plugins."""

    group_name = "ConnectionPlugin"
    _locked_group_name = "ConnectionPlugin"

    @abstractmethod
    def create(
        self,
        key: ConnectionKey,
    ) -> ConnectionBase | Awaitable[ConnectionBase]:
        ...


__all__ = [
    "ConnectionKey",
    "ResolvedConnectionParams",
    "ConnectionBase",
    "ConnectionPluginBase",
]
