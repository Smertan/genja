from __future__ import annotations

from typing import Any, Awaitable, Callable

from .task import GenjaTaskProtocol

class PluginManager:
    """Registry for Python and Rust plugins used by a Genja runtime."""

    def __init__(self) -> None:
        """Create a plugin manager preloaded with built-in plugins."""
        ...
    def load_rust_plugins_from_directory(self, path: str) -> None:
        """Load Rust plugins from a filesystem directory."""
        ...
    def register_plugin(self, plugin: Any) -> None:
        """Register a Python plugin object with the manager."""
        ...
    def load_python_plugins_from_pyproject(self, path: str | None = None) -> None:
        """Load Python plugin entry points declared in a pyproject file."""
        ...
    def deregister_plugin(self, name: str) -> str | None:
        """Remove a plugin by name and return the removed plugin group, if found."""
        ...
    def plugin_names(self) -> list[str]:
        """Return registered plugin names."""
        ...
    def plugin_names_and_groups(self) -> list[tuple[str, str]]:
        """Return registered plugin names paired with their plugin groups."""
        ...

class OptionsConfig:
    """Inventory file path options."""

    def __init__(
        self,
        hosts_file: str | None = None,
        groups_file: str | None = None,
        defaults_file: str | None = None,
    ) -> None:
        """Create inventory file options."""
        ...
    @property
    def hosts_file(self) -> str | None:
        """Path to a hosts inventory file, if configured."""
        ...
    @property
    def groups_file(self) -> str | None:
        """Path to a groups inventory file, if configured."""
        ...
    @property
    def defaults_file(self) -> str | None:
        """Path to a defaults inventory file, if configured."""
        ...

class CoreConfig:
    """Core runtime behavior settings."""

    def __init__(self, raise_on_error: bool | None = None) -> None:
        """Create core runtime settings."""
        ...
    @property
    def raise_on_error(self) -> bool:
        """Whether task execution should raise when errors occur."""
        ...

class InventoryConfig:
    """Inventory plugin and transform configuration."""

    def __init__(
        self,
        plugin: str | None = None,
        options: OptionsConfig | None = None,
        transform_function: str | None = None,
        transform_function_options: Any | None = None,
    ) -> None:
        """Create inventory configuration."""
        ...
    @property
    def plugin(self) -> str:
        """Inventory plugin name."""
        ...
    @property
    def options(self) -> OptionsConfig:
        """Inventory plugin file options."""
        ...
    @property
    def transform_function(self) -> str | None:
        """Inventory transform plugin name, if configured."""
        ...
    @property
    def transform_function_options(self) -> Any | None:
        """Options passed to the inventory transform plugin."""
        ...

class SSHConfig:
    """SSH-related runtime configuration."""

    def __init__(self, config_file: str | None = None) -> None:
        """Create SSH configuration."""
        ...
    def validate(self) -> None:
        """Validate the SSH configuration."""
        ...
    @property
    def config_file(self) -> str | None:
        """Path to an SSH config file, if configured."""
        ...

class RunnerConfig:
    """Task runner plugin and execution limit configuration."""

    def __init__(
        self,
        plugin: str | None = None,
        worker_count: int | None = None,
        max_task_depth: int | None = None,
        max_connection_attempts: int | None = None,
        retry: RunnerRetryConfig | None = None,
    ) -> None:
        """Create runner configuration."""
        ...
    @property
    def plugin(self) -> str:
        """Runner plugin name."""
        ...
    @property
    def worker_count(self) -> int | None:
        """Maximum runner worker count, if configured."""
        ...
    @property
    def max_task_depth(self) -> int:
        """Maximum nested task execution depth."""
        ...
    @property
    def max_connection_attempts(self) -> int:
        """Maximum connection attempts per host."""
        ...
    @property
    def retry(self) -> RunnerRetryConfig:
        """Default retry behavior for task execution."""
        ...

class RunnerRetryConfig:
    """Default task retry settings configured at runner scope."""

    def __init__(
        self,
        allow: bool | None = None,
        max_attempts: int | None = None,
        delay_ms: int | None = None,
    ) -> None:
        """Create runner retry configuration."""
        ...
    @property
    def allow(self) -> bool | None:
        """Whether retries are allowed by default."""
        ...
    @property
    def max_attempts(self) -> int | None:
        """Maximum attempts for retryable tasks."""
        ...
    @property
    def delay_ms(self) -> int | None:
        """Delay in milliseconds between retry attempts."""
        ...

class LoggingConfig:
    """Runtime logging configuration."""

    def __init__(
        self,
        enabled: bool | None = None,
        level: str | None = None,
        log_file: str | None = None,
        to_console: bool | None = None,
        file_size: int | None = None,
        max_file_count: int | None = None,
    ) -> None:
        """Create logging configuration."""
        ...
    @property
    def enabled(self) -> bool:
        """Whether runtime logging is enabled."""
        ...
    @property
    def level(self) -> str:
        """Configured logging level."""
        ...
    @property
    def log_file(self) -> str:
        """Path to the runtime log file."""
        ...
    @property
    def to_console(self) -> bool:
        """Whether logs should also be emitted to the console."""
        ...
    @property
    def file_size(self) -> int:
        """Maximum log file size before rotation."""
        ...
    @property
    def max_file_count(self) -> int:
        """Maximum number of rotated log files to keep."""
        ...

