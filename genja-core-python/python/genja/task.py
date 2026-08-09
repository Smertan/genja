"""Python task authoring API for Genja.

Import task-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.task`` is the primary public surface for:

- ``@task(...)`` task metadata decoration
- task processor selection metadata
- ``TaskMessage``
- ``TaskSuccessResult``
- ``TaskFailureResult``
- ``TaskSkipResult``
- ``RetryConfig``
- ``SessionVerificationConfig``
- ``IdempotencyMode``
- ``IdempotencyCheckResult``
- task ``options`` metadata

The canonical authoring shape is:

.. code-block:: python

    from genja.task import (
        Host,
        TaskFailureKind,
        TaskFailureResult,
        TaskRuntimeContext,
        IdempotencyCheckResult,
        IdempotencyMode,
        RetryConfig,
        SessionVerificationConfig,
        TaskInfo,
        TaskMessage,
        TaskMessageLevel,
        TaskSuccessResult,
        task,
    )

    @task(
        name="backup_config",
        connection_plugin_name="ssh",
        idempotency=IdempotencyMode.CHECK,
        processors=["audit"],
        retry=RetryConfig(allow=True, max_attempts=3, delay_ms=500),
        session_verification=SessionVerificationConfig(max_attempts=3, delay_ms=5000),
        options={"backup_path": "/tmp/configs", "compress": True},
    )
    class BackupConfigTask:
        def check(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> IdempotencyCheckResult:
            return IdempotencyCheckResult.change_required(
                diff=f"+backup {host.hostname}",
                details={"host": host.hostname},
            )

        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(
                changed=True,
                summary=(
                    f"backed up {host.hostname} to "
                    f"{task.options['backup_path']}"
                ),
                messages=[
                    TaskMessage(
                        level=TaskMessageLevel.INFO,
                        text=(
                            f"task={task.name} "
                            f"has_connection={context.has_connection()}"
                        ),
                    )
                ],
                metadata={
                    "platform": host.platform,
                    "backup_path": task.options["backup_path"],
                    "compress": task.options["compress"],
                },
            )

    class ValidateBackupTask:
        def start(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskFailureResult:
            return TaskFailureResult(
                message=f"backup validation timed out for {host.hostname}",
                kind=TaskFailureKind.TIMEOUT,
                retryable=True,
            )

    @task(name="async_backup", connection_plugin_name="ssh")
    class AsyncBackupTask:
        async def start_async(
            self,
            task: TaskInfo,
            host: Host,
            context: TaskRuntimeContext,
        ) -> TaskSuccessResult:
            return TaskSuccessResult(
                summary=f"asynchronously backed up {host.hostname}",
            )

Task classes must define exactly one execution method:

- ``def start(...)`` for blocking tasks
- ``async def start_async(...)`` for async tasks

Each execution method must resolve to one of:

- ``TaskSuccessResult``
- ``TaskFailureResult``
- ``TaskSkipResult``

Task metadata comes from ``@task(...)``:

- ``name``: required and must be non-empty
- ``connection_plugin_name``: optional; when provided it must be non-empty
- ``sub_tasks``: optional list of decorated task classes
- ``processors``: optional list of processor plugin names
- ``retry``: optional grouped retry metadata
- ``session_verification``: optional post-change new-session verification metadata
- ``idempotency``: optional task-authored convergence check mode
- ``options``: optional JSON-serializable task options payload

Retry metadata only controls policy. A task is retried only when both of these
conditions are true:

- the effective retry policy allows another attempt
- the task returns ``TaskFailureResult(..., retryable=True)``

Genja does not infer whether a task is safe to repeat, mutable, or idempotent.
``delay_ms`` is a fixed local delay before retry attempts.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import datetime
from enum import Enum
import json
from typing import Any, Awaitable, Literal, Protocol, TypeVar, cast

from pydantic import BaseModel, ConfigDict, Field, field_serializer, field_validator

from .genja import IdempotencyCheckResult, IdempotencyMode, SessionVerificationConfig

_TaskClassT = TypeVar("_TaskClassT", bound=type)


@dataclass(frozen=True)
class _DryRunMode:
    article: str
    label: str
    start_method: str
    dry_run_method: str


_SYNC_DRY_RUN = _DryRunMode("a", "sync", "start", "dry_run")
_ASYNC_DRY_RUN = _DryRunMode("an", "async", "start_async", "dry_run_async")
_DRY_RUN_METHOD_SIGNATURE_ARGS = "self, task, host, context"
_CHECK_METHOD_SIGNATURE_ARGS = "self, task, host, context"


def _task_decorator_class_error(cls_name: str, message: str) -> str:
    return f"@task-decorated class '{cls_name}' {message}"


def _dry_run_method_signature(method_name: str) -> str:
    return f"{method_name}({_DRY_RUN_METHOD_SIGNATURE_ARGS})"


def _check_method_signature(method_name: str) -> str:
    return f"{method_name}({_CHECK_METHOD_SIGNATURE_ARGS})"


def _missing_dry_run_method_error(cls_name: str, mode: _DryRunMode) -> str:
    return _task_decorator_class_error(
        cls_name,
        (
            f"is {mode.article} {mode.label} task with supports_dry_run=True, so it must "
            f"define a dry-run method named "
            f"'{_dry_run_method_signature(mode.dry_run_method)}'"
        ),
    )


def _wrong_dry_run_method_error(
    cls_name: str,
    mode: _DryRunMode,
    invalid_method: str,
) -> str:
    return _task_decorator_class_error(
        cls_name,
        (
            f"is {mode.article} {mode.label} task, so dry-run support requires "
            "a dry-run method named "
            f"'{_dry_run_method_signature(mode.dry_run_method)}', "
            f"not '{invalid_method}'"
        ),
    )


def _missing_check_method_error(cls_name: str, mode: _DryRunMode) -> str:
    return _task_decorator_class_error(
        cls_name,
        (
            f"is {mode.article} {mode.label} task with idempotency enabled, so it must "
            "define an idempotency check method named "
            f"'{_check_method_signature('check' if mode is _SYNC_DRY_RUN else 'check_async')}'"
        ),
    )


def _wrong_check_method_error(
    cls_name: str,
    mode: _DryRunMode,
    invalid_method: str,
) -> str:
    expected = "check" if mode is _SYNC_DRY_RUN else "check_async"
    return _task_decorator_class_error(
        cls_name,
        (
            f"is {mode.article} {mode.label} task, so idempotency support requires "
            "a check method named "
            f"'{_check_method_signature(expected)}', "
            f"not '{invalid_method}'"
        ),
    )


def _ensure_json_serializable(value: Any, field_name: str) -> Any:
    """Validate that a value can be serialized to JSON.

    Attempts to serialize the provided value using json.dumps() to ensure
    it contains only JSON-compatible types. If the value is None, it is
    considered valid and returned immediately without serialization checks.

    Args:
        value (Any): The value to validate for JSON serializability.
        field_name (str): The name of the field being validated, used in
            error messages to provide context about which field failed
            validation.

    Returns:
        Any: The original value if it is None or successfully serializes
            to JSON.

    Raises:
        TypeError: If the value cannot be serialized to JSON, with a
            message indicating which field failed validation.
    """
    if value is None:
        return value

    try:
        json.dumps(value)
    except (TypeError, ValueError) as err:
        raise TypeError(f"{field_name} must be JSON-serializable") from err

    return value


class _GenjaModel(BaseModel):
    """Base model class for Genja data structures with dictionary-like access.

    Extends Pydantic's BaseModel to provide convenient dictionary conversion
    and attribute access via subscript notation for all Genja model classes.
    """

    model_config = ConfigDict(arbitrary_types_allowed=True)

    def to_dict(self) -> dict[str, Any]:
        """Convert the model instance to a JSON-serializable dictionary.

        Returns:
            dict[str, Any]: A dictionary representation of the model with all
                fields serialized in JSON-compatible format.
        """
        return self.model_dump(mode="json")

    def __getitem__(self, key: str) -> Any:
        """Enable dictionary-style attribute access using subscript notation.

        Args:
            key (str): The name of the attribute to retrieve.

        Returns:
            Any: The value of the requested attribute.

        Raises:
            KeyError: If the specified field does not exist on the model.
        """
        try:
            return getattr(self, key)
        except AttributeError as err:
            raise KeyError(key) from err


class RetryConfig(_GenjaModel):
    """Optional task retry metadata.

    ``RetryConfig`` groups task-level retry overrides used by ``@task(...)`` and
    ``TaskInfo``. Every field is optional; omitted fields fall back to runner
    retry defaults before built-in defaults are applied by the runtime. The
    runtime applies omitted fields field by field.

    Attributes:
        allow (bool | None): Optional explicit override for whether retries are
            allowed for this task.
        max_attempts (int | None): Optional total-attempt override. When
            provided, it must be at least 1. A value of 1 means the initial
            attempt only.
        delay_ms (int | None): Optional fixed in-process delay before retry
            attempts, in milliseconds. When provided, it must be 0 or greater.
    """

    allow: bool | None = Field(
        default=None,
        description="Optional explicit retry authorization override.",
    )
    max_attempts: int | None = Field(
        default=None,
        description="Optional total-attempt override.",
    )
    delay_ms: int | None = Field(
        default=None,
        description="Optional fixed delay before retry attempts, in milliseconds.",
    )

    @field_validator("max_attempts", mode="before")
    @classmethod
    def _validate_max_attempts(cls, value: Any) -> Any:
        """Validate that ``max_attempts`` is either omitted or positive."""
        if value is None:
            return value
        if isinstance(value, bool) or not isinstance(value, int) or value < 1:
            raise ValueError("max_attempts must be a positive integer or None")
        return value

    @field_validator("delay_ms", mode="before")
    @classmethod
    def _validate_delay_ms(cls, value: Any) -> Any:
        """Validate that ``delay_ms`` is either omitted or non-negative."""
        if value is None:
            return value
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            raise ValueError("delay_ms must be a non-negative integer or None")
        return value


class TaskInfo(_GenjaModel):
    """Task metadata passed into Python task entrypoint methods.

    This class encapsulates all metadata associated with a task execution,
    including the task name, connection configuration, processor plugins,
    custom options, and optional nested sub-task information. Instances of
    this class are provided to task entrypoint methods to give tasks
    access to their configuration and execution context.

    Note:
        When a task has sub-tasks, the parent receives a ``TaskInfo`` with
        ``sub_tasks`` populated for introspection. When a sub-task executes,
        it receives its own separate ``TaskInfo`` instance with its metadata at
        the top level rather than nested under the parent.

    See Also:
        ``TaskRuntimeContext`` for runtime execution state, ``task`` for the
        decorator that populates this metadata, and ``GenjaTaskProtocol`` for
        the task class contract.

    Attributes:
        name (str): Unique task name that identifies this task within the
            Genja execution environment.
        connection_plugin_name (str | None): Optional name of the connection
            plugin used to establish connectivity for this task. Connection
            plugins provide the transport used to talk to target hosts, and
            the resolved connection object is exposed through
            ``TaskRuntimeContext.connection()``. If None, no specific connection
            plugin is configured.
        processors (list[str]): List of processor plugin names that should be
            applied to this task's execution. Processors are lifecycle hooks
            that can observe or modify task execution before and after task or
            host-level result handling. Defaults to an empty list if no
            processors are specified.
        retry (RetryConfig | None): Optional task-level retry overrides. If
            None, runner defaults apply.
        session_verification (SessionVerificationConfig | None): Optional
            post-change new-session verification configuration. If None,
            session verification is disabled.
        supports_dry_run (bool): Whether the task declares a dry-run entrypoint
            that the runtime may call when dry-run execution is requested.
        idempotency (IdempotencyMode): Task-authored convergence check mode.
        options (Any | None): Optional JSON-serializable payload containing
            task-specific configuration options. Can be any JSON-compatible
            data structure or None if no options are provided.
        sub_tasks (list[TaskInfo]): Nested TaskInfo instances representing
            sub-tasks that will be executed after this task. This field allows
            parent tasks to introspect their execution graph. When a sub-task
            runs, it receives its own TaskInfo instance (not one nested under
            the parent).
    """

    name: str = Field(description="Unique task name.")
    connection_plugin_name: str | None = Field(
        default=None,
        description="Connection plugin name used to execute this task.",
    )
    processors: list[str] = Field(
        default_factory=list,
        description="Processor plugin names applied to this task.",
    )
    retry: RetryConfig | None = Field(
        default=None,
        description="Optional grouped task retry overrides.",
    )
    session_verification: SessionVerificationConfig | None = Field(
        default=None,
        description="Optional post-change new-session verification configuration.",
    )
    supports_dry_run: bool = Field(
        default=False,
        description="Whether the task declares dry-run support.",
    )
    idempotency: IdempotencyMode = Field(
        default=IdempotencyMode.DISABLED,
        description="Task-authored convergence check mode.",
    )
    options: Any | None = Field(
        default=None,
        description="JSON-serializable task options payload.",
    )
    sub_tasks: list[TaskInfo] = Field(
        default_factory=list,
        description="Nested task metadata for optional sub-tasks.",
    )

    @field_serializer("idempotency")
    def _serialize_idempotency(self, value: IdempotencyMode) -> str:
        """Serialize the Rust-backed idempotency enum as its stable value."""
        return value.value

    @field_serializer("session_verification")
    def _serialize_session_verification(
        self,
        value: SessionVerificationConfig | None,
    ) -> dict[str, Any] | None:
        """Serialize the Rust-backed session verification config as a dict."""
        return value.to_dict() if value is not None else None


class Host(_GenjaModel):
    """Host payload passed into Python task entrypoint methods.

    This class encapsulates all host-specific information required for task
    execution, including connection credentials, platform details, and
    additional inventory data. Instances of this class are provided to task
    entrypoint methods to give tasks access to the target host's
    configuration and connection parameters.

    Attributes:
        hostname (str): Inventory hostname for the current target. This is
            the unique identifier for the host within the inventory system.
        port (int | None): Network port used for the current host connection.
            If None, the default port for the connection type will be used.
        username (str | None): Username used for the current host connection.
            If None, authentication may use other methods or default credentials.
        password (str | None): Password used for the current host connection.
            If None, authentication may use other methods such as SSH keys or
            tokens.
        platform (str | None): Platform identifier associated with the current
            host, such as "linux", "windows", or vendor-specific identifiers.
            If None, the platform is either unknown or not specified.
        data (Any | None): Additional inventory data attached to the host.
            This can contain any JSON-serializable data structure with
            host-specific variables, metadata, or configuration. If None, no
            additional data is available.
    """

    hostname: str = Field(description="Inventory hostname for the current target.")
    port: int | None = Field(
        default=None,
        description="Network port used for the current host connection.",
    )
    username: str | None = Field(
        default=None,
        description="Username used for the current host connection.",
    )
    password: str | None = Field(
        default=None,
        description="Password used for the current host connection.",
    )
    platform: str | None = Field(
        default=None,
        description="Platform identifier associated with the current host.",
    )
    data: Any | None = Field(
        default=None,
        description="Additional inventory data attached to the host.",
    )


class TaskRuntimeContext:
    """Runtime context passed into Python task entrypoint methods.

    This class encapsulates runtime execution context information provided to
    task entrypoint methods during execution. It includes depth tracking for
    nested task execution, depth limits, dry-run state, and the resolved
    connection object that the task can use to interact with the target host.

    Task code can inspect the current retry attempt through
    ``current_attempt``, dry-run state through ``dry_run``, and the resolved
    connection through ``connection()`` and ``has_connection()``. Depth
    bookkeeping is retained internally by the runtime and is not part of the
    public Python task API.
    """

    def __init__(
        self,
        *,
        current_depth: int = 0,
        max_depth: int | None = None,
        current_attempt: int = 1,
        dry_run: bool = False,
        connection: Any | None = None,
    ) -> None:
        self._current_depth = current_depth
        self._max_depth = max_depth
        self._current_attempt = max(current_attempt, 1)
        self._dry_run = dry_run
        self._connection = connection

    @property
    def current_attempt(self) -> int:
        return self._current_attempt

    @property
    def dry_run(self) -> bool:
        """Whether the current task entrypoint is running in dry-run mode."""
        return self._dry_run

    def connection(self) -> Any | None:
        return self._connection

    def has_connection(self) -> bool:
        return self._connection is not None

    def to_dict(self) -> dict[str, Any]:
        return {
            "current_depth": self._current_depth,
            "max_depth": self._max_depth,
            "current_attempt": self._current_attempt,
            "dry_run": self._dry_run,
            "connection": self._connection,
        }


class GenjaTaskProtocol(Protocol):
    """Structural typing contract for Python-authored Genja task classes.

    This protocol defines the interface that all Genja task classes must
    implement to be recognized and executed by the Genja task runtime. Task
    classes decorated with @task automatically conform to this protocol by
    having the required __genja_task_info__ attribute and start method added
    or validated during decoration.

    Attributes:
        __genja_task_info__ (dict[str, Any]): Dictionary containing task
            metadata set by the @task decorator, including name, connection
            plugin name, processors, options, and optional sub-task references.
    """

    __genja_task_info__: dict[str, Any]

    def start(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult | TaskFailureResult | TaskSkipResult:
        """Execute the task logic against a target host.

        This method contains the core task implementation and is invoked by
        the Genja task runtime when the task is executed. It can be implemented
        as a synchronous function (def). The method receives all necessary
        context about the task, target host, and runtime environment to
        perform its operations.

        Args:
            task (TaskInfo): Metadata about the current task execution,
                including the task name, connection plugin configuration,
                processor plugins, custom options, and optional sub-task
                information.
            host (Host): Information about the target host where the task
                will be executed, including hostname, connection credentials,
                platform identifier, and additional inventory data.
            context (TaskRuntimeContext): Runtime execution context providing
                access to the resolved connection object for communicating with
                the target host.

        Returns:
            TaskSuccessResult | TaskFailureResult | TaskSkipResult: The outcome
                of the task execution.
        """
        ...

    async def start_async(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult | TaskFailureResult | TaskSkipResult:
        """Execute the task logic against a target host asynchronously.

        Implement this method instead of ``start(...)`` for async Python tasks.
        """
        ...

    def dry_run(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult | TaskFailureResult | TaskSkipResult:
        """Preview task behavior for sync dry-run execution."""
        ...

    async def dry_run_async(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> TaskSuccessResult | TaskFailureResult | TaskSkipResult:
        """Preview task behavior for async dry-run execution."""
        ...

    def check(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> IdempotencyCheckResult:
        """Check convergence for sync idempotent execution."""
        ...

    async def check_async(
        self,
        task: TaskInfo,
        host: Host,
        context: TaskRuntimeContext,
    ) -> IdempotencyCheckResult:
        """Check convergence for async idempotent execution."""
        ...


def _validate_sub_tasks(
    cls_name: str,
    sub_tasks: list[type[GenjaTaskProtocol]] | None,
) -> list[type[GenjaTaskProtocol]]:
    if sub_tasks is None:
        return []
    if not isinstance(sub_tasks, list):
        raise TypeError(
            f"@task-decorated class '{cls_name}' sub_tasks must be a list of task classes or None"
        )

    validated: list[type[GenjaTaskProtocol]] = []
    for sub_task in sub_tasks:
        if not isinstance(sub_task, type):
            raise TypeError(
                f"@task-decorated class '{cls_name}' sub_tasks must contain only task classes"
            )
        if not hasattr(sub_task, "__genja_task_info__"):
            raise TypeError(
                f"@task-decorated class '{cls_name}' sub_task '{sub_task.__name__}' must also be decorated with @task"
            )
        validated.append(sub_task)

    return validated


def task(
    name: str,
    connection_plugin_name: str | None = None,
    sub_tasks: list[type[GenjaTaskProtocol]] | None = None,
    processors: list[str] | None = None,
    retry: RetryConfig | None = None,
    session_verification: SessionVerificationConfig | None = None,
    supports_dry_run: bool = False,
    idempotency: IdempotencyMode = IdempotencyMode.DISABLED,
    options: Any | None = None,
    **kwargs: Any,
):
    """Attach Genja task metadata to a Python task class.

    This decorator function attaches execution metadata to a Python class to
    register it as a Genja task. The decorated class must implement exactly one
    of ``start(...)`` or ``start_async(...)``. The decorator validates all
    provided metadata and stores it in the class's
    ``__genja_task_info__`` attribute for use by the Genja task runtime.

    Args:
        name (str): Required unique task name. Must be a non-empty string that
            identifies this task within the Genja execution environment.
        connection_plugin_name (str | None): Optional name of the connection
            plugin to use when executing this task. If provided, must be a
            non-empty string. If None, the task will execute without a specific
            connection plugin.
        sub_tasks (list[type[GenjaTaskProtocol]] | None): Optional nested task
            classes to execute after this task completes. If provided, each
            entry must already be decorated with @task. If None, no sub-tasks
            will be executed.
        processors (list[str] | None): Optional list of processor plugin names
            to apply to this task's execution. If provided, must be a list of
            non-empty strings. If None, no processors will be applied.
        retry (RetryConfig | None): Optional grouped task-level retry
            overrides. This setting does not force retries by itself; the task
            must still return ``TaskFailureResult(..., retryable=True)``. Omitted
            retry fields fall back to runner defaults field by field.
        session_verification (SessionVerificationConfig | None): Optional
            post-change replacement session verification. When provided, the
            task must also declare ``connection_plugin_name``. Defaults to None.
        supports_dry_run (bool): Declares that the task supports dry-run
            execution. Sync tasks must define ``dry_run(...)`` and async tasks
            must define ``dry_run_async(...)`` when this is True.
        idempotency (IdempotencyMode): Declares task-authored convergence check
            behavior. Enabled sync tasks must define ``check(...)`` and enabled
            async tasks must define ``check_async(...)``.
        options (Any | None): Optional JSON-serializable payload containing
            task-specific configuration options. Can be any JSON-compatible
            data structure. If None, no options are provided to the task.

    Returns:
        Callable[[_TaskClassT], _TaskClassT]: A decorator function that accepts
            a task class and returns the same class with Genja task metadata
            attached via the ``__genja_task_info__`` attribute.

    Raises:
        TypeError: If the decorator is applied to a non-class object, if the
            decorated class does not define exactly one callable task entrypoint,
            if the name is not a non-empty string, if connection_plugin_name is
            not a non-empty string or None, if sub_tasks is not a list of
            @task-decorated classes or None, if processors is not a list of
            non-empty strings or None, if retry is not RetryConfig or None, if
            session_verification is not SessionVerificationConfig or None, if
            session_verification is provided without connection_plugin_name, if
            idempotency is not IdempotencyMode, if retry fields are passed
            outside RetryConfig, or if options is not JSON-serializable.
    """
    if "allow_retries" in kwargs:
        raise TypeError(
            "@task got keyword argument 'allow_retries'; did you mean retry=RetryConfig(allow=...)?"
        )
    if "max_task_attempts" in kwargs:
        raise TypeError(
            "@task got keyword argument 'max_task_attempts'; did you mean retry=RetryConfig(max_attempts=...)?"
        )
    if "delay_ms" in kwargs:
        raise TypeError(
            "@task got keyword argument 'delay_ms'; did you mean retry=RetryConfig(delay_ms=...)?"
        )
    if kwargs:
        unknown = next(iter(kwargs))
        raise TypeError(f"@task got unexpected keyword argument '{unknown}'")

    def wrap(cls: _TaskClassT) -> _TaskClassT:
        if not isinstance(cls, type):
            raise TypeError("@task can only decorate classes")

        class_dict = vars(cls)
        start = class_dict.get("start")
        start_async = class_dict.get("start_async")
        has_start = callable(start)
        has_start_async = callable(start_async)
        if has_start == has_start_async:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' must define exactly one of 'start' or 'start_async'"
            )
        if start is not None and not has_start:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' attribute 'start' must be callable"
            )
        if start_async is not None and not has_start_async:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' attribute 'start_async' must be callable"
            )
        dry_run = class_dict.get("dry_run")
        dry_run_async = class_dict.get("dry_run_async")
        has_dry_run = callable(dry_run)
        has_dry_run_async = callable(dry_run_async)
        if dry_run is not None and not has_dry_run:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' attribute 'dry_run' must be callable"
            )
        if dry_run_async is not None and not has_dry_run_async:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' attribute 'dry_run_async' must be callable"
            )
        if not isinstance(supports_dry_run, bool):
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' supports_dry_run must be a bool"
            )
        if not isinstance(idempotency, IdempotencyMode):
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' idempotency must be IdempotencyMode"
            )
        if supports_dry_run:
            if has_start and not has_dry_run:
                raise TypeError(
                    _missing_dry_run_method_error(cls.__name__, _SYNC_DRY_RUN)
                )
            if has_start_async and not has_dry_run_async:
                raise TypeError(
                    _missing_dry_run_method_error(cls.__name__, _ASYNC_DRY_RUN)
                )
            if has_start and has_dry_run_async:
                raise TypeError(
                    _wrong_dry_run_method_error(
                        cls.__name__,
                        _SYNC_DRY_RUN,
                        _ASYNC_DRY_RUN.dry_run_method,
                    )
                )
            if has_start_async and has_dry_run:
                raise TypeError(
                    _wrong_dry_run_method_error(
                        cls.__name__,
                        _ASYNC_DRY_RUN,
                        _SYNC_DRY_RUN.dry_run_method,
                    )
                )
        check = class_dict.get("check")
        check_async = class_dict.get("check_async")
        has_check = callable(check)
        has_check_async = callable(check_async)
        if check is not None and not has_check:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' attribute 'check' must be callable"
            )
        if check_async is not None and not has_check_async:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' attribute 'check_async' must be callable"
            )
        if idempotency != IdempotencyMode.DISABLED:
            if has_start and has_check_async:
                raise TypeError(
                    _wrong_check_method_error(
                        cls.__name__, _SYNC_DRY_RUN, "check_async"
                    )
                )
            if has_start_async and has_check:
                raise TypeError(
                    _wrong_check_method_error(cls.__name__, _ASYNC_DRY_RUN, "check")
                )
            if has_start and not has_check:
                raise TypeError(
                    _missing_check_method_error(cls.__name__, _SYNC_DRY_RUN)
                )
            if has_start_async and not has_check_async:
                raise TypeError(
                    _missing_check_method_error(cls.__name__, _ASYNC_DRY_RUN)
                )
        if not isinstance(name, str) or not name.strip():
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' name must be a non-empty string"
            )

        validated_sub_tasks = _validate_sub_tasks(cls.__name__, sub_tasks)
        if connection_plugin_name is not None:
            if (
                not isinstance(connection_plugin_name, str)
                or not connection_plugin_name.strip()
            ):
                raise TypeError(
                    f"@task-decorated class '{cls.__name__}' connection_plugin_name must be a non-empty string or None"
                )
        if processors is not None:
            if not isinstance(processors, list):
                raise TypeError(
                    f"@task-decorated class '{cls.__name__}' processors must be a list of processor names or None"
                )
            for processor_name in processors:
                if not isinstance(processor_name, str) or not processor_name.strip():
                    raise TypeError(
                        f"@task-decorated class '{cls.__name__}' processors must contain non-empty strings"
                    )
        if retry is not None and not isinstance(retry, RetryConfig):
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' retry must be RetryConfig or None"
            )
        if session_verification is not None and not isinstance(
            session_verification, SessionVerificationConfig
        ):
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' session_verification must be SessionVerificationConfig or None"
            )
        if session_verification is not None and connection_plugin_name is None:
            raise TypeError(
                f"@task-decorated class '{cls.__name__}' session_verification requires connection_plugin_name"
            )
        _ensure_json_serializable(options, "options")

        task_cls = cast(type[GenjaTaskProtocol], cls)
        task_cls.__genja_task_info__ = {
            "name": name,
            "connection_plugin_name": connection_plugin_name,
            "processors": list(processors or []),
            "retry": retry.to_dict() if retry is not None else None,
            "session_verification": session_verification,
            "supports_dry_run": supports_dry_run,
            "idempotency": idempotency,
            "options": options,
            "sub_tasks": validated_sub_tasks,
        }
        return cls

    return wrap


class TaskMessage(_GenjaModel):
    """A structured message attached to a task result.

    This class represents a single diagnostic or informational message that
    can be attached to task execution results. Messages provide structured
    logging and diagnostic information about task execution, including
    severity levels, human-readable text, optional machine-readable codes,
    and timestamps. Multiple TaskMessage instances can be included in
    TaskSuccessResult, TaskFailureResult, or other result types to provide
    detailed execution context.

    Attributes:
        level (TaskMessageLevel): The severity level of the message, indicating
            its importance and type (INFO, WARNING, ERROR, or DEBUG). This helps
            consumers filter and prioritize messages based on their significance.
        text (str): Human-readable message text that describes the event,
            condition, or diagnostic information. This is the primary content
            of the message intended for display to users or in logs.
        code (str | None): Optional machine-readable message code that can be
            used for programmatic message identification, filtering, or
            internationalization. If None, no specific code is associated with
            this message.
        timestamp (datetime | None): Optional timestamp indicating when the
            message was generated during task execution. If None, no specific
            timestamp is recorded for this message.
    """

    level: TaskMessageLevel = Field(description="Message severity level.")
    text: str = Field(description="Human-readable message text.")
    code: str | None = Field(
        default=None,
        description="Optional machine-readable message code.",
    )
    timestamp: datetime | None = Field(
        default=None,
        description="Timestamp associated with the message.",
    )


class TaskStatus(str, Enum):
    """Canonical task status values returned by Genja task results.

    This enumeration defines the possible execution states that a Genja task
    can return upon completion. Each status represents a distinct outcome
    category that determines how the task result should be interpreted and
    processed by the Genja execution engine.

    Attributes:
        PASSED (str): Indicates the task completed successfully without errors.
            This status is returned by TaskSuccessResult and signifies that the
            task's intended operation was performed as expected.
        PASSED_WITH_WARNINGS (str): Indicates the task completed successfully
            but produced important non-fatal warnings that should be surfaced
            prominently.
        FAILED (str): Indicates the task encountered an error and could not
            complete successfully. This status is returned by TaskFailureResult
            and signifies that the task's operation failed due to an exception,
            validation error, or other failure condition.
        SKIPPED (str): Indicates the task was intentionally bypassed and did
            not execute its main logic. This status is returned by TaskSkipResult
            and signifies that the task determined it should not run based on
            conditional logic, prerequisites, or other criteria.
    """

    PASSED = "passed"
    PASSED_WITH_WARNINGS = "passed_with_warnings"
    FAILED = "failed"
    SKIPPED = "skipped"


class TaskFailureKind(str, Enum):
    """Canonical task failure categories returned by Genja task results.

    This enumeration defines the types of failures that can occur during task
    execution. Each failure kind categorizes the root cause of a task failure,
    enabling consumers to understand the nature of the error and determine
    appropriate remediation strategies. These categories are used in
    TaskFailureResult to provide structured failure classification.

    Attributes:
        CONNECTION (str): Indicates a failure related to establishing or
            maintaining network connectivity to the target host. This includes
            network timeouts, connection refused errors, and other transport-level
            issues.
        AUTHENTICATION (str): Indicates a failure related to authenticating
            with the target host or service. This includes invalid credentials,
            expired tokens, insufficient permissions, and other authentication
            mechanism failures.
        VALIDATION (str): Indicates a failure related to input validation,
            configuration validation, or precondition checks. This includes
            invalid parameters, malformed data, or unmet prerequisites.
        TIMEOUT (str): Indicates a failure caused by an operation exceeding
            its allowed execution time. This includes command timeouts, response
            timeouts, and other time-bound operation failures.
        COMMAND (str): Indicates a failure related to executing a command or
            operation on the target host. This includes command execution errors,
            non-zero exit codes, and other command-level failures.
        UNSUPPORTED (str): Indicates a failure caused by attempting an operation
            that is not supported by the target platform, connection plugin, or
            current configuration. This includes unsupported features, incompatible
            versions, and unavailable capabilities.
        INTERNAL (str): Indicates a failure caused by an internal error within
            the Genja task implementation or runtime. This includes programming
            errors, unexpected exceptions, and other internal system failures.
        EXTERNAL (str): Indicates a failure caused by an external system or
            dependency. This includes third-party service failures, external API
            errors, and other failures outside the direct control of the task.
    """

    CONNECTION = "connection"
    AUTHENTICATION = "authentication"
    VALIDATION = "validation"
    TIMEOUT = "timeout"
    COMMAND = "command"
    UNSUPPORTED = "unsupported"
    INTERNAL = "internal"
    EXTERNAL = "external"


class TaskMessageLevel(str, Enum):
    """Canonical task message severity levels returned by Genja task results.

    This enumeration defines the severity levels for structured messages
    emitted during task execution. Each level indicates the importance and
    nature of the message, enabling consumers to filter, prioritize, and
    display messages appropriately. These levels are used in TaskMessage
    instances attached to task results.

    Attributes:
        INFO (str): Indicates an informational message that provides general
            context, progress updates, or non-critical details about task
            execution. These messages are typically used for logging normal
            operational events.
        WARNING (str): Indicates a warning message that highlights a potential
            issue, unexpected condition, or non-fatal problem that occurred
            during task execution. These messages alert users to conditions
            that may require attention but did not prevent task completion.
        ERROR (str): Indicates an error message that describes a failure,
            exception, or critical problem that occurred during task execution.
            These messages provide diagnostic information about task failures
            and are typically associated with TaskFailureResult outcomes.
        DEBUG (str): Indicates a debug message that provides detailed technical
            information useful for troubleshooting and development. These
            messages contain verbose diagnostic data and are typically filtered
            out in production environments unless debug logging is enabled.
    """

    INFO = "info"
    WARNING = "warning"
    ERROR = "error"
    DEBUG = "debug"


class TaskSuccessResult(_GenjaModel):
    """Successful task outcome returned from task entrypoint methods.

    This class represents the result of a successfully completed task execution.
    It encapsulates all information about what the task accomplished, including
    the primary result payload, state change indicators, diagnostic messages,
    and additional metadata. Instances of this class are returned by task
    task entrypoint methods when the task completes without errors.

    Attributes:
        status (Literal[TaskStatus.PASSED, TaskStatus.PASSED_WITH_WARNINGS]):
            The task execution status for successful results. Use
            TaskStatus.PASSED_WITH_WARNINGS when the task's desired state is
            satisfied but important non-fatal warnings should be visible in the
            top-level outcome.
        result (Any | None): The primary output or return value produced by
            the task execution. This can contain any JSON-serializable data
            structure representing the task's main result. If None, the task
            completed successfully but did not produce a specific result payload.
        changed (bool): Indicates whether the task modified any remote or
            managed state during execution. Set to True if the task made changes
            to the target system (e.g., modified files, updated configuration,
            created resources). Set to False if the task was idempotent and no
            changes were necessary. Defaults to False.
        diff (str | None): Optional human-readable representation of the changes
            made by the task, typically in unified diff format. This provides
            visibility into what was modified when changed is True. If None, no
            diff information is available or applicable.
        summary (str | None): Optional brief human-readable description of what
            the task accomplished. This provides a concise overview of the task
            outcome suitable for display in logs or user interfaces. If None, no
            summary is provided.
        warnings (list[str]): List of non-fatal warning messages that occurred
            during task execution. These warnings indicate potential issues or
            unexpected conditions that did not prevent task completion. Defaults
            to an empty list if no warnings were generated.
        messages (list[TaskMessage]): List of structured diagnostic messages
            emitted during task execution. These messages provide detailed
            logging and diagnostic information with severity levels, timestamps,
            and optional machine-readable codes. Defaults to an empty list if no
            messages were generated.
        metadata (Any | None): Optional JSON-serializable dictionary or data
            structure containing additional task-specific metadata about the
            execution. This can include performance metrics, resource identifiers,
            or any other supplementary information. If None, no additional
            metadata is provided.
    """

    status: Literal[TaskStatus.PASSED, TaskStatus.PASSED_WITH_WARNINGS] = Field(
        default=TaskStatus.PASSED,
        description="Task status for a successful result.",
    )
    result: Any | None = Field(
        default=None,
        description="Primary task result payload.",
    )
    changed: bool = Field(
        default=False,
        description="Whether the task changed remote or managed state.",
    )
    diff: str | None = Field(
        default=None,
        description="Optional human-readable diff for the applied change.",
    )
    summary: str | None = Field(
        default=None,
        description="Short summary of the successful task outcome.",
    )
    warnings: list[str] = Field(
        default_factory=list,
        description="Non-fatal warnings produced during task execution.",
    )
    messages: list[TaskMessage] = Field(
        default_factory=list,
        description="Structured messages emitted during task execution.",
    )
    metadata: Any | None = Field(
        default=None,
        description="Additional JSON-serializable metadata for the result.",
    )

    @field_validator("metadata", mode="before")
    @classmethod
    def _validate_metadata(cls, value: Any) -> Any:
        """Validate that the metadata field contains only JSON-serializable data.

        This validator is executed before Pydantic's standard validation to ensure
        that the metadata field can be safely serialized to JSON. It delegates to
        the _ensure_json_serializable helper function to perform the actual
        serialization check.

        Args:
            cls (type): The TaskSuccessResult class being validated. This parameter
                is automatically provided by Pydantic's field_validator decorator.
            value (Any): The metadata value to validate for JSON serializability.
                This can be any Python object, but must be convertible to JSON.

        Returns:
            Any: The original metadata value if it is None or successfully passes
                JSON serialization validation.

        Raises:
            TypeError: If the metadata value cannot be serialized to JSON, with a
                message indicating that the metadata field must be JSON-serializable.
        """
        return _ensure_json_serializable(value, "metadata")


class TaskFailureResult(_GenjaModel):
    """Failed task outcome returned from task entrypoint methods.

    This class represents the result of a task execution that encountered an
    error and could not complete successfully. It encapsulates all information
    about the failure, including the error message, failure category, retry
    eligibility, diagnostic messages, and additional failure details. Instances
    of this class are returned by task entrypoint methods when the task
    encounters an error condition.

    Example:
        >>> result = TaskFailureResult(
        ...     message="Failed to connect to router1",
        ...     kind=TaskFailureKind.CONNECTION,
        ...     retryable=True,
        ...     details={"error_code": "ETIMEDOUT", "timeout_seconds": 30},
        ...     messages=[
        ...         TaskMessage(
        ...             level=TaskMessageLevel.ERROR,
        ...             text="Connection attempt 1 of 3 failed",
        ...         )
        ...     ],
        ... )

    Attributes:
        message (str): Human-readable description of the failure that occurred
            during task execution. This message should clearly explain what went
            wrong and provide context for troubleshooting the failure.
        status (Literal[TaskStatus.FAILED]): The task execution status, always
            set to TaskStatus.FAILED for failure results. This field is
            automatically populated and indicates that the task encountered an
            error and did not complete successfully.
        kind (TaskFailureKind): The category of failure that occurred, classifying
            the root cause of the error. This enables consumers to understand the
            nature of the failure and determine appropriate remediation strategies.
            Defaults to TaskFailureKind.EXTERNAL if not specified.
        retryable (bool): Indicates whether retrying the task may succeed. Set to
            True if the failure was caused by a transient condition (e.g., network
            timeout, temporary resource unavailability) that may resolve on retry.
            Set to False if the failure is permanent and retrying will not help
            (e.g., invalid credentials, unsupported operation). Defaults to False.
        details (Any | None): Optional JSON-serializable dictionary or data
            structure containing additional technical details about the failure.
            This can include stack traces, error codes, diagnostic data, or any
            other supplementary information useful for troubleshooting. If None,
            no additional failure details are provided.
        warnings (list[str]): List of non-fatal warning messages that occurred
            before the failure. These warnings provide context about conditions
            or issues that were encountered during task execution prior to the
            final failure. Defaults to an empty list if no warnings were generated.
        messages (list[TaskMessage]): List of structured diagnostic messages
            emitted before the failure occurred. These messages provide detailed
            logging and diagnostic information with severity levels, timestamps,
            and optional machine-readable codes that can help understand the
            execution context leading up to the failure. Defaults to an empty
            list if no messages were generated.
    """

    message: str = Field(description="Human-readable failure message.")
    status: Literal[TaskStatus.FAILED] = Field(
        default=TaskStatus.FAILED,
        description="Task status for a failed result.",
    )
    kind: TaskFailureKind = Field(
        default=TaskFailureKind.EXTERNAL,
        description="Failure category identifier.",
    )
    retryable: bool = Field(
        default=False,
        description="Whether the failure may succeed on retry.",
    )
    details: Any | None = Field(
        default=None,
        description="Additional JSON-serializable failure details.",
    )
    warnings: list[str] = Field(
        default_factory=list,
        description="Non-fatal warnings produced before the failure occurred.",
    )
    messages: list[TaskMessage] = Field(
        default_factory=list,
        description="Structured messages emitted before the failure occurred.",
    )

    @field_validator("details", mode="before")
    @classmethod
    def _validate_details(cls, value: Any) -> Any:
        """Validate that the details field contains only JSON-serializable data.

        This validator is executed before Pydantic's standard validation to ensure
        that the details field can be safely serialized to JSON. It delegates to
        the _ensure_json_serializable helper function to perform the actual
        serialization check.

        Args:
            cls (type): The TaskFailureResult class being validated. This parameter
                is automatically provided by Pydantic's field_validator decorator.
            value (Any): The details value to validate for JSON serializability.
                This can be any Python object, but must be convertible to JSON.

        Returns:
            Any: The original details value if it is None or successfully passes
                JSON serialization validation.

        Raises:
            TypeError: If the details value cannot be serialized to JSON, with a
                message indicating that the details field must be JSON-serializable.
        """
        return _ensure_json_serializable(value, "details")


class TaskSkipResult(_GenjaModel):
    """Skipped task outcome returned from task entrypoint methods.

    This class represents the result of a task execution that was intentionally
    bypassed and did not execute its main logic. It encapsulates information
    about why the task was skipped, including both machine-readable reason codes
    and human-readable explanatory messages. Instances of this class are returned
    by task entrypoint methods when the task determines it should not execute
    based on conditional logic, prerequisites, or other criteria.

    Example:
        >>> result = TaskSkipResult(
        ...     reason="maintenance_mode",
        ...     message="Host is currently in maintenance mode",
        ... )

    Attributes:
        status (Literal[TaskStatus.SKIPPED]): The task execution status, always
            set to TaskStatus.SKIPPED for skipped results. This field is
            automatically populated and indicates that the task was intentionally
            bypassed and did not execute its main logic.
        reason (str | None): Optional machine-readable code or identifier that
            categorizes why the task was skipped. This can be used for programmatic
            filtering, conditional logic, or analytics to understand skip patterns.
            Common examples might include "condition_not_met", "already_configured",
            or "platform_unsupported". If None, no specific reason code is provided.
        message (str | None): Optional human-readable explanation describing why
            the task was skipped. This message provides context for users or logs
            about the skip decision and should clearly explain the conditions or
            logic that led to the task being bypassed. If None, no explanatory
            message is provided.
    """

    status: Literal[TaskStatus.SKIPPED] = Field(
        default=TaskStatus.SKIPPED,
        description="Task status for a skipped result.",
    )
    reason: str | None = Field(
        default=None,
        description="Machine-readable reason the task was skipped.",
    )
    message: str | None = Field(
        default=None,
        description="Human-readable explanation for the skipped task.",
    )


__all__ = [
    "task",
    "GenjaTaskProtocol",
    "RetryConfig",
    "SessionVerificationConfig",
    "IdempotencyMode",
    "IdempotencyCheckResult",
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
