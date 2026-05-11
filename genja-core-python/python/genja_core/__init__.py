"""Python bindings for Genja core.

Task authoring helpers live in ``genja_core.task``. They are re-exported here
for compatibility, but new code should prefer:

    from genja_core.task import task, TaskMessage, TaskSuccessResult
"""

from .genja_core import *
from .connection import (
    ConnectionKey,
    ConnectionPluginProtocol,
    ConnectionProtocol,
    ResolvedConnectionParams,
)
from .inventory import InventoryPluginProtocol
from .plugin_manager import PluginManager
from .processor import TaskProcessorContext, TaskProcessorProtocol
from .runner import RunnerPluginProtocol
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
    TaskRuntimeContext,
    TaskFailureResult,
    TaskInfo,
    TaskMessage,
    TaskSkipResult,
    TaskSuccessResult,
    task,
)
from .transform import TransformFunctionPluginProtocol


__doc__ = genja_core.__doc__
if hasattr(genja_core, "__all__"):
    __all__ = genja_core.__all__

__all__ = list(__all__) + [
    "task",
    "ConnectionKey",
    "ResolvedConnectionParams",
    "ConnectionProtocol",
    "ConnectionPluginProtocol",
    "InventoryPluginProtocol",
    "PluginManager",
    "TaskProcessorContext",
    "TaskProcessorProtocol",
    "RunnerPluginProtocol",
    "TransformFunctionPluginProtocol",
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
    "TaskRuntimeContext",
    "TaskMessage",
    "TaskSuccessResult",
    "TaskFailureResult",
    "TaskSkipResult",
]
