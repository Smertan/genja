"""Shared base classes for Python-authored Genja plugins."""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import ClassVar, final


class PluginBase(ABC):
    """Base class for Python plugin types with locked group names."""

    group_name: ClassVar[str]
    """Plugin group name used by Genja's plugin registry."""

    _locked_group_name: ClassVar[str | None] = None
    """Internal locked group name used by concrete plugin base classes."""

    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        base_locked = next(
            (
                base.__dict__.get("_locked_group_name")
                for base in cls.__mro__[1:]
                if base.__dict__.get("_locked_group_name") is not None
            ),
            None,
        )
        locked = base_locked or cls.__dict__.get("_locked_group_name")
        if locked is None:
            return

        if cls.__dict__.get("_locked_group_name", locked) != locked:
            raise TypeError(f"{cls.__name__} must use _locked_group_name = {locked!r}")

        if "group" in cls.__dict__:
            raise TypeError(f"{cls.__name__} must not override group")

        if cls.__dict__.get("group_name", locked) != locked:
            raise TypeError(f"{cls.__name__} must use group_name = {locked!r}")

    @property
    @abstractmethod
    def name(self) -> str:
        """Return the plugin name used to select this plugin."""
        ...

    @property
    @final
    def group(self) -> str:
        """Return the plugin group used by Genja's plugin registry."""
        return self.group_name


__all__ = ["PluginBase"]
