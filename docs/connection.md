# Connections

Connection plugins create and manage per-host sessions for tasks that declare a
`connection_plugin_name`. They are selected by task metadata, and each host may
provide plugin-specific connection overrides through `connection_options`.

## Select A Connection Plugin

Tasks choose a connection plugin by name.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::TaskDerive;

    #[derive(TaskDerive)]
    #[task(connection_plugin_name = "ssh")]
    struct BackupConfig {
        name: &'static str,
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import task


    @task(name="backup_config", connection_plugin_name="ssh")
    class BackupConfig:
        ...
    ```

The named plugin must already be registered, and the host inventory must
resolve the connection parameters that plugin needs.

## Inventory Connection Options

Hosts, groups, and defaults may define `connection_options`. Genja merges these
values while resolving the connection requested by the task.

```yaml
router1:
  hostname: 10.0.0.1
  platform: ios
  username: admin
  connection_options:
    ssh:
      port: 22
      extras:
        device_type: cisco_ios
    netconf:
      port: 830

core:
  connection_options:
    ssh:
      username: automation

defaults:
  connection_options:
    ssh:
      extras:
        timeout: 30
```

The resolved connection payload contains:

- `hostname`
- `port`
- `username`
- `password`
- `platform`
- `extras`

## Rust Connection Plugins

Rust connection plugins implement `PluginConnection`. The trait is async because
`open(...)` and `execute_command(...)` may perform network I/O.

Required methods:

- `create(...)`
- `open(...)`
- `close(...)`
- `is_alive()`

Optional defaulted methods:

- `execute_command(...)`
- `group()`

```rust
use genja::async_trait;
use genja::genja_core::inventory::{ConnectionKey, ResolvedConnectionParams};
use genja_plugin_manager::plugin_types::{Plugin, PluginConnection, Plugins};
use genja_plugin_manager::PluginManager;

// Factory: Creates connection instances
#[derive(Debug)]
struct SshPlugin;

impl Plugin for SshPlugin {
    fn name(&self) -> String {
        "ssh".to_string()
    }
}

#[async_trait]
impl PluginConnection for SshPlugin {
    // Factory only needs create() - other methods can panic or return defaults
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        Box::new(SshConnection::new(key.clone()))
    }

    async fn open(&mut self, _params: &ResolvedConnectionParams) -> Result<(), String> {
        panic!("open() should not be called on factory")
    }

    fn close(&mut self) -> ConnectionKey {
        panic!("close() should not be called on factory")
    }

    fn is_alive(&self) -> bool {
        false // Factory itself is not a connection
    }
}

// Instance: Manages a specific host connection
#[derive(Debug)]
struct SshConnection {
    key: ConnectionKey,
    alive: bool,
}

impl SshConnection {
    fn new(key: ConnectionKey) -> Self {
        Self {
            key,
            alive: false,
        }
    }
}

impl Plugin for SshConnection {
    fn name(&self) -> String {
        "ssh".to_string()
    }
}

#[async_trait]
impl PluginConnection for SshConnection {
    fn create(&self, key: &ConnectionKey) -> Box<dyn PluginConnection> {
        // Instance shouldn't create new instances
        panic!("create() should not be called on connection instance")
    }

    async fn open(&mut self, params: &ResolvedConnectionParams) -> Result<(), String> {
        // Actual SSH connection logic would go here
        self.alive = true;
        Ok(())
    }

    async fn execute_command(&mut self, command: &str) -> Result<String, String> {
        if !self.alive {
            return Err("Connection not open".to_string());
        }
        Ok(format!("ran {command}"))
    }

    fn close(&mut self) -> ConnectionKey {
        self.alive = false;
        self.key.clone()
    }

    fn is_alive(&self) -> bool {
        self.alive
    }
}

// Register only the factory
let mut plugins = PluginManager::new();
plugins.register_plugin(Plugins::Connection(Box::new(SshPlugin)));
```

`create(...)` returns a per-host connection instance. The registered plugin acts
as the factory; the returned `SshConnection` instance handles `open`, command
execution, liveness checks, and `close`.

## Python Connection Plugins

Python connection plugins extend `ConnectionPluginBase` and return a
`ConnectionBase` instance from `create(...)`.

```python
import genja as genja_lib
from genja.connection import (
    ConnectionBase,
    ConnectionKey,
    ConnectionPluginBase,
    ResolvedConnectionParams,
)


class SshConnection(ConnectionBase):
    def __init__(self, key: ConnectionKey):
        self.key = key
        self.alive = False

    def open(self, params: ResolvedConnectionParams) -> None:
        self.alive = True

    def execute_command(self, command: str) -> str:
        return f"ran {command}"

    def close(self) -> ConnectionKey:
        self.alive = False
        return self.key

    def is_alive(self) -> bool:
        return self.alive


class SshPlugin(ConnectionPluginBase):
    name = "ssh"

    def create(self, key: ConnectionKey) -> SshConnection:
        return SshConnection(key)


plugins = genja_lib.PluginManager()
plugins.register_plugin(SshPlugin())
```

### Async Variant

Python connection factories and instance methods may be synchronous or
asynchronous. Async implementations use the same base classes:

```python
from genja.connection import (
    ConnectionBase,
    ConnectionKey,
    ConnectionPluginBase,
    ResolvedConnectionParams,
)


class AsyncSshConnection(ConnectionBase):
    def __init__(self, key: ConnectionKey):
        self.key = key
        self.alive = False

    async def open(self, params: ResolvedConnectionParams) -> None:
        self.alive = True

    async def execute_command(self, command: str) -> str:
        return f"ran {command}"

    async def close(self) -> ConnectionKey:
        self.alive = False
        return self.key

    async def is_alive(self) -> bool:
        return self.alive


class AsyncSshPlugin(ConnectionPluginBase):
    name = "ssh"

    async def create(self, key: ConnectionKey) -> AsyncSshConnection:
        return AsyncSshConnection(key)
```

## Runtime Context

When a task declares a connection plugin, the runtime passes a
`TaskRuntimeContext` value into `start(...)`. That context is created for the
current task execution on the current host.

The connection plugin result is accessed from the runtime context:

- Rust: `TaskRuntimeContext::connection()`
- Python: `TaskRuntimeContext.connection()`

The connection accessors are the relevant public task-facing API for
connection-aware Python tasks.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    async fn start(
        &self,
        _host: &genja::genja_core::inventory::Host,
        context: &genja::genja_core::task::TaskRuntimeContext,
    ) -> Result<genja::genja_core::task::HostTaskResult, genja::genja_core::task::TaskError> {
        let output = context.execute_command("show version").await?;
        Ok(genja::genja_core::task::HostTaskResult::passed(
            genja::genja_core::task::TaskSuccess::new().with_result(output),
        ))
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import TaskSuccessResult


    def start(self, task, host, context) -> TaskSuccessResult:
        connection = context.connection()
        output = None
        if connection is not None:
            output = connection.execute_command("show version")
        return TaskSuccessResult(
            summary=f"connected to {host.hostname}",
            metadata={"show_version": output},
        )
    ```
