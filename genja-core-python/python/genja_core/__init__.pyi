from __future__ import annotations

from typing import Any

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
    @staticmethod
    def builder(
        hosts: dict[str, Any],
        settings: Settings | None = None,
        plugin_manager: PluginManager | None = None,
    ) -> GenjaBuilder: ...
    @staticmethod
    def from_hosts(
        hosts: dict[str, Any],
        settings: Settings | None = None,
        plugin_manager: PluginManager | None = None,
    ) -> Genja: ...
    @staticmethod
    def from_settings_file(
        path: str,
        plugin_manager: PluginManager | None = None,
    ) -> Genja: ...
    def with_runner(self, runner: str) -> Genja: ...
    def filter_by_key(self, key: str) -> Genja: ...
    def filter_by_key_value(self, key: str, value_pattern: str) -> Genja: ...
    def inventory(self) -> dict[str, dict[str, Any]]: ...
    def iter_inventory_hosts(self) -> list[tuple[str, dict[str, Any]]]: ...
    def hosts_raw(self) -> dict[str, dict[str, Any]]: ...
    def run_task(
        self,
        task_class: type[GenjaTaskProtocol],
        max_depth: int | None = None,
    ) -> TaskResults: ...

class GenjaBuilder:
    def with_plugin(self, plugin: Any) -> GenjaBuilder: ...
    def with_plugin_manager(self, plugin_manager: PluginManager) -> GenjaBuilder: ...
    def with_runner(self, runner: str) -> GenjaBuilder: ...
    def build(self) -> Genja: ...

__all__: list[str]
