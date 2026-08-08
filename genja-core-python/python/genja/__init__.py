"""Python bindings for Genja core.

Task authoring helpers live in ``genja.task``. They are re-exported here
for compatibility, but new code should prefer:

    from genja.task import (
        IdempotencyCheckResult,
        IdempotencyMode,
        task,
        TaskMessage,
        TaskSuccessResult,
    )
"""

from .genja import (
    CoreConfig,
    Genja,
    GenjaBuilder,
    HostTaskResult,
    IdempotencyCheckResult,
    IdempotencyMode,
    InventoryConfig,
    LoggingConfig,
    OptionsConfig,
    PluginManager as _CorePluginManager,
    RunnerConfig,
    RunnerRetryConfig,
    SessionVerificationConfig,
    SSHConfig,
    Settings as _CoreSettings,
    TaskConnectionResolver,
    TaskDefinition,
    TaskRunOptions,
    TaskResults,
    Tasks,
)
from .connection import (
    ConnectionBase,
    ConnectionKey,
    ConnectionPluginBase,
    ResolvedConnectionParams,
)
from .inventory import InventoryPluginBase
from .plugin_manager import PluginManager
from .plugin import PluginBase
from .processor import (
    ProcessorPluginBase,
    TaskProcessorContext,
)
from .runner import BatchRunnerPluginBase, RunnerPluginBase
from .settings import Settings
from .task import (
    GenjaTaskProtocol,
    Host,
    RetryConfig,
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
    TransformFunctionPluginBase,
)

assert PluginManager is _CorePluginManager
assert Settings is _CoreSettings

__all__ = [
    "HostTaskResult",
    "IdempotencyCheckResult",
    "IdempotencyMode",
    "TaskConnectionResolver",
    "TaskDefinition",
    "TaskRunOptions",
    "Tasks",
    "TaskResults",
    "Genja",
    "GenjaBuilder",
    "task",
    "ConnectionKey",
    "ResolvedConnectionParams",
    "ConnectionBase",
    "ConnectionPluginBase",
    "InventoryPluginBase",
    "PluginManager",
    "PluginBase",
    "TaskProcessorContext",
    "ProcessorPluginBase",
    "RunnerPluginBase",
    "BatchRunnerPluginBase",
    "TransformFunctionPluginBase",
    "Settings",
    "CoreConfig",
    "InventoryConfig",
    "OptionsConfig",
    "SSHConfig",
    "RunnerConfig",
    "RunnerRetryConfig",
    "SessionVerificationConfig",
    "LoggingConfig",
    "GenjaTaskProtocol",
    "RetryConfig",
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