class Settings:
    """Complete Genja runtime settings."""

    def __init__(
        self,
        core: CoreConfig | None = None,
        inventory: InventoryConfig | None = None,
        ssh: SSHConfig | None = None,
        runner: RunnerConfig | None = None,
        logging: LoggingConfig | None = None,
    ) -> None:
        """Create runtime settings from optional section configs."""
        ...
    @staticmethod
    def from_file(path: str) -> Settings:
        """Load runtime settings from a YAML or JSON settings file."""
        ...
    def validate(self) -> None:
        """Validate runtime settings and raise ValueError on invalid configuration."""
        ...
    @property
    def core(self) -> CoreConfig:
        """Core runtime settings."""
        ...
    @property
    def inventory(self) -> InventoryConfig:
        """Inventory loading settings."""
        ...
    @property
    def ssh(self) -> SSHConfig:
        """SSH settings."""
        ...
    @property
    def runner(self) -> RunnerConfig:
        """Runner settings."""
        ...
    @property
    def logging(self) -> LoggingConfig:
        """Logging settings."""
        ...

class HostTaskResult:
    """Result for a single task executed on a single host."""

    @staticmethod
    def from_python_result(result: Any) -> HostTaskResult:
        """Convert a Python task result payload into a HostTaskResult."""
        ...
    @property
    def status(self) -> str:
        """Host result status as a lowercase string."""
        ...
    def to_dict(self) -> dict[str, Any]:
        """Return the host task result as a dictionary."""
        ...

class TaskConnectionResolver:
    """Connection resolver used by task execution internals."""

    ...

class TaskDefinition:
    """Runtime task definition built from a decorated Python task class."""

    @staticmethod
    def from_python_class(py_task_class: type[GenjaTaskProtocol]) -> TaskDefinition:
        """Build a task definition from a class decorated with `genja.task.task`."""
        ...
    @property
    def name(self) -> str:
        """Task name."""
        ...
    @property
    def connection_plugin_name(self) -> str | None:
        """Connection plugin selected by the task, if any."""
        ...
    @property
    def retry(self) -> dict[str, Any] | None:
        """Task retry metadata, if configured."""
        ...
    @property
    def sub_tasks(self) -> list[TaskDefinition]:
        """Nested sub-task definitions."""
        ...
    def to_dict(self) -> dict[str, Any]:
        """Return the task definition as a dictionary."""
        ...
    def run_on_host(
        self,
        host: Any,
        connection_resolver: TaskConnectionResolver | None = None,
        max_depth: int = 0,
    ) -> TaskResults:
        """Execute this task definition against a single host payload."""
        ...
    def run_on_hosts(
        self,
        hosts: dict[str, Any],
        connection_resolver: TaskConnectionResolver | None = None,
        max_depth: int = 0,
    ) -> TaskResults:
        """Execute this task definition against multiple host payloads."""
        ...

class Tasks:
    """Ordered collection of task classes for multi-task execution."""

    def __init__(self) -> None:
        """Create an empty task collection."""
        ...
    def add_task(self, task_class: type[GenjaTaskProtocol]) -> None:
        """Append a decorated task class to the collection."""
        ...
    def task_definitions(self) -> list[TaskDefinition]:
        """Return task definitions for all tasks in the collection."""
        ...
    def to_list(self) -> list[TaskDefinition]:
        """Return task definitions as a list."""
        ...
    def __len__(self) -> int:
        """Return the number of tasks in the collection."""
        ...
    def __getitem__(self, index: int) -> TaskDefinition:
        """Return a task definition by index."""
        ...

class TaskResults:
    """Aggregated task execution results."""

    @property
    def task_name(self) -> str:
        """Name of the task these results belong to."""
        ...
    @property
    def passed_hosts(self) -> list[str]:
        """Host IDs that passed."""
        ...
    @property
    def failed_hosts(self) -> list[str]:
        """Host IDs that failed."""
        ...
    @property
    def skipped_hosts(self) -> list[str]:
        """Host IDs that were skipped."""
        ...
    def host_summary(self) -> dict[str, int]:
        """Return pass/fail/skip counts for hosts."""
        ...
    def task_summary(self) -> dict[str, Any]:
        """Return summary information for the task result tree."""
        ...
    def merge(self, other: TaskResults) -> None:
        """Merge another TaskResults object into this one."""
        ...
    def to_dict(self, *, raw: bool = False) -> dict[str, Any]:
        """Return task results as a dictionary."""
        ...
    def to_json(self, *, raw: bool = False, pretty: bool = False) -> str:
        """Return task results as JSON."""
        ...

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

__all__: list[str]
