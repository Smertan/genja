from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any, Awaitable, ClassVar
from .plugin import PluginBase

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

class ConnectionBase(ABC):
    @abstractmethod
    def open(
        self,
        params: ResolvedConnectionParams,
    ) -> None | Awaitable[None]: ...
    def execute_command(self, command: str) -> str | Awaitable[str]: ...
    @abstractmethod
    def close(
        self,
    ) -> (
        ConnectionKey
        | dict[str, Any]
        | None
        | Awaitable[ConnectionKey | dict[str, Any] | None]
    ): ...
    @abstractmethod
    def is_alive(self) -> bool | Awaitable[bool]: ...

class ConnectionPluginBase(PluginBase):
    group_name: ClassVar[str]
    _locked_group_name: ClassVar[str]
    @abstractmethod
    def create(
        self,
        key: ConnectionKey,
    ) -> ConnectionBase | Awaitable[ConnectionBase]: ...

__all__: list[str]
