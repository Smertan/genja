# genja-core-derive Macro Surface

This note records the currently supported public surface for
`genja-core-derive`. It is the baseline audit for deciding what the crate
promises before adding broader compile tests or hardening behavior.

## Public Macros

`#[genja_task(...)]`

- Primary public task authoring macro.
- Applies to inherent `impl` blocks.
- Generates `genja_core::task::TaskInfo`.
- Generates `genja_core::task::Task`.
- Infers execution mode from exactly one of:
  - `fn start(...)`
  - `async fn start_async(...)`
- Supports metadata keys:
  - `name = "..."`
  - `connection_plugin_name = "..."`
  - `processors = ["...", "..."]`
- Supports optional helper methods:
  - `options()`
  - `sub_tasks()`

`#[derive(Task)]`

- Generates `genja_core::task::TaskInfo`.
- Generates field-driven `sub_tasks()` support.
- Does not generate `genja_core::task::Task`.
- This is a legacy metadata helper, not the recommended task authoring path.
- Accepts the helper attribute `#[task(...)]`.

`#[derive(DerefMacro)]`

- Generates `std::ops::Deref`.
- Dereferences to `self.0`.
- Requires a `DerefTarget` trait to be in scope for the derived type:
  `<TypeName as DerefTarget>::Target`.

`#[derive(DerefMutMacro)]`

- Generates `std::ops::DerefMut`.
- Mutably dereferences to `self.0`.
- Requires `Self::Target` to already be available, normally from
  `DerefMacro`.

## Supported Legacy `#[derive(Task)]` Inputs

`Task` supports named structs with a required `name` field.

```rust
#[derive(genja_core_derive::Task)]
struct MyTask {
    name: String,
}
```

Supported `Task` fields:

- `name: String`
- `name: &'static str`
- `connection_plugin_name: String`
- `connection_plugin_name: &'static str`
- `connection_plugin_name: Option<String>`
- `connection_plugin_name: Option<&'static str>`
- `options: Option<serde_json::Value>`
- `options: Option<Value>` when `Value` resolves to `serde_json::Value`
- `processor_names: Vec<String>`
- `#[task(subtask)] child: Arc<dyn Task>`
- `#[task(subtask)] child: std::sync::Arc<dyn Task>`
- `#[task(subtask)] child: Arc<dyn Task + Send + Sync>`
- `#[task(subtask)] child: std::sync::Arc<dyn Task + Send + Sync>`

Supported struct-level processor configuration:

```rust
#[derive(genja_core_derive::Task)]
#[task(processors = ["audit", "metrics"])]
struct MyTask {
    name: &'static str,
}
```

`processor_names: Vec<String>` and `#[task(processors = [...])]` are mutually
exclusive. Unknown struct-level `#[task(...)]` attributes are rejected.

## Generated Legacy `#[derive(Task)]` Behavior

For supported inputs, `Task` generates:

- `TaskInfo::name()` from the `name` field.
- `TaskInfo::connection_plugin_name()` from `connection_plugin_name`, with
  empty and whitespace-only names treated as absent.
- `TaskInfo::get_connection_key(hostname)` from `hostname` and the resolved
  connection plugin name.
- `TaskInfo::options()` from `options`, or `None` when the field is absent.
- `TaskInfo::processor_names()` from `processor_names`,
  `#[task(processors = [...])]`, or an empty vector.
- `sub_tasks()` with all fields marked `#[task(subtask)]` in declaration
  order.
- Inherent `with_processor` and `with_processors` builder helpers only when a
  `processor_names: Vec<String>` field is present.

Multiple subtask fields are supported. Field names should be short,
action-oriented snake_case names that describe each subtask's role in the parent
workflow. The field name is only used for collection; task result names come
from each subtask's own `TaskInfo::name()` implementation. Unknown field-level
`#[task(...)]` attributes are rejected.

## Rejected Legacy `#[derive(Task)]` Inputs

The macro should reject these cases with compile errors:

- `Task` used on an enum or union.
- `Task` used on a tuple struct or unit struct.
- Missing `name`.
- `name` with any type other than `String` or `&'static str`.
- `connection_plugin_name` with any unsupported type.
- `options` with any type other than `Option<serde_json::Value>` or
  `Option<Value>`.
- `processor_names` with any type other than `Vec<String>`.
- A field named `processors`; callers must use `processor_names`.
- Both `processor_names` and `#[task(processors = [...])]`.
- `#[task(subtask)]` on a field that is not a supported `Arc<dyn Task>` form.
- Unknown struct-level or field-level `#[task(...)]` helper attributes.

## Current Limitations

These cases are not part of the current supported contract:

- Generic task structs such as `struct MyTask<T>`.
- Task structs with non-static borrowed names such as `name: &'a str`.
- Subtasks stored as `Option<Arc<dyn Task>>` or `Vec<Arc<dyn Task>>`.
- Task trait aliases such as `Arc<dyn CoreTask>`.
- `DerefMacro` or `DerefMutMacro` on anything other than a tuple wrapper with
  the target value in field `0`.
- `DerefMacro` without an in-scope `DerefTarget` trait.
