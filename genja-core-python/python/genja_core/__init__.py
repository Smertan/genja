"""Python bindings for Genja core.

Task authoring helpers live in ``genja_core.task``. They are re-exported here
for compatibility, but new code should prefer:

    from genja_core.task import task, TaskMessage, TaskSuccessResult
"""

from .genja_core import *
from .settings import (
    CoreConfig,
    InventoryConfig,
    LoggingConfig,
    OptionsConfig,
    RunnerConfig,
    SSHConfig,
    Settings,
)
from .task import (
    GenjaTaskProtocol,
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
    "Settings",
    "CoreConfig",
    "InventoryConfig",
    "OptionsConfig",
    "SSHConfig",
    "RunnerConfig",
    "LoggingConfig",
    "GenjaTaskProtocol",
    "TaskInfo",
    "Host",
    "TaskContext",
    "TaskMessage",
    "TaskSuccessResult",
    "TaskFailureResult",
    "TaskSkipResult",
]
