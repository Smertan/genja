"""Python bindings for Genja core.

Task authoring helpers live in ``genja_core.task``. They are re-exported here
for compatibility, but new code should prefer:

    from genja_core.task import task, TaskMessage, TaskSuccessResult
"""

from .genja_core import *
from .task import (
    Host,
    TaskFailureResult,
    TaskContext,
    TaskInfo,
    TaskMessage,
    TaskSkipResult,
    TaskSuccessResult,
    task,
)


__doc__ = genja_core.__doc__
if hasattr(genja_core, "__all__"):
    __all__ = genja_core.__all__

__all__ = list(__all__) + [
    "task",
    "TaskInfo",
    "Host",
    "TaskContext",
    "TaskMessage",
    "TaskSuccessResult",
    "TaskFailureResult",
    "TaskSkipResult",
]
