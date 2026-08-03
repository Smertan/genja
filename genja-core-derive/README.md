# genja-core-derive

`genja-core-derive` provides the procedural macros used by `genja-core`.

## Macros

- `#[genja_task(...)]` generates `genja_core::task::TaskInfo` and
  `genja_core::task::Task` from an inherent `impl` block.
- `#[derive(DerefMacro)]` generates `std::ops::Deref` for tuple wrappers.
- `#[derive(DerefMutMacro)]` generates `std::ops::DerefMut` for tuple wrappers.

## Task Authoring

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

`#[genja_task(...)]` also validates optional task hooks. Use
`supports_dry_run = true` with `dry_run(...)` or `dry_run_async(...)`, and use
`idempotency = IdempotencyMode::Check` or
`idempotency = IdempotencyMode::CheckAndVerify` with `check(...)` or
`check_async(...)`. The hook must match the task entrypoint mode.
