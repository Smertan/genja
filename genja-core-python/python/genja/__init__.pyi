from __future__ import annotations

from typing import Any, Awaitable, Callable

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
from .settings import (
    CoreConfig,
    InventoryConfig,
    LoggingConfig,
    OptionsConfig,
    RunnerConfig,
    RunnerRetryConfig,
    SSHConfig,
    Settings,
)
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

class HostTaskResult:
    @staticmethod
    def from_python_result(result: Any) -> HostTaskResult: ...
    @property
    def status(self) -> str: ...
    def to_dict(self) -> dict[str, Any]: ...

class TaskConnectionResolver: ...

class TaskDefinition:
    @staticmethod
    def from_python_class(py_task_class: type[GenjaTaskProtocol]) -> TaskDefinition: ...
    @property
    def name(self) -> str: ...
    @property
    def connection_plugin_name(self) -> str | None: ...
    @property
    def retry(self) -> dict[str, Any] | None: ...
    @property
    def sub_tasks(self) -> list[TaskDefinition]: ...
    def to_dict(self) -> dict[str, Any]: ...
    def run_on_host(
        self,
        host: Any,
        connection_resolver: TaskConnectionResolver | None = None,
        max_depth: int = 0,
    ) -> TaskResults: ...
    def run_on_hosts(
        self,
        hosts: dict[str, Any],
        connection_resolver: TaskConnectionResolver | None = None,
        max_depth: int = 0,
    ) -> TaskResults: ...

class Tasks:
    def __init__(self) -> None: ...
    def add_task(self, task_class: type[GenjaTaskProtocol]) -> None: ...
    def task_definitions(self) -> list[TaskDefinition]: ...
    def to_list(self) -> list[TaskDefinition]: ...
    def __len__(self) -> int: ...
    def __getitem__(self, index: int) -> TaskDefinition: ...

class TaskResults:
    @property
    def task_name(self) -> str: ...
    @property
    def passed_hosts(self) -> list[str]: ...
    @property
    def failed_hosts(self) -> list[str]: ...
    @property
    def skipped_hosts(self) -> list[str]: ...
    def host_summary(self) -> dict[str, int]: ...
    def task_summary(self) -> dict[str, Any]: ...
    def merge(self, other: TaskResults) -> None: ...
    def to_dict(self, *, raw: bool = False) -> dict[str, Any]: ...
    def to_json(self, *, raw: bool = False, pretty: bool = False) -> str: ...

class Genja:
    """Genja runtime for loading inventory, filtering hosts, and running tasks."""

    @staticmethod
    def builder(
        hosts: dict[str, Any],
        settings: Settings | None = None,
        plugin_manager: PluginManager | None = None,
    ) -> GenjaBuilder:
        """Create a builder from hosts data plus optional settings and plugins."""
        ...

    @staticmethod
    def from_hosts(
        hosts: dict[str, Any],
        settings: Settings | None = None,
        plugin_manager: PluginManager | None = None,
    ) -> Genja:
        """Create a runtime directly from a mapping of host IDs to host payloads."""
        ...

    @staticmethod
    def from_inventory(
        inventory: Any,
        settings: Settings | None = None,
        plugin_manager: PluginManager | None = None,
    ) -> Genja:
        """Create a runtime from a full inventory structure."""
        ...

    @staticmethod
    def from_settings(
        settings: Settings,
        plugin_manager: PluginManager | None = None,
    ) -> Genja:
        """Validate settings, load configured inventory, and build a runtime."""
        ...

    @staticmethod
    def from_settings_file(
        path: str,
        plugin_manager: PluginManager | None = None,
    ) -> Genja:
        """Load settings from a file, load inventory, and build a runtime."""
        ...

    def with_runner(self, runner: str) -> Genja:
        """Return a new runtime configured to use the named runner plugin."""
        ...

    def plugins_loaded(self) -> bool:
        """Return whether plugins are loaded for this runtime."""
        ...

    def inventory_loaded(self) -> bool:
        """Return whether inventory is loaded for this runtime."""
        ...

    def settings(self) -> Settings:
        """Return a copy of the runtime settings."""
        ...

    def host_count(self) -> int:
        """Return the number of currently selected hosts."""
        ...

    def host_ids(self) -> list[str]:
        """Return IDs for currently selected hosts."""
        ...

    def iter_selected_hosts(self) -> list[tuple[str, dict[str, Any]]]:
        """Return selected host IDs paired with transformed host payloads."""
        ...

    def filter_hosts(self, predicate: Callable[[dict[str, Any]], Any]) -> Genja:
        """Return a new runtime containing hosts kept by a Python predicate."""
        ...

    def filter_by_key(self, key: str) -> Genja:
        """Return a new runtime containing hosts where a key or dot path exists."""
        ...

    def filter_by_key_value(self, key: str, value_pattern: str) -> Genja:
        """Return a new runtime containing hosts where a value matches a regex."""
        ...

    def inventory(self) -> dict[str, dict[str, Any]]:
        """Return transformed inventory hosts, ignoring the current host filter."""
        ...

    def inventory_full(self) -> dict[str, Any]:
        """Return the full transformed inventory structure."""
        ...

    def inventory_raw(self) -> dict[str, Any]:
        """Return the raw inventory structure before transforms."""
        ...

    def iter_inventory_hosts(self) -> list[tuple[str, dict[str, Any]]]:
        """Return all transformed inventory host IDs and payloads."""
        ...

    def hosts_raw(self) -> dict[str, dict[str, Any]]:
        """Return raw inventory hosts before transforms."""
        ...

    def run_task(
        self,
        task_class: type[GenjaTaskProtocol],
        max_depth: int | None = None,
    ) -> TaskResults:
        """Execute one decorated task class against currently selected hosts."""
        ...

    def run_task_async(
        self,
        task_class: type[GenjaTaskProtocol],
        max_depth: int | None = None,
    ) -> Awaitable[TaskResults]:
        """Asynchronously execute one decorated task class against selected hosts."""
        ...

    def run_tasks(
        self,
        tasks: Tasks,
        max_depth: int | None = None,
    ) -> list[TaskResults]:
        """Execute an ordered task collection against currently selected hosts."""
        ...

    def run_tasks_async(
        self,
        tasks: Tasks,
        max_depth: int | None = None,
    ) -> Awaitable[list[TaskResults]]:
        """Asynchronously execute an ordered task collection against selected hosts."""
        ...

class GenjaBuilder:
    """Builder for constructing a Genja runtime."""

    def with_plugin(self, plugin: Any) -> GenjaBuilder:
        """Return a new builder with a Python plugin registered."""
        ...

    def with_plugin_manager(self, plugin_manager: PluginManager) -> GenjaBuilder:
        """Return a new builder using the provided plugin manager."""
        ...

    def with_runner(self, runner: str) -> GenjaBuilder:
        """Return a new builder configured with the named runner plugin."""
        ...

    def build(self) -> Genja:
        """Build and return the configured Genja runtime."""
        ...

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
