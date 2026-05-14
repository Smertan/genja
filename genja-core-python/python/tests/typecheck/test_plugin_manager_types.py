from __future__ import annotations

from typing import Any

from genja.plugin_manager import PluginManager


class DummyPlugin:
    def name(self) -> str:
        return "dummy"

    def group(self) -> str:
        return "ProcessorPlugin"


def check_plugin_manager_types() -> None:
    manager = PluginManager()

    plugin: Any = DummyPlugin()
    manager.register_plugin(plugin)

    maybe_name: str | None = manager.deregister_plugin("dummy")
    names: list[str] = manager.plugin_names()
    names_and_groups: list[tuple[str, str]] = manager.plugin_names_and_groups()

    manager.load_python_plugins_from_pyproject()
    manager.load_python_plugins_from_pyproject("pyproject.toml")
    manager.load_rust_plugins_from_directory("plugins")

    _ = maybe_name
    _ = names
    _ = names_and_groups
