class StaticInventoryPlugin:
    name = "python_inventory"

    group = "InventoryPlugin"

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


class AsyncInventoryPlugin:
    name = "python_async_inventory"

    group = "InventoryPlugin"

    async def load(self, settings, plugins):
        assert settings.inventory.plugin == "python_async_inventory"
        assert "python_async_inventory" in plugins.plugin_names()
        return {
            "router1": {
                "hostname": "10.20.20.1",
                "platform": "ios",
            },
            "router2": {
                "hostname": "10.20.20.2",
                "platform": "nxos",
            },
        }
