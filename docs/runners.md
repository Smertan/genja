# Runners

Runners control how Genja executes tasks across the selected hosts. A runner is
selected by name in settings or with `with_runner(...)`.

The runner does not decide which hosts are selected. Inventory loading and
filtering decide the selected host set, then the runner decides how task work is
scheduled across those hosts.

## Built-In Runners

Genja includes two built-in runners:

- `threaded`: executes host work concurrently with bounded async workers.
- `serial`: executes host work one host at a time.

The default runner is `threaded`.

## Configure A Runner

Configure the runner in `settings.yaml`:

```yaml
runner:
  plugin: threaded
  worker_count: 10
  max_task_depth: 10
  max_connection_attempts: 3
  allow_retries: false
  max_task_attempts: 1
```

Runner settings:

- `plugin`: runner plugin name. Defaults to `threaded`.
- `options`: plugin-specific JSON object. Defaults to `{}`.
- `worker_count`: optional concurrency limit for runners that support it.
- `max_task_depth`: maximum nested task depth. Defaults to `10`.
- `max_connection_attempts`: maximum connection attempts. Defaults to `3`.
- `allow_retries`: whether task retries are allowed by default. Defaults to `false`.
- `max_task_attempts`: total number of task attempts allowed by default. Defaults to `1`.

`max_connection_attempts` is part of the shared runner configuration. The
built-in runners pass the task connection resolver into task execution; the
connection layer is responsible for interpreting connection retry behavior.
Custom runners should pass the resolver through when they delegate to task
execution helpers.

`allow_retries` and `max_task_attempts` define default retry policy for task
execution. Tasks may override these defaults through task metadata.
Retries are only attempted when the effective policy allows them and the task
returns a failed host result marked as retryable.
Genja does not infer task mutability or idempotency when applying retries; the
decision is driven entirely by configured policy and the task's returned
failure result.

## Threaded Runner

The `threaded` runner executes a task across multiple hosts concurrently. It is
usually the best default for network or I/O-bound automation.

The effective worker count is chosen in this order:

1. Use `runner.worker_count` when set.
2. Otherwise use system available parallelism.
3. Never run more workers than selected hosts.
4. Clamp configured worker counts to at least `1`.

Use `threaded` when hosts can be processed independently:

```yaml
runner:
  plugin: threaded
  worker_count: 20
```

The threaded runner keeps up to `worker_count` host executions in flight. As
soon as one host finishes, the next pending host is scheduled until the selected
host list is exhausted.

Host results are merged as workers finish, so the order of host keys in a
threaded result should not be used as an execution-order guarantee. Use the
status lists and host IDs in the result data when making automation decisions.

## Serial Runner

The `serial` runner executes work sequentially. It is useful for debugging,
small inventories, strict ordering, or tasks that should not run concurrently.

```yaml
runner:
  plugin: serial
```

`worker_count` does not affect the built-in `serial` runner.

Serial execution follows the selected host iteration order. That makes logs and
result inspection easier while you are developing a task or diagnosing an
inventory issue.

## Select A Runner In Code

`with_runner(...)` returns a new runtime with the selected runner. It preserves
the existing runner options and limits while changing only the runner plugin
name.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;

    fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;
        let genja = genja.with_runner("serial")?;

        Ok(())
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib

    genja = genja_lib.Genja.from_settings_file("settings.yaml")
    genja = genja.with_runner("serial")
    ```

The named runner must already be registered. Unknown plugins fail, and plugins
registered under a different plugin type are rejected.

## Task Depth

`max_task_depth` limits nested sub-task execution.

```yaml
runner:
  plugin: threaded
  max_task_depth: 1
```

When running a task directly, the call may also provide a maximum depth.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    let results = genja.run_task(task, 1)?;
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    results = genja.run_task(CollectFacts, max_depth=1)
    ```

Use `0` for only the top-level task. Use a higher value when sub-tasks should
run.

Rust `run_task(...)` and `run_tasks(...)` require the depth argument. Python
accepts `max_depth=None`; when omitted, the runtime uses
`runner.max_task_depth` from settings.

## Multiple Tasks

When running an ordered task list, root tasks are executed in list order. The
runner controls host execution for each root task.

- `serial`: each root task runs host-by-host before the next root task starts.
- `threaded`: each root task runs selected hosts concurrently before the next
  root task starts.

Sub-tasks belong to their parent task tree and are controlled by the same task
depth limit.

This means the built-in threaded runner parallelizes hosts within a root task;
it does not run separate root tasks from the same `Tasks` list at the same time.

