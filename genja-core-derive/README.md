# genja-core-derive

`genja-core-derive` provides procedural macros used by `genja-core`.

## Macros

- `#[genja_task(...)]` is the primary task authoring API. It generates
  `genja_core::task::TaskInfo` and `genja_core::task::Task` from an inherent
  `impl` block.
- `#[derive(Task)]` is a legacy helper that generates
  `genja_core::task::TaskInfo` and field-driven `sub_tasks()` support.
- `#[derive(DerefMacro)]` generates `std::ops::Deref` for tuple wrappers.
- `#[derive(DerefMutMacro)]` generates `std::ops::DerefMut` for tuple wrappers.

`#[derive(Task)]` does not implement `genja_core::task::Task`. For normal task
authoring, use `#[genja_task(...)]`.

## Recommended Task Authoring

```rust
use genja_core::genja_task;
use genja_core::inventory::Host;
use genja_core::task::{
    HostTaskResult, TaskError, TaskRuntimeContext, TaskSuccess,
};

struct CheckTask;

#[genja_task(name = "check", connection_plugin_name = "ssh")]
impl CheckTask {
    async fn start_async(
        &self,
        _host: &Host,
        _context: &TaskRuntimeContext,
    ) -> Result<HostTaskResult, TaskError> {
        Ok(HostTaskResult::passed(TaskSuccess::new()))
    }
}
```

## Legacy `#[derive(Task)]`

`#[derive(Task)]` is still available as a lower-level, field-driven metadata
helper, but it is no longer the recommended task authoring path.

## Legacy Task Fields

Supported fields:

- `name: String`
- `name: &'static str`
- `connection_plugin_name: String`
- `connection_plugin_name: &'static str`
- `connection_plugin_name: Option<String>`
- `connection_plugin_name: Option<&'static str>`
- `options: Option<serde_json::Value>`
- `processor_names: Vec<String>`
- `#[task(subtask)] child: Arc<dyn Task>`
- `#[task(subtask)] child: std::sync::Arc<dyn Task>`
- `#[task(subtask)] child: Arc<dyn Task + Send + Sync>`
- `#[task(subtask)] child: std::sync::Arc<dyn Task + Send + Sync>`

Empty and whitespace-only connection plugin names are treated as absent.

## Connection Plugins And Options

```rust
use genja_core::task::TaskInfo;
use genja_core_derive::Task as TaskDerive;
use serde_json::json;

#[derive(TaskDerive)]
struct DeployTask {
    name: &'static str,
    connection_plugin_name: Option<String>,
    options: Option<serde_json::Value>,
}

let task = DeployTask {
    name: "deploy",
    connection_plugin_name: Some("ssh".to_string()),
    options: Some(json!({"dry_run": true})),
};

assert_eq!(task.connection_plugin_name(), Some("ssh"));
assert_eq!(task.options(), Some(&json!({"dry_run": true})));
```

## Processor Names

Use `processor_names` when the set is configured at runtime:

```rust
use genja_core::task::TaskInfo;
use genja_core_derive::Task as TaskDerive;

#[derive(TaskDerive)]
struct DeployTask {
    name: &'static str,
    processor_names: Vec<String>,
}

let task = DeployTask {
    name: "deploy",
    processor_names: Vec::new(),
}
.with_processor("audit")
.with_processors(["metrics", "trace"]);

assert_eq!(task.processor_names(), vec!["audit", "metrics", "trace"]);
```

Use `#[task(processors = [...])]` when the set is fixed at compile time:

```rust
use genja_core::task::TaskInfo;
use genja_core_derive::Task as TaskDerive;

#[derive(TaskDerive)]
#[task(processors = ["audit", "metrics"])]
struct DeployTask {
    name: &'static str,
}

let task = DeployTask { name: "deploy" };

assert_eq!(task.processor_names(), vec!["audit", "metrics"]);
```

Do not use both `processor_names` and `#[task(processors = [...])]` on the same
task. Unknown struct-level `#[task(...)]` attributes are rejected.

## Legacy Subtasks

Subtasks are `Arc<dyn Task>` fields marked with `#[task(subtask)]`. Fully
qualified `std::sync::Arc<dyn Task>`, `Arc<dyn Task + Send + Sync>`, and
`std::sync::Arc<dyn Task + Send + Sync>` are also supported. Unknown
field-level `#[task(...)]` attributes are rejected.

A task can have multiple subtasks by declaring multiple fields. Subtasks are
returned in declaration order. Field names should be short, action-oriented
snake_case names that describe the subtask's role in the parent workflow, such
as `validate_config`, `upload_artifact`, `restart_service`, or `verify_health`.
Avoid generic field names like `child`, `subtask1`, or `task_a` in application
code.

```rust
use std::sync::Arc;

use genja_core::task::{SubTasks, Task};
use genja_core_derive::Task as TaskDerive;

#[derive(TaskDerive)]
struct ParentTask {
    name: &'static str,
    #[task(subtask)]
    validate_config: Arc<dyn Task>,
    #[task(subtask)]
    verify_health: Arc<dyn Task>,
}
```

The field name does not become the task result name. The result name comes from
the subtask's own `TaskInfo::name()` implementation.

Task trait aliases such as `Arc<dyn CoreTask>` are not supported. Spell the
trait as `Task` in the subtask field type.

## Deref Wrappers

`DerefMacro` and `DerefMutMacro` expect a tuple wrapper with the wrapped value
in field `0` and a `DerefTarget` trait in scope.

```rust
use genja_core_derive::{DerefMacro, DerefMutMacro};

trait DerefTarget {
    type Target;
}

#[derive(DerefMacro, DerefMutMacro)]
struct Values(Vec<String>);

impl DerefTarget for Values {
    type Target = Vec<String>;
}

let mut values = Values(Vec::new());
values.push("one".to_string());

assert_eq!(values.as_slice(), ["one".to_string()]);
```

## Limitations

The current supported contract does not include:

- generic task structs
- non-static borrowed task names such as `name: &'a str`
- subtasks stored as `Option<Arc<dyn Task>>` or `Vec<Arc<dyn Task>>`
- task trait aliases such as `Arc<dyn CoreTask>`
- unknown `#[task(...)]` helper attributes
- `DerefMacro` or `DerefMutMacro` on non-tuple-wrapper types
- `DerefMacro` without an in-scope `DerefTarget` trait
