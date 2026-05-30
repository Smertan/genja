# Runners

Runners control how Genja executes tasks across the selected hosts. A runner is
selected by name in settings or with `with_runner(...)`.

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
```

Runner settings:

- `plugin`: runner plugin name. Defaults to `threaded`.
- `options`: plugin-specific JSON object. Defaults to `{}`.
- `worker_count`: optional concurrency limit for runners that support it.
- `max_task_depth`: maximum nested task depth. Defaults to `10`.
- `max_connection_attempts`: maximum connection attempts. Defaults to `3`.

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

## Serial Runner

The `serial` runner executes work sequentially. It is useful for debugging,
small inventories, strict ordering, or tasks that should not run concurrently.

```yaml
runner:
  plugin: serial
```

`worker_count` does not affect the built-in `serial` runner.

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

## Multiple Tasks

When running an ordered task list, root tasks are executed in list order. The
runner controls host execution for each root task.

- `serial`: each root task runs host-by-host before the next root task starts.
- `threaded`: each root task runs selected hosts concurrently before the next
  root task starts.

Sub-tasks belong to their parent task tree and are controlled by the same task
depth limit.

## Python Runner Plugins

Python can author custom runner plugins by extending `RunnerPluginBase` and
registering the plugin with `PluginManager`.

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
        first_host = dict(list(hosts.items())[:1])
        return task.run_on_hosts(first_host, connection_resolver, max_depth)


plugins = genja_lib.PluginManager()
plugins.register_plugin(FirstHostOnlyRunner())
```

Custom runners may also implement `run_tasks(...)` for custom ordered task-list
execution. If `run_tasks(...)` is not provided, Genja delegates each root task to
`run_task(...)` in order.

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
