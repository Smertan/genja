# Runner Plugins

Runner plugins control how Genja schedules tasks across selected hosts. This
guide is for authors creating a runner. For configuration and behavior of
the built-in `serial` and `threaded` runners, see [Runners](../runners.md).

## Create a Runner Plugin

Create a custom runner by defining a runner type or class for your language
and registering it under the `RunnerPlugin` group.
Pass the connection resolver through when delegating to task execution helpers
so tasks can use their declared connections.

=== ":fontawesome-brands-rust: Rust"

    Rust runner plugins provide the async `PluginRunner::run_task(...)`
    method. When the plugin crate already depends on `genja`, import
    `genja::async_trait` instead of adding a separate `async-trait` dependency.
    This example delegates task execution to the core runtime and declares
    support for custom Rust retry policies.

    ```rust
    use std::sync::Arc;
    use genja::async_trait;
    use genja::genja_core::inventory::Hosts;
    use genja::genja_core::settings::RunnerConfig;
    use genja::genja_core::task::{
        TaskConnectionResolver, TaskDefinition, TaskResults, TaskRunOptions,
    };
    use genja::genja_plugin_manager::plugin_types::{Plugin, PluginRunner, Plugins};
    use genja::genja_plugin_manager::PluginManager;

    #[derive(Debug)]
    struct FirstHostOnlyRunner;

    impl Plugin for FirstHostOnlyRunner {
        fn name(&self) -> String {
            "first_host_only".to_string()
        }
    }

    #[async_trait]
    impl PluginRunner for FirstHostOnlyRunner {
        fn supports_custom_task_retry_policy(&self) -> bool {
            true
        }

        async fn run_task(
            &self,
            task: &TaskDefinition,
            hosts: &Hosts,
            connection_resolver: Option<Arc<dyn TaskConnectionResolver>>,
            runner_config: &RunnerConfig,
            run_options: TaskRunOptions,
        ) -> Result<TaskResults, genja::GenjaError> {
            let mut results = TaskResults::new(task.as_task().name());

            if let Some((host_id, host)) = hosts.iter().next() {
                task.start_with_connection_resolver_runner_config_and_options(
                    host_id.as_ref(),
                    host,
                    &mut results,
                    connection_resolver.as_deref(),
                    runner_config,
                    run_options,
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
            run_options: genja_lib.TaskRunOptions,
        ) -> genja_lib.TaskResults:
            _, first_host = next(iter(hosts.items()))
            return task.run_on_host(
                first_host,
                connection_resolver=connection_resolver,
                run_options=run_options,
            )


    plugins = genja_lib.PluginManager()
    plugins.register_plugin(FirstHostOnlyRunner())
    ```

## Custom Retry Policy Support

=== ":fontawesome-brands-rust: Rust"

    Add this method inside your runner's `impl PluginRunner` block when it
    delegates task execution to the core `TaskDefinition` start path:

    ```rust
    fn supports_custom_task_retry_policy(&self) -> bool {
        true
    }
    ```

    The method defaults to `false`. Return `true` only when `run_task(...)`
    and any overridden `run_tasks(...)` path carry the selected policy through
    the core runtime, or apply it with the same retry safety checks. Genja
    returns an error before calling a runner that reports `false` for a custom
    policy invocation. This method does not enable retries for tasks.

=== ":fontawesome-brands-python: Python"

    Python-authored custom retry policies are not currently supported, so
    Python runner authors do not need a capability override.

## Task List Execution

Custom runners may provide `run_tasks(...)` for custom ordered task-list
execution. If `run_tasks(...)` is omitted, Genja delegates each root task to
`run_task(...)` in order.

Python runners that provide `run_tasks(...)` should inherit from
`BatchRunnerPluginBase` when using the typed public API.

## Python Async Variant

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
            run_options: genja_lib.TaskRunOptions,
        ) -> genja_lib.TaskResults:
            _, first_host = next(iter(hosts.items()))
            return task.run_on_host(
                first_host,
                connection_resolver=connection_resolver,
                run_options=run_options,
            )
    ```
