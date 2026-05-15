class StaticInventoryPlugin:
    def name(self) -> str:
        return "python_inventory"

    def group(self) -> str:
        return "InventoryPlugin"

    def load(self, settings, plugins):
        assert settings.inventory.plugin == "python_inventory"
        assert "python_inventory" in plugins.plugin_names()
        return {
            "router1": {
                "hostname": "10.10.10.1",
                "platform": "ios",
            },
            "router2": {
                "hostname": "10.10.10.2",
                "platform": "nxos",
            },
        }
