from __future__ import annotations

from abc import ABC, abstractmethod
from typing import ClassVar, final

class PluginBase(ABC):
    group_name: ClassVar[str]
    _locked_group_name: ClassVar[str | None]

    @property
    @abstractmethod
    def name(self) -> str: ...
    @property
    @final
    def group(self) -> str: ...

__all__: list[str]
