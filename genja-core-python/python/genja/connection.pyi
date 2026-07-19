"""Python connection plugin authoring API for Genja.

Import connection-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.connection`` is the primary public surface for:

- ``ConnectionKey``
- ``ResolvedConnectionParams``
- ``ConnectionPluginBase``
- ``ConnectionBase``

Connection plugins are registered on ``PluginManager`` and selected by task
metadata. Connection factories and connection methods may be implemented as
either ``def`` or ``async def``; Genja will resolve either form.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any, Awaitable, ClassVar

from .plugin import PluginBase

class ConnectionKey:
    """Connection lookup key for a host and connection plugin."""

    hostname: str
    """Inventory hostname the connection targets."""

    plugin_name: str
    """Connection plugin name selected for the task."""

    def to_dict(self) -> dict[str, Any]:
        """Return the connection key as a dictionary."""
        ...

class ResolvedConnectionParams:
    """Resolved connection parameters passed to connection instances."""

    hostname: str
    """Inventory hostname the connection targets."""

    port: int | None
    """Network port, if configured."""

    username: str | None
    """Username, if configured."""

    password: str | None
    """Password or secret, if configured."""

    platform: str | None
    """Platform identifier, if configured."""

    extras: Any | None
    """Additional connection options from inventory."""

    def to_dict(self) -> dict[str, Any]:
        """Return resolved connection parameters as a dictionary."""
        ...

class ConnectionBase(ABC):
    """Base class for Python-authored connection instances."""

    @abstractmethod
    def open(
        self,
        params: ResolvedConnectionParams,
    ) -> None | Awaitable[None]:
        """Open the connection using resolved inventory parameters."""
        ...

    def execute_command(self, command: str) -> str | Awaitable[str]:
        """Execute a command on the connection.

        Override this for command-oriented connection plugins.
        """
        ...

    @abstractmethod
    def close(
        self,
    ) -> (
        ConnectionKey
        | dict[str, Any]
        | None
        | Awaitable[ConnectionKey | dict[str, Any] | None]
    ):
        """Close the connection and optionally return a reusable connection key."""
        ...

    @abstractmethod
    def is_alive(self) -> bool | Awaitable[bool]:
        """Return whether the connection is currently usable."""
        ...

class ConnectionPluginBase(PluginBase):
    """Base class for Python-authored connection plugins."""

    group_name: ClassVar[str]
    """Plugin group name used by Genja's plugin registry."""

    _locked_group_name: ClassVar[str]
    """Internal locked plugin group name."""

    @abstractmethod
    def create(
        self,
        key: ConnectionKey,
    ) -> ConnectionBase | Awaitable[ConnectionBase]:
        """Create a connection instance for a connection key."""
        ...

__all__: list[str]
