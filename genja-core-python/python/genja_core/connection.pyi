from __future__ import annotations

from typing import Any, Protocol


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
    def open(self, params: ResolvedConnectionParams) -> None: ...
    def close(self) -> ConnectionKey | dict[str, Any] | None: ...
    def is_alive(self) -> bool: ...


class ConnectionPluginProtocol(Protocol):
    def name(self) -> str: ...
    def group(self) -> str: ...
    def create(self, key: ConnectionKey) -> ConnectionProtocol: ...


__all__: list[str]
