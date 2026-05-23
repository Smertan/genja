"""Python bindings for Genja core.

Task authoring helpers live in ``genja.task``. They are re-exported here
for compatibility, but new code should prefer:

    from genja.task import task, TaskMessage, TaskSuccessResult
"""

from .genja import (
    CoreConfig,
    Genja,
    GenjaBuilder,
    HostTaskResult,
    InventoryConfig,
    LoggingConfig,
    OptionsConfig,
    PluginManager as _CorePluginManager,
    RunnerConfig,
    SSHConfig,
    Settings as _CoreSettings,
    TaskConnectionResolver,
    TaskDefinition,
    TaskResults,
    Tasks,
)
from .connection import (
    ConnectionKey,
    ConnectionPluginProtocol,
    ConnectionProtocol,
    ResolvedConnectionParams,
)
from .inventory import InventoryPluginProtocol
from .plugin_manager import PluginManager
from .processor import TaskProcessorContext, TaskProcessorProtocol
from .runner import BatchRunnerPluginProtocol, RunnerPluginProtocol
from .settings import Settings
from .task import (
    GenjaTaskProtocol,
    Host,
    TaskFailureKind,
    TaskRuntimeContext,
    TaskFailureResult,
    TaskInfo,
    TaskMessage,
    TaskMessageLevel,
    TaskSkipResult,
    TaskStatus,
    TaskSuccessResult,
    task,
)
from .transform import TransformFunctionPluginProtocol

assert PluginManager is _CorePluginManager
assert Settings is _CoreSettings

__all__ = [
    "HostTaskResult",
    "TaskConnectionResolver",
    "TaskDefinition",
    "Tasks",
    "TaskResults",
    "Genja",
    "GenjaBuilder",
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
    "BatchRunnerPluginProtocol",
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
    "TaskMessageLevel",
    "TaskStatus",
    "TaskFailureKind",
    "TaskSuccessResult",
    "TaskFailureResult",
    "TaskSkipResult",
]
