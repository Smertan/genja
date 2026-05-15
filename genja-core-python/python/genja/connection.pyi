from __future__ import annotations

from typing import Any, Awaitable, Protocol

class ConnectionKey:
    hostname: str
    plugin_name: str

    def to_dict(self) -> dict[str, Any]: ...

class ResolvedConnectionParams:
    hostname: str
    port: int | None
    username: str | None
    password: str | None
    platform: str | None
    extras: Any | None

    def to_dict(self) -> dict[str, Any]: ...

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

__all__: list[str]
