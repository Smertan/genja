# Transforms

Transform plugins normalize or enrich inventory values when Genja accesses
hosts, groups, or defaults. They are useful when the source inventory is valid
but not yet in the exact shape tasks and plugins should consume.

## When To Use A Transform

Transforms are a good fit for:

- normalizing hostnames, usernames, or platform names
- filling derived fields into host `data`
- merging default values into a cleaner runtime shape
- mapping source-system field names into the naming Genja tasks expect
- enriching inventory values from a lightweight lookup before task execution

Use an inventory plugin instead when you need to:

- fetch inventory from a new source
- load hosts, groups, or defaults from an API, database, or service
- decide which inventory records exist in the first place

## Hook Model

A transform may implement one, multiple, or all of these hooks:

- `transform_host`
- `transform_group`
- `transform_defaults`

Missing hooks pass the original value through unchanged.

Genja passes the same optional `transform_function_options` object to every
implemented hook, so use nested keys when different hooks need different
settings.

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

## Rust Transform Plugins

Rust transforms implement `Transform` and return updated inventory values.

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

## Python Transform Plugins

Python transforms extend `TransformFunctionPluginBase`.

```python
import genja as genja_lib
from genja.transform import TransformFunctionPluginBase


class NormalizeInventory(TransformFunctionPluginBase):
    name = "normalize_inventory"

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
plugins.register_plugin(NormalizeInventory())
```

### Async Variant

Python transform hooks may be synchronous or asynchronous. Use `async def` when
the transform needs to read from an async API before returning the updated
inventory value.

```python
from genja.transform import TransformFunctionPluginBase


class AsyncNormalizeInventory(TransformFunctionPluginBase):
    name = "async_normalize_inventory"

    async def transform_host(
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
```

## Common Patterns

Typical transform patterns include:

- append or rewrite hostnames for lab vs production environments
- set fallback platform values in `defaults`
- copy nested source metadata into flatter task-friendly keys
- normalize group names or host tags before filtering

Keep transforms narrow. They work best when they reshape already-loaded
inventory, not when they become a second inventory source.
