# API Surface

Genja is split into several Rust crates and one Python package. Most users
should depend on the highest-level package for their language and only reach for
lower-level crates when building extensions.

## Recommended Dependencies

=== ":fontawesome-brands-rust: Rust"

    ```toml
    [dependencies]
    genja = "0.1.0"
    ```

    Use the `genja` crate for applications. It provides the runtime,
    built-in plugins, task macro re-exports, and the common core types needed
    by normal task code.

=== ":fontawesome-brands-python: Python"

    ```bash
    pip install genja-py
    ```

    The Python distribution is named `genja-py`, but the import package is
    `genja`:

    ```python
    import genja
    ```

## Rust Crates

| Crate | Audience | Use it for |
| --- | --- | --- |
| `genja` | Applications and most Rust users | Runtime construction, built-in plugins, host filtering, task execution, and re-exported task authoring APIs. |
| `genja-core` | Advanced integrations and core-model users | Inventory models, settings, task traits/results, state types, and error types without the full runtime composition layer. |
| `genja-core-derive` | Macro internals and direct macro users | Procedural macros such as `#[genja_task]`. Most users get this through `genja`. |
| `genja-plugin-manager` | Rust plugin authors and applications with dynamic plugins | Plugin traits, plugin registration, dynamic shared-library loading, and build support helpers. |
| `genja-core-python` | Python binding build artifact | PyO3 extension crate that builds the `genja` Python module. Do not depend on it from normal Rust applications. |

## Python Package

| Package | Import name | Use it for |
| --- | --- | --- |
| `genja-py` | `genja` | Python runtime construction, Python task authoring, Python plugins, settings access, inventory access, and result inspection. |

Use the public Python submodules for task and plugin authoring:

```python
from genja.task import TaskSuccessResult, task
from genja.inventory import Host, Inventory
from genja.runner import RunnerPluginBase
from genja.processor import ProcessorPluginBase
from genja.connection import ConnectionPluginBase
from genja.transform import TransformFunctionPluginBase
```

## What To Import In Rust

For application code, start with `genja`:

```rust
use genja::{Genja, genja_task};
use genja::genja_core::inventory::Host;
use genja::genja_core::task::{
    HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
};
```

The `genja::genja_core` re-export keeps application code on a single top-level
dependency while still exposing the core model types needed by tasks.

The `genja` crate currently re-exports:

| Re-export | Use it for |
| --- | --- |
| `genja::genja_core` | Core inventory, settings, task, result, state, and error types. |
| `genja::genja_plugin_manager` | Plugin manager, plugin traits, dynamic loading, and build support helpers. |
| `genja::genja_task` | The task authoring macro from `genja-core-derive`. |
| `genja::GenjaError` | The shared Genja runtime error type. |
| `genja::async_trait` | The async trait macro used by Rust plugin examples. |
| `genja::plugins` | Built-in plugin types such as the file inventory and runner plugins. |

That means most application task code can access core types through `genja`
without adding separate `genja-core` or `genja-core-derive` dependencies.

Application code can also access plugin-manager APIs through `genja`:

```rust
use genja::genja_plugin_manager::PluginManager;
use genja::genja_plugin_manager::plugin_types::Plugins;
```

Use a direct `genja-plugin-manager` dependency when building a standalone Rust
plugin crate that should not depend on the full `genja` runtime.

## Which Package Should I Use?

Use `genja` when:

- you are writing a Rust application
- you want to load settings and inventory
- you want built-in runners and inventory plugins
- you want to run tasks against selected hosts

Use `genja-core` when:

- you are building lower-level integrations
- you need the inventory, settings, task, result, or state models without the
  full runtime
- you are writing code that must avoid depending on dynamic plugin loading

Use `genja-plugin-manager` when:

- you are registering custom Rust plugins
- you are dynamically loading shared-library plugins
- you need plugin traits such as `PluginInventory`, `PluginRunner`,
  `PluginConnection`, `PluginProcessor`, or `PluginTransformFunction`

Use `genja-py` when:

- you are writing Python automation code
- you are authoring Python tasks
- you are registering Python plugins
- you want to run Genja from Python with the same settings and inventory files

## Avoid Depending On Internals

Prefer public package APIs over workspace-internal details. In particular:

- Rust applications should not depend on `genja-core-python`.
- Most Rust applications should not depend on `genja-core-derive` directly.
- Python users should import from `genja.task`, `genja.inventory`,
  `genja.runner`, `genja.processor`, `genja.connection`, and
  `genja.transform` instead of relying on implementation details.

## Related Guides

- [Installation](installation.md)
- [Quickstart](quickstart.md)
- [Tasks](tasks.md)
- [Plugins](plugins/index.md)
- [Examples](examples.md)
