# Tasks

Tasks are the unit of work Genja executes against selected hosts. A task
receives task metadata, the current host, and runtime context, then returns a
structured result for that host.

## Define A Task

=== ":fontawesome-brands-rust: Rust"

    Rust tasks derive Genja task metadata with `TaskDerive` and implement the
    `Task` trait to provide execution logic.

    ```rust
    use genja::genja_core::inventory::Host;
    use genja::genja_core::task::{
        HostTaskResult, Task, TaskError, TaskRuntimeContext, TaskSuccess,
    };
    use genja::{async_trait, TaskDerive};
    use serde_json::json;

    #[derive(TaskDerive)]
    struct CollectFacts {
        name: &'static str,
    }

    #[async_trait]
    impl Task for CollectFacts {
        async fn start(
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
    ```

=== ":fontawesome-brands-python: Python"

    Python tasks use the `@task(...)` decorator and implement a `start(...)`
    method. The method may be synchronous or asynchronous.

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(name="collect_facts")
    class CollectFacts:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(
                summary=f"collected facts from {host.hostname}",
                metadata={
                    "hostname": host.hostname,
                    "platform": host.platform,
                    "facts_collected": True,
                },
            )
    ```

## Task Inputs

Each task receives:

- task metadata: task name, options, processors, connection plugin, and sub-task
  metadata
- host data: hostname, port, username, password, platform, and inventory `data`
- runtime context: current task depth, maximum depth, and resolved connection
  when a connection plugin is used

=== ":fontawesome-brands-rust: Rust"

    ```rust
    async fn start(
        &self,
        host: &Host,
        context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        println!("host={:?} depth={}", host.hostname(), context.current_depth());
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult


    def start(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult:
        print(f"task={task.name} host={host.hostname} depth={context.current_depth}")
        return TaskSuccessResult()
    ```

## Result Types

Tasks return one result per host.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_core::task::{
        HostTaskResult, TaskFailure, TaskFailureKind, TaskSkip, TaskSuccess,
    };

    let passed = HostTaskResult::passed(TaskSuccess::new().with_summary("ok"));

    let failed = HostTaskResult::failed(
        TaskFailure::new(std::io::Error::other("connection failed"))
            .with_kind(TaskFailureKind::Connection),
    );

    let skipped = HostTaskResult::Skipped(
        TaskSkip::new()
            .with_reason("unsupported_platform")
            .with_message("host platform is not supported"),
    );
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import (
        Host,
        TaskInfo,
        TaskFailureKind,
        TaskFailureResult,
        TaskRuntimeContext,
        TaskSkipResult,
        TaskSuccessResult,
    )

    passed = TaskSuccessResult(summary="ok")

    failed = TaskFailureResult(
        message="connection failed",
        kind=TaskFailureKind.CONNECTION,
        retryable=True,
    )

    skipped = TaskSkipResult(
        reason="unsupported_platform",
        message="host platform is not supported",
    )
    ```

Success results can include result payloads, change status, diffs, summaries,
warnings, messages, and metadata. Failure results include a message, failure
kind, retryability, details, warnings, and messages. Skip results include a
machine-readable reason and human-readable message.

## Failure Kinds

Use failure kinds to make task failures easier to classify:

- `connection`
- `authentication`
- `validation`
- `timeout`
- `command`
- `unsupported`
- `internal`
- `external`

## Run A Task

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::Genja;

    fn main() -> Result<(), genja::GenjaError> {
        let genja = Genja::from_settings_file("settings.yaml")?;
        let results = genja.run_task(CollectFacts { name: "collect_facts" }, 1)?;

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

    genja = genja_lib.Genja.from_settings_file("settings.yaml")
    results = genja.run_task(CollectFacts, max_depth=1)

    print(results.to_json(pretty=True))
    ```

The maximum depth controls nested sub-task execution. Use `0` when only the
top-level task should run, and a higher value when sub-tasks are expected.

## Processors

Processors are lifecycle hooks selected by task metadata. A processor may
implement one, multiple, or all of these hooks:

- `on_task_start`
- `on_task_finish`
- `on_instance_start`
- `on_instance_finish`

Missing hooks are skipped. Hooks that receive results may return a replacement
result object or `None` to leave the current value unchanged.

Python processor hooks are sync-only. Use normal `def` methods for
`on_task_start`, `on_task_finish`, `on_instance_start`, and
`on_instance_finish`.

Tasks select processors by plugin name. The processor must be registered before
the runtime executes the task.

Rust processor plugins implement two traits: `PluginProcessor` registers the
plugin and returns the processor implementation, while `TaskProcessor` defines
the lifecycle hooks.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use std::sync::Arc;
    use genja::genja_core::task::{
        HostTaskResult, TaskProcessor, TaskProcessorContext,
    };
    use genja::TaskDerive;
    use genja_plugin_manager::PluginManager;
    use genja_plugin_manager::plugin_types::{Plugin, PluginProcessor, Plugins};

    #[derive(Clone)]
    struct AuditProcessor;

    impl Plugin for AuditProcessor {
        fn name(&self) -> String {
            "audit".to_string()
        }
    }

    impl PluginProcessor for AuditProcessor {
        fn processor(&self) -> Arc<dyn TaskProcessor> {
            Arc::new(self.clone())
        }
    }

    impl TaskProcessor for AuditProcessor {
        fn on_instance_finish(
            &self,
            context: &TaskProcessorContext,
            _result: &mut HostTaskResult,
        ) -> Result<(), genja::GenjaError> {
            println!("processed task {}", context.task_name());
            Ok(())
        }
    }

    #[derive(TaskDerive)]
    #[task(processors = ["audit"])]
    struct BackupConfig {
        name: &'static str,
    }

    let mut plugins = PluginManager::new();
    plugins.register_plugin(Plugins::Processor(Box::new(AuditProcessor)));
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    import genja as genja_lib
    from genja.processor import ProcessorPluginBase, TaskProcessorContext
    from genja.task import TaskSuccessResult, task


    class AuditProcessor(ProcessorPluginBase):
        name = "audit"

        def on_instance_finish(
            self,
            context: TaskProcessorContext,
            result: genja_lib.HostTaskResult,
        ) -> dict[str, object]:
            data = result.to_dict()
            data["metadata"] = {
                **(data.get("metadata") or {}),
                "processed_by": context.task_name,
            }
            return data


    @task(name="backup_config", processors=["audit"])
    class BackupConfig:
        def start(self, task, host, context) -> TaskSuccessResult:
            return TaskSuccessResult(summary=f"backed up {host.hostname}")


    plugins = genja_lib.PluginManager()
    plugins.register_plugin(AuditProcessor())
    ```

## Sub-Tasks

Sub-tasks let a task define child work that runs after the parent task. Use
sub-tasks for execution trees such as deploy, validate, and collect logs.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use std::sync::Arc;
    use genja::genja_core::task::Task;
    use genja::TaskDerive;

    #[derive(TaskDerive)]
    struct DeployConfig {
        name: &'static str,
        #[task(subtask)]
        validate_config: Arc<dyn Task>,
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(name="validate_config")
    class ValidateConfig:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(summary=f"validated {host.hostname}")


    @task(name="deploy_config", sub_task=ValidateConfig)
    class DeployConfig:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(summary=f"deployed {host.hostname}")
    ```

## Task Options

Task options are JSON-serializable metadata passed into task execution.

=== ":fontawesome-brands-rust: Rust"

    `TaskDerive` supports task options when the struct defines an
    `options: Option<serde_json::Value>` field.

    ```rust
    use genja::genja_core::inventory::Host;
    use genja::genja_core::task::{
        HostTaskResult, Task, TaskError, TaskInfo, TaskRuntimeContext, TaskSuccess,
    };
    use genja::{async_trait, TaskDerive};
    use serde_json::json;

    #[derive(TaskDerive)]
    struct BackupConfig {
        name: &'static str,
        options: Option<serde_json::Value>,
    }

    #[async_trait]
    impl Task for BackupConfig {
        async fn start(
            &self,
            _host: &Host,
            _context: &TaskRuntimeContext,
        ) -> Result<HostTaskResult, TaskError> {
            let backup_path = self
                .options()
                .and_then(|options| options.get("backup_path"))
                .and_then(|value| value.as_str())
                .unwrap_or("/tmp/configs");

            Ok(HostTaskResult::passed(
                TaskSuccess::new()
                    .with_summary(format!("backup path is {backup_path}")),
            ))
        }
    }

    let task = BackupConfig {
        name: "backup_config",
        options: Some(json!({"backup_path": "/tmp/configs", "compress": true})),
    };
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import Host, TaskInfo, TaskRuntimeContext, TaskSuccessResult, task


    @task(
        name="backup_config",
        options={"backup_path": "/tmp/configs", "compress": True},
    )
    class BackupConfig:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(
                summary=f"backup path is {task.options['backup_path']}",
            )
    ```

## Repository Examples

The repository includes task examples for both languages:

=== ":fontawesome-brands-rust: Rust"

    ```bash
    cargo run -p genja --example run_task
    cargo run -p genja --example run_task_tree
    ```

=== ":fontawesome-brands-python: Python"

    ```bash
    python genja/examples/python/run_task.py
    python genja/examples/python/run_task_tree.py
    ```
