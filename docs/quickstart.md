# Quickstart

This guide creates a small inventory, loads Genja settings, runs a task across
two hosts, and prints the results.

You should finish with a working runtime and a JSON result that shows the task
passed for each host.

## Install

=== ":fontawesome-brands-rust: Rust"

    ```bash
    cargo add genja
    ```

    The task examples below also use Tokio and serde_json:

    ```bash
    cargo add tokio --features macros,rt-multi-thread
    cargo add serde_json
    ```

=== ":fontawesome-brands-python: Python"

    ```bash
    pip install genja-py
    ```

    The Python distribution is named `genja-py`, but the package is imported as
    `genja`.

## Create Settings

Create `settings.yaml`:

```yaml
inventory:
  plugin: FileInventoryPlugin
  options:
    hosts_file: ./hosts.yaml

runner:
  plugin: serial

logging:
  level: info
  to_console: true
```

Create `hosts.yaml`:

```yaml
router1:
  hostname: 10.0.0.1
  platform: ios
  groups:
    - core
  data:
    site:
      name: core

router2:
  hostname: 10.0.0.2
  platform: nxos
  groups:
    - edge
  data:
    site:
      name: edge
```

## Load The Runtime

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

Expected output:

```text
router1
router2
```

## Run A Task

Rust callers can use either:

- `run_task(...)` from synchronous code
- `run_task_async(...)` from inside `#[tokio::main]` or another active Tokio runtime

The sync wrapper returns an error if it is called from an active Tokio runtime.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::inventory::Host;
    use genja::genja_core::task::{
        HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
    };
    use genja::{Genja, genja_task};
    use serde_json::json;

    struct CollectFacts;

    #[genja_task(name = "collect_facts")]
    impl CollectFacts {
        async fn start_async(
            &self,
            host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            Ok(HostTaskResult::passed(TaskSuccess::new().with_result(
                json!({
                    "hostname": host.hostname(),
                    "platform": host.platform(),
                    "facts_collected": true,
                }),
            )))
        }
    }

    fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;
        let results = genja.run_task(CollectFacts, 1)?;

        let output = results.to_pretty_json_string().map_err(|err| {
            genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
        })?;
        println!("{output}");

        Ok(())
    }
    ```

    ### Async Variant

    ```rust
    use genja::Genja;

    #[tokio::main]
    async fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;
        let results = genja
            .run_task_async(CollectFacts, 1)
            .await?;

        let output = results.to_pretty_json_string().map_err(|err| {
            genja::GenjaError::Message(format!("failed to serialize task results: {err}"))
        })?;
        println!("{output}");

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(name="collect_facts")
    class CollectFacts:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            connection = context.connection()
            show_version = None
            if connection is not None:
                show_version = connection.execute_command("show version")

            return TaskSuccessResult(
                summary=f"collected facts from {host.hostname}",
                metadata={
                    "hostname": host.hostname,
                    "platform": host.platform,
                    "facts_collected": True,
                    "show_version": show_version,
                },
            )


    genja = genja_lib.Genja.from_settings_file("settings.yaml")
    results = genja.run_task(CollectFacts, max_depth=1)

    print(results.to_json(pretty=True))
    ```

    ### Async Variant

    Use `run_task_async(...)` when the surrounding Python application is already
    using `asyncio`.

    ```python
    import asyncio
    import genja as genja_lib
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(name="collect_facts_async")
    class CollectFactsAsync:
        async def start_async(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            connection = context.connection()
            show_version = None
            if connection is not None:
                show_version = await connection.execute_command("show version")

            return TaskSuccessResult(
                summary=f"collected facts from {host.hostname}",
                metadata={
                    "hostname": host.hostname,
                    "platform": host.platform,
                    "facts_collected": True,
                    "show_version": show_version,
                },
            )


    async def main() -> None:
        genja = genja_lib.Genja.from_settings_file("settings.yaml")
        results = await genja.run_task_async(CollectFactsAsync, max_depth=1)

        print(results.to_json(pretty=True))


    asyncio.run(main())
    ```

    `TaskRuntimeContext` keeps execution depth internal. Python tasks access the
    resolved connection through `context.connection()` and can guard it with
    `context.has_connection()`.

The printed JSON includes the task name, per-host statuses, summaries or
metadata, and any nested sub-task results. For this task, both hosts should be
reported as passed.

## Run Repository Examples

Installed packages do not include the repository example source files. To run
the examples, clone the repository:

```bash
git clone https://github.com/Smertan/genja.git
cd genja
```

=== ":fontawesome-brands-rust: Rust"

    ```bash
    cargo run -p genja --example basic_runtime
    cargo run -p genja --example run_task
    cargo run -p genja --example run_task_tree
    cargo run -p genja --example filter_hosts
    cargo run -p genja --example async_inventory_plugin
    ```

=== ":fontawesome-brands-python: Python"

    ```bash
    python genja/examples/python/basic_runtime.py
    python genja/examples/python/filter_hosts.py
    python genja/examples/python/run_task.py
    python genja/examples/python/run_task_tree.py
    ```

    See `genja/examples/python/README.md` for local setup notes.
