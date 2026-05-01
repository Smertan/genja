"""Python plugin-manager API for Genja.

Import plugin-manager helpers from this module instead of from ``genja_core``
directly. The top-level package re-exports these names for compatibility, but
``genja_core.plugin_manager`` is the primary public surface for:

- ``PluginManager``
"""

from .genja_core import PluginManager


__all__ = ["PluginManager"]
