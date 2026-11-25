# genja-core-derive Macro Surface

This note records the original public-surface baseline audit for
`genja-core-derive`, before broader compile tests and later macro features
were added. The metadata list below is historical. For the current macro
surface, use the `genja_task` rustdoc in `src/lib.rs` and the task guide in
`../docs/tasks.md`.

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

## Current Limitations

These cases are not part of the current supported contract:

- Generic task impl blocks or where clauses.
- `DerefMacro` or `DerefMutMacro` on anything other than a tuple wrapper with
  the target value in field `0`.
- `DerefMacro` without an in-scope `DerefTarget` trait.
