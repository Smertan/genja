import sys
import textwrap

import genja
from tests.fixtures.connection_plugins import ConnectionPlugin


def test_plugin_manager_register_plugin_accepts_python_connection_plugin():
    plugins = genja.PluginManager()
    plugins.register_plugin(ConnectionPlugin())

    assert "ssh" in plugins.plugin_names()
    assert ("ssh", "Connection") in plugins.plugin_names_and_groups()


def test_plugin_manager_loads_python_connections_from_pyproject(tmp_path):
    module_path = tmp_path / "connection_plugins.py"
    module_path.write_text(
        textwrap.dedent(
            """
            from tests.fixtures.connection_plugins import ConnectionPlugin as BaseConnectionPlugin

            class ConnectionPlugin(BaseConnectionPlugin):
                pass
            """
        )
    )
    pyproject_path = tmp_path / "pyproject.toml"
    pyproject_path.write_text(
        textwrap.dedent(
            """
            [tool.genja.plugins.connection]
            ssh = "connection_plugins:ConnectionPlugin"
            """
        )
    )

    sys.path.insert(0, str(tmp_path))
    try:
        plugins = genja.PluginManager()
        plugins.load_python_plugins_from_pyproject(str(pyproject_path))
    finally:
        sys.path.remove(str(tmp_path))
        sys.modules.pop("connection_plugins", None)

    assert "ssh" in plugins.plugin_names()
    assert ("ssh", "Connection") in plugins.plugin_names_and_groups()
