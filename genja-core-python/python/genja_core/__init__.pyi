from __future__ import annotations

from typing import Any

from .task import (
    Host,
    TaskContext,
    TaskFailureResult,
    TaskInfo,
    TaskMessage,
    TaskSkipResult,
    TaskSuccessResult,
    task,
)


class OptionsConfig:
    @property
    def hosts_file(self) -> str | None: ...
    @property
    def groups_file(self) -> str | None: ...
    @property
    def defaults_file(self) -> str | None: ...


class CoreConfig:
    @property
    def raise_on_error(self) -> bool: ...


class InventoryConfig:
    @property
    def plugin(self) -> str: ...
    @property
    def options(self) -> OptionsConfig: ...
    @property
    def transform_function(self) -> str | None: ...


class SSHConfig:
    @property
    def config_file(self) -> str | None: ...


class RunnerConfig:
    @property
    def plugin(self) -> str: ...
    @property
    def worker_count(self) -> int | None: ...
    @property
    def max_task_depth(self) -> int: ...
    @property
    def max_connection_attempts(self) -> int: ...


class LoggingConfig:
    @property
    def enabled(self) -> bool: ...
    @property
    def level(self) -> str: ...
    @property
    def log_file(self) -> str: ...
    @property
    def to_console(self) -> bool: ...
    @property
    def file_size(self) -> int: ...
    @property
    def max_file_count(self) -> int: ...


class Settings:
    def __init__(self) -> None: ...

    @staticmethod
    def from_file(path: str) -> Settings: ...

    @property
    def core(self) -> CoreConfig: ...
    @property
    def inventory(self) -> InventoryConfig: ...
    @property
    def ssh(self) -> SSHConfig: ...
    @property
    def runner(self) -> RunnerConfig: ...
    @property
    def logging(self) -> LoggingConfig: ...


class HostTaskResult:
    @staticmethod
    def from_python_result(result: Any) -> HostTaskResult: ...

    @property
    def status(self) -> str: ...

    def to_dict(self) -> dict[str, Any]: ...


class TaskDefinition:
    @staticmethod
    def from_python_class(py_task_class: Any) -> TaskDefinition: ...

    @property
    def name(self) -> str: ...
    @property
    def plugin_name(self) -> str: ...
    @property
    def sub_tasks(self) -> list[TaskDefinition]: ...

    def to_dict(self) -> dict[str, Any]: ...
    def run_on_host(self, host: Any) -> HostTaskResult: ...


__all__: list[str]