## Empty Host Selections

If filtering selects no hosts, runners still return a task result object for the
requested task. The host result map is empty, and lifecycle processors can still
observe task start and finish hooks.

This is useful for workflows that treat "nothing matched" as a reportable
outcome instead of a runtime crash. If an empty host set is unexpected, check
`host_ids()` before running the task.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    if genja.host_ids().is_empty() {
        eprintln!("no hosts selected");
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    if not genja.host_ids():
        print("no hosts selected")
    ```

## Custom Runner Plugins

Custom runner plugins implement the runner interface for the language they are
authored in and register under the `RunnerPlugin` group.

=== ":fontawesome-brands-rust: Rust"

    Rust runner plugins are async-only because `PluginRunner::run_task(...)` is
    an async trait method. When the plugin crate already depends on `genja`,
    import the macro from `genja::async_trait` instead of adding a separate
    `async-trait` dependency just for the example.

    ```rust
    use std::sync::Arc;
    use genja::async_trait;
    use genja::genja_core::inventory::Hosts;
    use genja::genja_core::settings::RunnerConfig;
    use genja::genja_core::task::{
        TaskConnectionResolver, TaskDefinition, TaskResults,
    };
    use genja_plugin_manager::plugin_types::{Plugin, PluginRunner, Plugins};
    use genja_plugin_manager::PluginManager;

    #[derive(Debug)]
    struct FirstHostOnlyRunner;

    impl Plugin for FirstHostOnlyRunner {
        fn name(&self) -> String {
            "first_host_only".to_string()
        }
    }

    #[async_trait]
    impl PluginRunner for FirstHostOnlyRunner {
        async fn run_task(
            &self,
            task: &TaskDefinition,
            hosts: &Hosts,
            connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
            _runner_config: &RunnerConfig,
            max_depth: usize,
        ) -> Result<TaskResults, genja::GenjaError> {
            let mut results = TaskResults::new(task.as_task().name());

            if let Some((host_id, host)) = hosts.iter().next() {
                task.start_with_connection_resolver(
                    host_id.as_ref(),
                    host,
                    &mut results,
                    connection_resolver.as_deref(),
                    max_depth,
                )
                .await?;
            }

            Ok(results)
        }
    }

    let mut plugins = PluginManager::new();
    plugins.register_plugin(Plugins::Runner(Box::new(FirstHostOnlyRunner)));
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib
    from genja.runner import RunnerPluginBase
    from genja.settings import RunnerConfig


    class FirstHostOnlyRunner(RunnerPluginBase):
        name = "first_host_only"

        def run_task(
            self,
            task: genja_lib.TaskDefinition,
            hosts: dict[str, object],
            connection_resolver: genja_lib.TaskConnectionResolver | None,
            runner_config: RunnerConfig,
            max_depth: int,
        ) -> genja_lib.TaskResults:
            _, first_host = next(iter(hosts.items()))
            return task.run_on_host(
                first_host,
                connection_resolver=connection_resolver,
                max_depth=max_depth,
            )


    plugins = genja_lib.PluginManager()
    plugins.register_plugin(FirstHostOnlyRunner())
    ```

Custom runners may also implement `run_tasks(...)` for custom ordered task-list
execution. If `run_tasks(...)` is not provided, Genja delegates each root task to
`run_task(...)` in order.

Python runners that implement `run_tasks(...)` should inherit from
`BatchRunnerPluginBase` when using the typed public API.

### Python Async Variant

Async Python runners use the same base class:

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib
    from genja.runner import RunnerPluginBase
    from genja.settings import RunnerConfig


    class AsyncFirstHostOnlyRunner(RunnerPluginBase):
        name = "async_first_host_only"

        async def run_task(
            self,
            task: genja_lib.TaskDefinition,
            hosts: dict[str, object],
            connection_resolver: genja_lib.TaskConnectionResolver | None,
            runner_config: RunnerConfig,
            max_depth: int,
        ) -> genja_lib.TaskResults:
            _, first_host = next(iter(hosts.items()))
            return task.run_on_host(
                first_host,
                connection_resolver=connection_resolver,
                max_depth=max_depth,
            )
    ```

## Choosing A Runner

Use `threaded` when:

- hosts can be processed independently
- tasks spend time waiting on network or remote systems
- inventory is large enough that concurrency matters

Use `serial` when:

- debugging task logic
- output ordering matters
- remote systems should not be touched concurrently
- a workflow has external sequencing constraints
