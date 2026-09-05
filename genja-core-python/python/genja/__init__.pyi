"""Python bindings for Genja core.

Task authoring helpers live in ``genja.task``. They are re-exported here for
compatibility, but new code should prefer:

    from genja.task import task, TaskMessage, TaskSuccessResult
"""

from __future__ import annotations

from typing import Any, Awaitable, Callable, ClassVar

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
from .genja import (
    IdempotencyCheckResult,
    IdempotencyMode,
    SessionVerificationConfig,
    TaskFailureKind,
    TaskMessageLevel,
)
from .task import (
    CustomTaskFactory,
    ExplicitInputSchema,
    GenjaTaskProtocol,
    Host,
    PydanticInputSchema,
    RetryConfig,
    TaskDescriptor,
    TaskExecutionMode,
    TaskFactory,
    TaskFactoryStrategy,
    TaskRuntimeContext,
    TaskFailureResult,
    TaskInfo,
    TaskMessage,
    TaskRegistration,
    TaskRegistrationError,
    TaskRegistrationKey,
    TaskSkipResult,
    TaskStatus,
    TaskSuccessResult,
    create_registered_task,
    create_registered_task_by_identity,
    get_registered_task_descriptor,
    get_registered_task_descriptor_by_identity,
    list_registered_tasks,
    parse_task_identity,
    task,
    validate_task_id,
    validate_task_version,
)
from .transform import (
    TransformFunctionPluginBase,
)

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

class IdempotencyMode:
    """Task-authored idempotency check mode."""

    DISABLED: ClassVar[IdempotencyMode]
    CHECK: ClassVar[IdempotencyMode]
    CHECK_AND_VERIFY: ClassVar[IdempotencyMode]

    @property
    def value(self) -> str:
        """Stable string value for this idempotency mode."""
        ...
    def __str__(self) -> str:
        """Return the stable string value for display."""
        ...
    def __repr__(self) -> str:
        """Return the qualified enum constant name."""
        ...
    def __copy__(self) -> IdempotencyMode:
        """Return this immutable enum value for shallow copy operations."""
        ...
    def __deepcopy__(self, memo: dict[int, Any]) -> IdempotencyMode:
        """Return this immutable enum value for deep copy operations."""
        ...

class TaskMessageLevel:
    """Task message severity level backed by the Rust core enum."""

    INFO: ClassVar[TaskMessageLevel]
    WARNING: ClassVar[TaskMessageLevel]
    ERROR: ClassVar[TaskMessageLevel]
    DEBUG: ClassVar[TaskMessageLevel]

    @property
    def value(self) -> str:
        """Stable string value for this task message level."""
        ...
    def __str__(self) -> str:
        """Return the stable string value for display."""
        ...
    def __repr__(self) -> str:
        """Return the qualified enum constant name."""
        ...
    def __copy__(self) -> TaskMessageLevel:
        """Return this immutable enum value for shallow copy operations."""
        ...
    def __deepcopy__(self, memo: dict[int, Any]) -> TaskMessageLevel:
        """Return this immutable enum value for deep copy operations."""
        ...

class TaskFailureKind:
    """Task failure category backed by the Rust core enum."""

    CONNECTION: ClassVar[TaskFailureKind]
    AUTHENTICATION: ClassVar[TaskFailureKind]
    VALIDATION: ClassVar[TaskFailureKind]
    TIMEOUT: ClassVar[TaskFailureKind]
    COMMAND: ClassVar[TaskFailureKind]
    UNSUPPORTED: ClassVar[TaskFailureKind]
    INTERNAL: ClassVar[TaskFailureKind]
    EXTERNAL: ClassVar[TaskFailureKind]

    @property
    def value(self) -> str:
        """Stable string value for this task failure kind."""
        ...
    def __str__(self) -> str:
        """Return the stable string value for display."""
        ...
    def __repr__(self) -> str:
        """Return the qualified enum constant name."""
        ...
    def __copy__(self) -> TaskFailureKind:
        """Return this immutable enum value for shallow copy operations."""
        ...
    def __deepcopy__(self, memo: dict[int, Any]) -> TaskFailureKind:
        """Return this immutable enum value for deep copy operations."""
        ...

