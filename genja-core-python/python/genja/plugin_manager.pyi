"""Python plugin-manager API for Genja.

Import plugin-manager helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.plugin_manager`` is the primary public surface for:

- ``PluginManager``

``PluginManager`` collects plugins from Rust shared libraries, Python plugin
instances, and ``pyproject.toml`` entries. Passing it into a ``Genja`` runtime
constructor transfers its owned plugin registry into the runtime, so the same
manager instance should not be reused afterward.
"""

from __future__ import annotations

from typing import Any

class PluginManager:
    """Plugin registry for extending Genja's runtime capabilities.

    ``PluginManager`` collects plugins from various sources (Rust shared
    libraries, Python plugin instances, and ``pyproject.toml`` entries) and can
    transfer them into a ``Genja`` runtime instance. Once transferred, the same
    manager instance should not be reused.

    Supported plugin groups:

    - ``ConnectionPlugin`` - Custom connection handlers
    - ``ProcessorPlugin`` - Task lifecycle hooks
    - ``InventoryPlugin`` - Inventory data sources
    - ``RunnerPlugin`` - Task execution strategies
    - ``TransformFunctionPlugin`` - Data transformation utilities

    Example:
        >>> manager = PluginManager()
        >>> manager.load_python_plugins_from_pyproject()
        >>> runtime = Genja.from_hosts(hosts, plugin_manager=manager)
    """

    def __init__(self) -> None:
        """Create a new plugin manager with built-in plugins pre-registered."""
        ...

    def load_rust_plugins_from_directory(self, path: str) -> None:
        """Load Rust-authored plugins from shared libraries in a directory.

        Args:
            path: Directory containing plugin shared libraries.

        Raises:
            ValueError: If the directory cannot be read or a plugin fails to load.
        """
        ...

    def register_plugin(self, plugin: Any) -> None:
        """Register a Python plugin instance directly.

        Args:
            plugin: Plugin instance implementing one of the supported plugin groups.

        Example:
            >>> manager.register_plugin(MyCustomProcessor())
        """
        ...

    def load_python_plugins_from_pyproject(self, path: str | None = None) -> None:
        """Load Python plugins declared in ``pyproject.toml``.

        Plugin entries are read from ``[tool.genja.plugins.<group>]`` tables
        using ``module:attribute`` import paths. The manifest key must match the
        plugin's declared ``name`` property.

        Args:
            path: Path to ``pyproject.toml``. If ``None``, uses
                ``"pyproject.toml"`` in the current directory.

        Example:
            ``pyproject.toml``:
            [tool.genja.plugins.processor]
            audit = "my_package.plugins:AuditProcessor"
        """
        ...

    def deregister_plugin(self, name: str) -> str | None:
        """Remove a plugin by name.

        Args:
            name: Registered plugin name.

        Returns:
            The deregistered plugin name if found, otherwise ``None``.
        """
        ...

    def plugin_names(self) -> list[str]:
        """Return the names of all registered plugins."""
        ...

    def plugin_names_and_groups(self) -> list[tuple[str, str]]:
        """Return ``(plugin_name, group_name)`` pairs for all registered plugins."""
        ...

__all__: list[str]
