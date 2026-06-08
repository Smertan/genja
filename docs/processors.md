# Processors

Processor plugins run lifecycle hooks around task execution and result handling.
Tasks opt into processors by plugin name, which keeps task behavior explicit and
per-task rather than global.

## Hook Order

A processor may implement one, multiple, or all of these hooks:

- `on_task_start`
- `on_task_finish`
- `on_instance_start`
- `on_instance_finish`

Missing hooks are skipped. Hooks that receive results may return a replacement
result object or `None` to leave the current value unchanged.

Processor names are resolved before execution starts. Unknown processor names
return `GenjaError::PluginNotFound`.

## Sync Only

Processor hooks are sync-only in both languages. They mirror the Rust
`TaskProcessor` trait, so implement them with normal `fn` or `def` methods.

## Select Processors On A Task

Tasks select processors by plugin name. The processor must already be
registered before the runtime executes the task.

=== ":fontawesome-brands-rust: Rust"

    ```rust
    use genja::genja_task;

    struct BackupConfig;

    #[genja_task(name = "backup_config", processors = ["audit"])]
    impl BackupConfig {
        async fn start_async(
            &self,
            _host: &genja::genja_core::inventory::Host,
            _context: &genja::genja_core::task::TaskRuntimeContext,
        ) -> Result<genja::genja_core::task::HostTaskResult, genja::genja_core::task::TaskError> {
            Ok(genja::genja_core::task::HostTaskResult::passed(
                genja::genja_core::task::TaskSuccess::new(),
            ))
        }
    }
    ```

=== ":fontawesome-brands-python: Python"

    ```python
    from genja.task import task


    @task(name="backup_config", processors=["audit"])
    class BackupConfig:
        ...
    ```

## Rust Processor Plugins

Rust processor plugins implement two traits:

- `PluginProcessor` registers the plugin and returns the processor instance
- `TaskProcessor` defines the lifecycle hooks

```rust
use std::sync::Arc;
use genja::genja_core::task::{
    HostTaskResult, TaskProcessor, TaskProcessorContext,
};
use genja::genja_task;
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

struct BackupConfig;

#[genja_task(name = "backup_config", processors = ["audit"])]
impl BackupConfig {
    async fn start_async(
        &self,
        _host: &genja::genja_core::inventory::Host,
        _context: &genja::genja_core::task::TaskRuntimeContext,
    ) -> Result<genja::genja_core::task::HostTaskResult, genja::genja_core::task::TaskError> {
        Ok(genja::genja_core::task::HostTaskResult::passed(
            genja::genja_core::task::TaskSuccess::new(),
        ))
    }
}

let mut plugins = PluginManager::new();
plugins.register_plugin(Plugins::Processor(Box::new(AuditProcessor)));
```

## Python Processor Plugins

Python processor plugins extend `ProcessorPluginBase`. The base class provides
the locked `group` value and defines the supported hook names.

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

## Context Data

Processor hooks receive `TaskProcessorContext`, which identifies the current
task execution. Use it for task names, parent task names, host names, and depth
information needed for auditing or result decoration.

## Common Uses

Processor plugins are a good fit for:

- audit metadata
- result normalization
- centralized warnings or annotations
- lightweight execution tracing

They are a weaker fit for:

- inventory mutation
- connection lifecycle work
- task execution strategy

Those belong in inventory, connection, or runner plugins instead.
