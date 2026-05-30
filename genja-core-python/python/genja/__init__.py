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
from .processor import (
    InstanceFinishProcessorHookProtocol,
    InstanceStartProcessorHookProtocol,
    TaskFinishProcessorHookProtocol,
    TaskProcessorContext,
    TaskProcessorProtocol,
    TaskStartProcessorHookProtocol,
)
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
from .transform import (
    TransformDefaultsHookProtocol,
    TransformFunctionPluginProtocol,
    TransformGroupHookProtocol,
    TransformHostHookProtocol,
)

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
    "TaskStartProcessorHookProtocol",
    "TaskFinishProcessorHookProtocol",
    "InstanceStartProcessorHookProtocol",
    "InstanceFinishProcessorHookProtocol",
    "RunnerPluginProtocol",
    "BatchRunnerPluginProtocol",
    "TransformFunctionPluginProtocol",
    "TransformHostHookProtocol",
    "TransformGroupHookProtocol",
    "TransformDefaultsHookProtocol",
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