class IdempotencyCheckResult:
    """Result returned from Python idempotency check methods."""

    @staticmethod
    def converged(
        summary: str | None = None,
        details: Any | None = None,
    ) -> IdempotencyCheckResult:
        """Create a check result indicating the host is already converged."""
        ...

    @staticmethod
    def change_required(
        diff: str | None = None,
        details: Any | None = None,
    ) -> IdempotencyCheckResult:
        """Create a check result indicating normal execution should run."""
        ...

    @property
    def status(self) -> str:
        """Current convergence state."""
        ...

    @property
    def summary(self) -> str | None:
        """Human-readable convergence summary, if this result is converged."""
        ...

    @property
    def diff(self) -> str | None:
        """Human-readable remaining diff, if this result requires change."""
        ...

    @property
    def details(self) -> Any | None:
        """Structured JSON-compatible check details."""
        ...

    def to_dict(self) -> dict[str, Any]:
        """Return the check result as a dictionary."""
        ...

class SessionVerificationConfig:
    """Post-change replacement session verification metadata."""

    def __init__(self, max_attempts: int = 1, delay_ms: int = 0) -> None:
        """Create session verification metadata."""
        ...

    @property
    def max_attempts(self) -> int:
        """Maximum total replacement session establishment attempts."""
        ...

    @property
    def delay_ms(self) -> int:
        """Fixed delay between replacement session attempts in milliseconds."""
        ...

    def to_dict(self) -> dict[str, int]:
        """Return the configuration as a JSON-compatible dictionary."""
        ...

class TaskConnectionResolver:
    """Connection resolver used by task execution internals."""

    ...

