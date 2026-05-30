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

## Inventory Transforms

Inventory transforms can normalize or enrich inventory values when they are
accessed. A transform may implement one, multiple, or all of these hooks:

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

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::inventory::{
        BaseBuilderHost, Defaults, Host, Transform, TransformFunctionOptions,
    };

    struct NormalizeInventory;

    impl Transform for NormalizeInventory {
        fn transform_host(
            &self,
            host: &Host,
            options: Option<&TransformFunctionOptions>,
        ) -> Host {
            let suffix = options
                .and_then(|options| options.get("hostname_suffix"))
                .and_then(|value| value.as_str())
                .unwrap_or("");

            match host.hostname() {
                Some(hostname) => host
                    .to_builder()
                    .hostname(format!("{hostname}{suffix}"))
                    .build(),
                None => host.clone(),
            }
        }

        fn transform_defaults(
            &self,
            defaults: &Defaults,
            _options: Option<&TransformFunctionOptions>,
        ) -> Defaults {
            defaults.to_builder().platform("linux").build()
        }
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib
    from genja.transform import (
        TransformDefaultsHookProtocol,
        TransformFunctionPluginProtocol,
        TransformHostHookProtocol,
    )


    class NormalizeInventory:
        def name(self) -> str:
            return "normalize_inventory"

        def group(self) -> str:
            return "TransformFunctionPlugin"

        def transform_host(
            self,
            host: dict[str, object],
            options: dict[str, object] | None,
        ) -> dict[str, object]:
            suffix = (options or {}).get("hostname_suffix", "")
            hostname = host.get("hostname")
            if hostname is None:
                return host

            return {
                **host,
                "hostname": f"{hostname}{suffix}",
            }

        def transform_defaults(
            self,
            defaults: dict[str, object],
            options: dict[str, object] | None,
        ) -> dict[str, object]:
            default_options = (options or {}).get("defaults", {})
            return {**defaults, **default_options}


    plugins = genja_lib.PluginManager()
    transform_plugin = NormalizeInventory()

    # Optional annotations for editor and type-checker support.
    plugin_contract: TransformFunctionPluginProtocol = transform_plugin
    host_hook: TransformHostHookProtocol = transform_plugin
    defaults_hook: TransformDefaultsHookProtocol = transform_plugin

    plugins.register_plugin(plugin_contract)
    ```

Protocol annotations are optional. They help editors and type checkers validate
plugin shape; Genja registers plugins structurally at runtime using `name()`,
`group()`, and any transform hooks present.

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
