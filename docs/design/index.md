# Design records

These notes preserve the decisions and reasoning behind core architecture and
execution policies.
They may include historical proposals or work that is not implemented yet.
For current runtime behavior, use the [Tasks](../tasks.md) and
[Runners](../runners.md) guides.

- [Execution policy baseline](execution-policy-baseline.md): original retry,
  deadline, verification, and validation design, with later revisions and the
  implemented ordered-root failure and recovery contract for #194.
- [Task retry policy contract](task-retry-policy-contract.md): retry policy
  selection, precedence, and safety rules.

Crate-specific records remain beside their code:

- `genja-core/DESIGN.md`: connection-manager lifecycle locking.
- `genja-core-derive/DESIGN.md`: historical macro-surface baseline audit.
