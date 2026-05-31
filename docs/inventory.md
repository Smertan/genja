# Inventory

Inventory defines the hosts Genja can operate on. A runtime can load inventory
from files through the built-in file inventory plugin, or receive inventory
directly from Rust or Python code.

## File Inventory

The built-in inventory plugin is named `FileInventoryPlugin`. This is the
default inventory loader and it supports both JSON and YAML files. Configure it
in settings and point it at an inventory file:

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml
```

Supported file extensions are `.json`, `.yaml`, and `.yml`.

Use a JSON file by changing the path extension:

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.json
```

## Hosts

Hosts files are maps keyed by host ID. The map key becomes the Genja host name.

```yaml
router1:
  hostname: 10.0.0.1
  platform: ios
  groups:
    - core
  data:
    site:
      name: core
      role: edge

router2:
  hostname: 10.0.0.2
  platform: nxos
  groups:
    - edge
  data:
    site:
      name: branch
      role: access
```

The same inventory can be written as JSON:

```json
{
  "router1": {
    "hostname": "10.0.0.1",
    "platform": "ios",
    "groups": ["core"],
    "data": {
      "site": {
        "name": "core",
        "role": "edge"
      }
    }
  },
  "router2": {
    "hostname": "10.0.0.2",
    "platform": "nxos",
    "groups": ["edge"],
    "data": {
      "site": {
        "name": "branch",
        "role": "access"
      }
    }
  }
}
```

## Host Fields

Hosts support these fields:

- `hostname` (string | null)
- `port` (number | null)
- `username` (string | null)
- `password` (string | null)
- `platform` (string | null)
- `groups` (list of strings | null)
- `data` (object | null)
- `connection_options` (map of string to object | null)

Unknown host fields are rejected so misspelled inventory keys fail early.

## Groups

Groups provide shared values for hosts. A host joins groups with the `groups`
field:

```yaml
core:
  username: admin
  platform: ios
  data:
    site_type: core

edge:
  username: admin
  data:
    site_type: edge
```

Configure the groups file in settings:

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml
    groups_file: ./groups.yaml
```

Groups support the same fields as hosts.

## Defaults

Defaults provide base values for the inventory:

```yaml
username: admin
port: 22
platform: linux
data:
  retries: 3
```

Configure the defaults file in settings:

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml
    groups_file: ./groups.yaml
    defaults_file: ./defaults.yaml
```

Defaults support the same fields as groups, except `groups` and `defaults`.

## Custom Inventory Plugins

Python inventory plugins extend `InventoryPluginBase` and implement
`load(...)`. The loader may return either:

- a host mapping in the same shape accepted by `Genja.from_hosts(...)`
- a full `Inventory` object with `hosts`, `groups`, and `defaults`

### Synchronous Loaders

=== ":fontawesome-brands-rust: Rust"

    Rust inventory plugins implement `PluginInventory`. Unlike Python, the Rust
    `load(...)` hook is synchronous and returns a full `Inventory`.

    ```rust
    use genja::genja_core::inventory::{Host, Hosts, Inventory};
    use genja::genja_core::{InventoryLoadError, Settings};
    use genja_plugin_manager::plugin_types::{Plugin, PluginInventory, Plugins};
    use genja_plugin_manager::PluginManager;

    #[derive(Debug)]
    struct StaticInventoryPlugin;

    impl Plugin for StaticInventoryPlugin {
        fn name(&self) -> String {
            "static_inventory".to_string()
        }
    }

    impl PluginInventory for StaticInventoryPlugin {
        fn load(
            &self,
            _settings: &Settings,
            _plugins: &PluginManager,
        ) -> Result<Inventory, InventoryLoadError> {
            let mut hosts = Hosts::new();
            hosts.add_host(
                "router1",
                Host::builder().hostname("10.0.0.1").platform("ios").build(),
            );
            hosts.add_host(
                "router2",
                Host::builder().hostname("10.0.0.2").platform("nxos").build(),
            );

            Ok(Inventory::builder().hosts(hosts).build())
        }
    }

    let mut plugins = PluginManager::new();
    plugins.register_plugin(Plugins::Inventory(Box::new(StaticInventoryPlugin)));
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib
    from genja import Settings
    from genja.inventory import Host, Inventory, InventoryPluginBase


    class StaticInventoryPlugin(InventoryPluginBase):
        name = "static_inventory"

        def load(
            self,
            settings: Settings,
            plugins: object,
        ) -> Inventory:
            return Inventory(
                hosts={
                    "router1": Host(hostname="10.0.0.1", platform="ios"),
                    "router2": Host(hostname="10.0.0.2", platform="nxos"),
                }
            )


    plugins = genja_lib.PluginManager()
    plugins.register_plugin(StaticInventoryPlugin())
    ```

### Async Loaders

