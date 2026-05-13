"""Python connection plugin authoring API for Genja.

Import connection-facing helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.connection`` is the primary public surface for:

- ``ConnectionKey``
- ``ResolvedConnectionParams``
- ``ConnectionPluginProtocol``
- ``ConnectionProtocol``

Connection plugins are registered on ``PluginManager`` and selected by task
metadata:

Connection factories and connection methods may be implemented as either
``def`` or ``async def``; Genja will resolve either form.

.. code-block:: python

    import genja_core
    from genja_core.connection import ConnectionKey, ResolvedConnectionParams

    class NetmikoConnection:
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

    class NetmikoPlugin:
        def name(self) -> str:
            return "ssh"

        def group(self) -> str:
            return "ConnectionPlugin"

        def create(self, key: ConnectionKey) -> NetmikoConnection:
            return NetmikoConnection(key)

    plugins = genja_core.PluginManager()
    plugins.register_plugin(NetmikoPlugin())
"""

from __future__ import annotations

from typing import Any, Awaitable, Protocol

from pydantic import BaseModel


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


class ConnectionProtocol(Protocol):
    def open(
        self,
        params: ResolvedConnectionParams,
    ) -> None | Awaitable[None]: ...

    def execute_command(self, command: str) -> str | Awaitable[str]: ...

    def close(
        self,
    ) -> (
        ConnectionKey
        | dict[str, Any]
        | None
        | Awaitable[ConnectionKey | dict[str, Any] | None]
    ): ...

    def is_alive(self) -> bool | Awaitable[bool]: ...


class ConnectionPluginProtocol(Protocol):
    def name(self) -> str: ...

    def group(self) -> str: ...

    def create(
        self,
        key: ConnectionKey,
    ) -> ConnectionProtocol | Awaitable[ConnectionProtocol]: ...


__all__ = [
    "ConnectionKey",
    "ResolvedConnectionParams",
    "ConnectionProtocol",
    "ConnectionPluginProtocol",
]