class TaskRunOptions:
    """Runtime execution controls for task or task-list invocations."""

    def __init__(
        self,
        max_depth: int | None = None,
        dry_run: bool = False,
    ) -> None:
        """Create runtime options for task execution."""
        ...

    @property
    def max_depth(self) -> int | None:
        """Maximum nested sub-task depth, or None to use runner settings."""
        ...

    @property
    def dry_run(self) -> bool:
        """Whether the runtime should call dry-run task entrypoints."""
        ...

    def with_max_depth(self, max_depth: int) -> TaskRunOptions:
        """Return a copy with a different maximum nested sub-task depth."""
        ...

    def with_dry_run(self, dry_run: bool) -> TaskRunOptions:
        """Return a copy with dry-run execution enabled or disabled."""
        ...

    def to_dict(self) -> dict[str, Any]:
        """Return the runtime options as a dictionary."""
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
    def session_verification(self) -> SessionVerificationConfig | None:
        """Post-change replacement session verification metadata, if configured."""
        ...

    @property
    def supports_dry_run(self) -> bool:
        """Whether the task declares dry-run support."""
        ...

    @property
    def idempotency(self) -> IdempotencyMode:
        """Task-authored idempotency check mode."""
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
        run_options: TaskRunOptions | None = None,
    ) -> TaskResults:
        """Execute this task definition against a single host payload."""
        ...

    def run_on_hosts(
        self,
        hosts: dict[str, Any],
        connection_resolver: TaskConnectionResolver | None = None,
        run_options: TaskRunOptions | None = None,
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
    """Runtime for loading inventory, filtering hosts, and running tasks.

    A `Genja` runtime owns the loaded inventory, runtime settings, plugin registry,
    selected runner, and the current host selection. Filtering and runner selection
    are immutable operations: they return a new runtime and leave the original
    runtime unchanged.
    """

    @staticmethod
    def builder(
        hosts: dict[str, Any],
        settings: Settings | None = None,
        plugin_manager: PluginManager | None = None,
    ) -> GenjaBuilder:
        """Create a builder for constructing a `Genja` runtime.

        Args:
            hosts: Inventory data. This may be a mapping of host IDs to host
                payloads, a full inventory structure with `hosts`, `groups`, and
                `defaults`, or an object such as a Pydantic model that can be
                converted with `model_dump()`.
            settings: Optional runtime settings for logging, runner selection,
                inventory options, and other behavior. Defaults are used when this
                is omitted.
            plugin_manager: Optional plugin manager. When omitted, a new manager
                with built-in plugins is created.

        Returns:
            A builder that can be further configured before calling `build()`.

        Raises:
            ValueError: If the inventory cannot be converted or the plugin manager
                cannot be initialized or accessed.

        Example:
            ```python
            runtime = (
                Genja.builder({"router1": {"hostname": "10.0.0.1"}})
                .with_runner("serial")
                .build()
            )
            ```
        """
        ...

    @staticmethod
    def from_hosts(
        hosts: dict[str, Any],
        settings: Settings | None = None,
        plugin_manager: PluginManager | None = None,
    ) -> Genja:
        """Create a runtime directly from a hosts mapping.

        This is the convenience form of `Genja.builder(...).build()` for callers
        that only need to provide host data and optional configuration.

        Args:
            hosts: Mapping of host IDs to host payloads. Host payloads should
                contain at least `hostname` and may include fields such as
                `platform`, `port`, `username`, `password`, `groups`, `data`, and
                `connection_options`.
            settings: Optional runtime settings. Defaults are used when omitted.
            plugin_manager: Optional plugin manager. When omitted, a new manager
                with built-in plugins is created.

        Returns:
            An initialized `Genja` runtime.

        Raises:
            ValueError: If `hosts` is not a valid mapping, a host payload is
                invalid, or the runtime cannot be built.

        Example:
            ```python
            runtime = Genja.from_hosts({
                "router1": {"hostname": "10.0.0.1", "platform": "ios"},
            })
            ```
        """
        ...

    @staticmethod
    def from_inventory(
        inventory: Any,
        settings: Settings | None = None,
        plugin_manager: PluginManager | None = None,
    ) -> Genja:
        """Create a runtime from a full inventory structure.

        Use this when hosts need shared configuration from groups or defaults.

        Args:
            inventory: Inventory data with optional `hosts`, `groups`, and
                `defaults` sections.
            settings: Optional runtime settings. Defaults are used when omitted.
            plugin_manager: Optional plugin manager. When omitted, a new manager
                with built-in plugins is created.

        Returns:
            An initialized `Genja` runtime.

        Raises:
            ValueError: If the inventory structure is invalid or the runtime cannot
                be built.

        Example:
            ```python
            runtime = Genja.from_inventory({
                "hosts": {"router1": {"hostname": "10.0.0.1", "groups": ["core"]}},
                "groups": {"core": {"platform": "ios"}},
                "defaults": {"username": "admin"},
            })
            ```
        """
        ...

    @staticmethod
    def from_settings(
        settings: Settings,
        plugin_manager: PluginManager | None = None,
    ) -> Genja:
        """Build a runtime from an already constructed settings object.

        The settings are validated, inventory is loaded through the configured
        inventory plugin, and the runtime is built with the loaded inventory and
        settings.

        Args:
            settings: Runtime settings containing inventory plugin configuration,
                runner selection, logging configuration, and other options.
            plugin_manager: Optional plugin manager. Pass one when custom Python
                plugins must be registered before inventory is loaded.

        Returns:
            A fully configured `Genja` runtime.

        Raises:
            ValueError: If settings validation, inventory loading, or runtime
                construction fails.

        Example:
            ```python
            settings = Settings(
                inventory=InventoryConfig(
                    options=OptionsConfig(hosts_file="hosts.yaml"),
                ),
            )
            runtime = Genja.from_settings(settings)
            ```
        """
        ...

    @staticmethod
    def from_settings_async(
        settings: Settings,
        plugin_manager: PluginManager | None = None,
    ) -> Awaitable[Genja]:
        """Build a runtime from settings using strict async inventory loading.

        The settings are validated, inventory is loaded through the configured
        async inventory plugin, and the runtime is built with the loaded inventory
        and settings. Sync-only inventory plugins, including the default
        `FileInventoryPlugin`, are rejected.

        Args:
            settings: Runtime settings containing async inventory plugin
                configuration, runner selection, logging configuration, and other
                options.
            plugin_manager: Optional plugin manager. Pass one when custom Python
                async inventory plugins must be registered before inventory is
                loaded.

        Returns:
            An awaitable resolving to a fully configured `Genja` runtime.

        Raises:
            ValueError: If settings validation, async inventory loading, or runtime
                construction fails.

        Example:
            ```python
            runtime = await Genja.from_settings_async(
                Settings(inventory=InventoryConfig(plugin="api_inventory")),
                plugin_manager=plugins,
            )
            ```
        """
        ...

    @staticmethod
    def from_settings_file(
        path: str,
        plugin_manager: PluginManager | None = None,
    ) -> Genja:
        """Load settings from a YAML or JSON file and build a runtime.

        The settings file controls runtime configuration, including inventory
        plugin configuration, runner selection, logging settings, and related
        options. Inventory is loaded using the configured inventory plugin, or the
        default file inventory plugin when no plugin is configured.

        Args:
            path: Path to a YAML or JSON settings file.
            plugin_manager: Optional plugin manager. Pass one when custom Python
                plugins must be registered before loading settings/inventory.

        Returns:
            A fully configured `Genja` runtime.

        Raises:
            ValueError: If the settings file cannot be read or parsed, the settings
                are invalid, inventory loading fails, or the configured runner
                cannot be initialized.
        """
        ...

    @staticmethod
    def from_settings_file_async(
        path: str,
        plugin_manager: PluginManager | None = None,
    ) -> Awaitable[Genja]:
        """Load settings from a file and build a runtime using strict async inventory loading.

        The settings file controls runtime configuration, including inventory
        plugin configuration, runner selection, logging settings, and related
        options. The configured inventory plugin must be async-capable. Sync-only
        inventory plugins, including the default `FileInventoryPlugin`, are
        rejected.

        Args:
            path: Path to a YAML or JSON settings file.
            plugin_manager: Optional plugin manager. Pass one when custom Python
                async inventory plugins must be registered before loading
                settings/inventory.

        Returns:
            An awaitable resolving to a fully configured `Genja` runtime.

        Raises:
            ValueError: If the settings file cannot be read or parsed, settings
                validation fails, async inventory loading fails, or runtime
                construction fails.

        Example:
            ```python
            runtime = await Genja.from_settings_file_async(
                "settings.yaml",
                plugin_manager=plugins,
            )
            ```
        """
        ...

    def with_runner(self, runner: str) -> Genja:
        """Return a new runtime configured to use a runner plugin.

        The original runtime is unchanged. Common runner names include `serial`
        for sequential execution and `threaded` for parallel execution. Custom
        runner names may be used when registered with the plugin manager.

        Args:
            runner: Runner plugin name.

        Returns:
            A new runtime with the selected runner.

        Raises:
            ValueError: If the runner plugin is not registered or cannot be
                initialized.
        """
        ...

    def plugins_loaded(self) -> bool:
        """Return `True` when plugins are loaded and available."""
        ...

    def inventory_loaded(self) -> bool:
        """Return `True` when inventory has been loaded into the runtime."""
        ...

    def settings(self) -> Settings:
        """Return a copy of the runtime settings."""
        ...

    def host_count(self) -> int:
        """Return the number of hosts in the current selection."""
        ...

    def host_ids(self) -> list[str]:
        """Return host IDs in the current selection.

        The list reflects filters applied with `filter_hosts`, `filter_by_key`, or
        `filter_by_key_value`. If no filters have been applied, all inventory host
        IDs are returned.
        """
        ...

    def iter_selected_hosts(self) -> list[tuple[str, dict[str, Any]]]:
        """Return selected host IDs paired with transformed host payloads.

        Host payloads include group/default transformations. Only hosts in the
        current selection are returned.

        Raises:
            ValueError: If inventory is unavailable, selected hosts cannot be
                retrieved, or a host payload cannot be converted to Python.
        """
        ...

    def filter_hosts(self, predicate: Callable[[dict[str, Any]], Any]) -> Genja:
        """Filter selected hosts with a Python predicate.

        The predicate receives the same transformed host dictionary shape returned
        by `iter_selected_hosts()`. Truthy return values keep a host; falsy values
        exclude it. Filtering applies only to the current selection, so it can be
        chained with other filters. The original runtime is unchanged.

        Args:
            predicate: Callable receiving a host dictionary and returning a truthy
                value to keep the host.

        Returns:
            A new runtime with the filtered host selection.

        Raises:
            TypeError: If `predicate` is not callable.
            ValueError: If inventory is unavailable, a host cannot be converted, or
                the predicate raises an exception.

        Example:
            ```python
            lab_ios = runtime.filter_hosts(
                lambda host: (
                    host["platform"] == "ios"
                    and host["data"]["site"]["name"] == "lab-a"
                )
            )
            ```
        """
        ...

    def filter_by_key(self, key: str) -> Genja:
        """Filter selected hosts by key existence.

        The key may be a plain key or a dot path such as `data.site.name`.
        Filtering applies only to the current selection and returns a new runtime;
        the original runtime is unchanged.

        Args:
            key: Key or dot path to check in each transformed host payload.

        Returns:
            A new runtime containing only hosts where the key exists.

        Raises:
            ValueError: If inventory is unavailable or filtering fails.
        """
        ...

    def filter_by_key_value(self, key: str, value_pattern: str) -> Genja:
        """Filter selected hosts by key value using a regular expression.

        Only hosts containing `key` are evaluated against `value_pattern`.
        Filtering applies only to the current selection and returns a new runtime;
        the original runtime is unchanged.

        Args:
            key: Key or dot path to read from each transformed host payload.
            value_pattern: Regular expression matched against the key value.

        Returns:
            A new runtime containing hosts whose key value matches the pattern.

        Raises:
            ValueError: If inventory is unavailable, the regex is invalid, or
                filtering fails.

        Example:
            ```python
            ios = runtime.filter_by_key_value("platform", "^ios$")
            ```
        """
        ...

    def inventory(self) -> dict[str, dict[str, Any]]:
        """Return raw inventory hosts as a dictionary.

        This accessor ignores the current host selection and returns host payloads
        from the loaded inventory before group/default transformations are applied.

        Raises:
            ValueError: If inventory is unavailable or host payloads cannot be
                converted to Python.
        """
        ...

    def inventory_full(self) -> dict[str, Any]:
        """Return the full transformed inventory structure.

        The returned dictionary contains `hosts`, `groups`, and `defaults`. Hosts
        include applied group/default transformations.

        Raises:
            ValueError: If inventory is unavailable or inventory components cannot
                be converted to Python.
        """
        ...

    def inventory_raw(self) -> dict[str, Any]:
        """Return the full raw inventory structure before transforms.

        The returned dictionary contains raw `hosts`, `groups`, and `defaults` as
        loaded from the inventory source.

        Raises:
            ValueError: If inventory is unavailable or raw inventory components
                cannot be converted to Python.
        """
        ...

    def iter_inventory_hosts(self) -> list[tuple[str, dict[str, Any]]]:
        """Return all transformed inventory host IDs and payloads.

        Unlike `iter_selected_hosts()`, this returns every host in the loaded
        inventory regardless of current filters.

        Raises:
            ValueError: If inventory is unavailable, hosts cannot be retrieved, or
                host payloads cannot be converted to Python.
        """
        ...

    def hosts_raw(self) -> dict[str, dict[str, Any]]:
        """Return raw inventory hosts before transforms.

        This returns only the raw hosts mapping, without `groups` or `defaults`.

        Raises:
            ValueError: If inventory is unavailable or raw host payloads cannot be
                converted to Python.
        """
        ...

    def run_task(
        self,
        task_class: type[GenjaTaskProtocol],
        run_options: TaskRunOptions | None = None,
    ) -> TaskResults:
        """Execute one decorated task class against selected hosts.

        The task runs through the configured runner plugin and results are
        aggregated across all currently selected hosts.

        Args:
            task_class: Class decorated with `genja.task.task`.
            run_options: Optional runtime execution controls such as depth and
                dry-run mode.

        Returns:
            Aggregated task results containing passed, failed, and skipped hosts.

        Raises:
            ValueError: If the task class is invalid, task execution fails, the
                runner fails, or the task exceeds the allowed sub-task depth.
        """
        ...

    def run_task_async(
        self,
        task_class: type[GenjaTaskProtocol],
        run_options: TaskRunOptions | None = None,
    ) -> Awaitable[TaskResults]:
        """Asynchronously execute one decorated task class against selected hosts.

        This is the async counterpart to `run_task()` and returns an awaitable that
        resolves to aggregated task results.
        """
        ...

    def run_tasks(
        self,
        tasks: Tasks,
        run_options: TaskRunOptions | None = None,
    ) -> list[TaskResults]:
        """Execute an ordered task collection against selected hosts.

        Each entry in `tasks` is a root task and may declare nested sub-tasks. The
        returned list preserves input order, so `results[n]` corresponds to
        `tasks[n]`.
        """
        ...

    def run_tasks_async(
        self,
        tasks: Tasks,
        run_options: TaskRunOptions | None = None,
    ) -> Awaitable[list[TaskResults]]:
        """Asynchronously execute an ordered task collection against selected hosts.

        This is the async counterpart to `run_tasks()` and preserves task result
        ordering.
        """
        ...

class GenjaBuilder:
    """Builder for constructing a configured `Genja` runtime.

    Builder methods consume the current builder state and return a new builder.
    After `build()` succeeds, the builder is consumed and cannot be reused.
    """

    def with_plugin(self, plugin: Any) -> GenjaBuilder:
        """Register a Python plugin and return a new builder.

        Args:
            plugin: Python plugin object implementing one of the supported plugin
                interfaces, such as an inventory, runner, transform, connection, or
                processor plugin.

        Returns:
            A new builder with the plugin registered.

        Raises:
            ValueError: If the builder has already been consumed or the plugin
                cannot be registered.
        """
        ...

    def with_plugin_manager(self, plugin_manager: PluginManager) -> GenjaBuilder:
        """Replace the builder's plugin manager and return a new builder.

        The provided plugin manager's internal registry is transferred into the
        builder and should not be reused afterward.

        Raises:
            ValueError: If the builder has already been consumed or the plugin
                manager cannot be accessed.
        """
        ...

    def with_runner(self, runner: str) -> GenjaBuilder:
        """Configure the runner plugin and return a new builder.

        Args:
            runner: Runner plugin name, such as `serial`, `threaded`, or a custom
                registered runner.

        Raises:
            ValueError: If the builder has already been consumed or its internal
                state cannot be accessed.
        """
        ...

    def build(self) -> Genja:
        """Build and return the configured runtime.

        The builder is consumed after this call. The runtime is initialized with
        the configured inventory, settings, plugin manager, and runner.

        Raises:
            ValueError: If the builder was already consumed, the runtime cannot be
                constructed, or the selected runner cannot be found or initialized.
        """
        ...

__all__ = [
    "HostTaskResult",
    "IdempotencyMode",
    "IdempotencyCheckResult",
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
    "CustomTaskFactory",
    "ExplicitInputSchema",
    "GenjaTaskProtocol",
    "PydanticInputSchema",
    "RetryConfig",
    "TaskDescriptor",
    "TaskExecutionMode",
    "TaskFactory",
    "TaskFactoryStrategy",
    "TaskRegistration",
    "TaskRegistrationError",
    "TaskRegistrationKey",
    "create_registered_task",
    "create_registered_task_by_identity",
    "get_registered_task_descriptor",
    "get_registered_task_descriptor_by_identity",
    "list_registered_tasks",
    "parse_task_identity",
    "validate_task_id",
    "validate_task_version",
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