Python and Rust both support async inventory loading, but the integration path
differs today. Python can pass an in-memory plugin manager into
`Genja.from_settings_file(...)`. Rust currently needs to register the plugin,
load the inventory explicitly, and then build `Genja` from that inventory.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja::async_trait;
    use genja::genja_core::inventory::{Host, Hosts, Inventory};
    use genja::genja_core::{InventoryLoadError, Settings};
    use genja_plugin_manager::plugin_types::{AsyncPluginInventory, Plugin, Plugins};
    use genja_plugin_manager::PluginManager;

    #[derive(Debug)]
    struct ApiInventoryPlugin;

    impl Plugin for ApiInventoryPlugin {
        fn name(&self) -> String {
            "api_inventory".to_string()
        }
    }

    #[async_trait]
    impl AsyncPluginInventory for ApiInventoryPlugin {
        async fn load_async(
            &self,
            _settings: &Settings,
            _plugins: &PluginManager,
        ) -> Result<Inventory, InventoryLoadError> {
            let mut hosts = Hosts::new();
            hosts.add_host(
                "router1",
                Host::builder().hostname("10.0.0.1").platform("ios").build(),
            );
            Ok(Inventory::builder().hosts(hosts).build())
        }
    }

    #[tokio::main]
    async fn main() -> Result<(), Box<dyn std::error::Error>> {
        let settings = Settings::from_file("settings.yaml")?;

        let mut plugins = PluginManager::new();
        plugins.register_plugin(Plugins::AsyncInventory(Box::new(ApiInventoryPlugin)));

        let inventory = plugins
            .get_async_inventory_plugin("api_inventory")
            .ok_or("missing async inventory plugin")?
            .load_async(&settings, &plugins)
            .await?;

        let genja = Genja::builder(inventory)
            .with_settings(settings)
            .with_plugin_manager(plugins)
            .build()?;

        for host_id in genja.host_ids() {
            println!("{host_id}");
        }

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"
    Python async inventory loaders use the same base class and still use
    `Genja.from_settings_file(...)`. Pass the plugin manager when loading
    settings so the runtime can resolve the Python plugin by name.

    ```python
    import genja as genja_lib
    from genja import Settings
    from genja.inventory import InventoryPluginBase


    class ApiInventoryPlugin(InventoryPluginBase):
        name = "api_inventory"

        async def load(
            self,
            settings: Settings,
            plugins: object,
        ) -> dict[str, dict[str, object]]:
            return {
                "router1": {
                    "hostname": "10.0.0.1",
                    "platform": "ios",
                },
                "router2": {
                    "hostname": "10.0.0.2",
                    "platform": "nxos",
                },
            }


    plugins = genja_lib.PluginManager()
    plugins.register_plugin(ApiInventoryPlugin())

    genja = genja_lib.Genja.from_settings_file(
        "settings.yaml",
        plugin_manager=plugins,
    )

    for host_id in genja.host_ids():
        print(host_id)
    ```

## Inventory Transforms

Inventory transforms can normalize or enrich inventory values when they are
accessed. They are documented in detail in [Transforms](transforms.md).

A transform may implement one, multiple, or all of these hooks:

- `transform_host`
- `transform_group`
- `transform_defaults`

Missing hooks pass the original value through unchanged. Genja passes the same
optional `transform_function_options` value to every implemented hook, so use
nested keys when different hooks need different settings.

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml
  transform_function: normalize_inventory
  transform_function_options:
    hostname_suffix: ".lab"
    defaults:
      platform: linux
```

## Load Inventory

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;

    fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;

        for host_id in genja.host_ids() {
            println!("{host_id}");
        }

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    genja = genja_lib.Genja.from_settings_file("settings.yaml")

    for host_id in genja.host_ids():
        print(host_id)
    ```

## Inline Inventory

Use inline inventory for small scripts, tests, or generated inventories.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;
    use genja::genja_core::inventory::{BaseBuilderHost, Host, Hosts, Inventory};

    let mut hosts = Hosts::new();
    hosts.add_host(
        "router1",
        Host::builder()
            .hostname("10.0.0.1")
            .platform("ios")
            .build(),
    );

    let inventory = Inventory::builder().hosts(hosts).build();
    let genja = Genja::from_inventory(inventory);
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    genja = genja_lib.Genja.from_hosts({
        "router1": {
            "hostname": "10.0.0.1",
            "platform": "ios",
        }
    })
    ```

## Filtering Hosts

Filtering creates a new runtime with the same inventory, settings, and plugins,
but with a narrower selected host list.

Use `filter_by_key` to select hosts where a key exists:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let hosts_with_site = genja.filter_by_key("data.site.name")?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    hosts_with_site = genja.filter_by_key("data.site.name")
    ```

Use `filter_by_key_value` to match values with a regular expression:

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let core_site = genja.filter_by_key_value("data.site.name", "^core$")?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    core_site = genja.filter_by_key_value("data.site.name", "^core$")
    ```

Plain keys can match nested objects recursively. Dot paths such as
`data.site.name` match from the host root or a nested object.

## Shared Example Data

The repository includes shared example inventory files:

- `genja/examples/inventory/hosts.yaml`
- `genja/examples/inventory/hosts.json`

Rust examples under `genja/examples/*.rs` and Python examples under
`genja/examples/python` use this inventory data.
