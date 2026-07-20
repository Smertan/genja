"""Python plugin-manager API for Genja.

Import plugin-manager helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.plugin_manager`` is the primary public surface for:

- ``PluginManager``

Python-authored plugins currently support these groups:

- ``ConnectionPlugin``
- ``ProcessorPlugin``
- ``InventoryPlugin``
- ``RunnerPlugin``
- ``TransformFunctionPlugin``

``PluginManager`` is a setup-time object. Passing it into ``Genja.builder(...)``,
``Genja.from_hosts(...)``, ``Genja.from_settings(...)``,
``Genja.from_settings_async(...)``, or ``Genja.from_settings_file(...)``
transfers its owned plugin registry into the runtime, so the same manager
instance should not be reused afterward.
"""

from .genja import PluginManager


__all__ = ["PluginManager"]
