"""Shared base classes for Python-authored Genja plugins."""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import ClassVar, final

class PluginBase(ABC):
    """Base class for Python plugin types with locked group names."""

    group_name: ClassVar[str]
    """Plugin group name used by Genja's plugin registry."""

    _locked_group_name: ClassVar[str | None]
    """Internal locked group name used by concrete plugin base classes."""

    @property
    @abstractmethod
    def name(self) -> str:
        """Return the plugin name used to select this plugin."""
        ...

    @property
    @final
    def group(self) -> str:
        """Return the plugin group used by Genja's plugin registry."""
        ...

__all__: list[str]
