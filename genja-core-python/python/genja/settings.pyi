"""Python settings API for Genja.

Import settings-facing helpers from this module instead of from ``genja``
directly. The top-level package re-exports these names for compatibility, but
``genja.settings`` is the primary public surface for:

- ``Settings``
- ``CoreConfig``
- ``InventoryConfig``
- ``OptionsConfig``
- ``SSHConfig``
- ``RunnerConfig``
- ``RunnerRetryConfig``
- ``LoggingConfig``
"""

from __future__ import annotations

from typing import Any

class OptionsConfig:
    """Inventory file path options.

    These options are used by file-based inventory loading to locate hosts,
    groups, and defaults files.
    """

    def __init__(
        self,
        hosts_file: str | None = None,
        groups_file: str | None = None,
        defaults_file: str | None = None,
    ) -> None:
        """Create inventory file options.

        Args:
            hosts_file: Optional path to a hosts inventory file.
            groups_file: Optional path to a groups inventory file.
            defaults_file: Optional path to a defaults inventory file.
        """
        ...

    @property
    def hosts_file(self) -> str | None:
        """Path to the hosts inventory file, if configured."""
        ...

    @property
    def groups_file(self) -> str | None:
        """Path to the groups inventory file, if configured."""
        ...

    @property
    def defaults_file(self) -> str | None:
        """Path to the defaults inventory file, if configured."""
        ...

class CoreConfig:
    """Core runtime behavior settings."""

    def __init__(self, raise_on_error: bool | None = None) -> None:
        """Create core runtime settings.

        Args:
            raise_on_error: Whether task execution should raise when errors occur.
                When omitted, the runtime default is used.
        """
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
        """Create inventory configuration.

        Args:
            plugin: Inventory plugin name. When omitted, the default file inventory
                plugin is used.
            options: File inventory options.
            transform_function: Optional transform plugin name applied after
                loading inventory.
            transform_function_options: JSON-serializable options passed to the
                transform plugin.

        Raises:
            TypeError: If `transform_function_options` cannot be converted to a
                JSON-compatible value.
        """
        ...

    @property
    def plugin(self) -> str:
        """Inventory plugin name."""
        ...

    @property
    def options(self) -> OptionsConfig:
        """Inventory plugin options."""
        ...

    @property
    def transform_function(self) -> str | None:
        """Transform plugin name, if configured."""
        ...

    @property
    def transform_function_options(self) -> Any | None:
        """Options passed to the transform plugin, if configured."""
        ...

class SSHConfig:
    """SSH-related runtime configuration."""

    def __init__(self, config_file: str | None = None) -> None:
        """Create SSH configuration.

        Args:
            config_file: Optional path to an SSH config file.
        """
        ...

    def validate(self) -> None:
        """Validate SSH configuration.

        Raises:
            ValueError: If the configured SSH config file or SSH options are
                invalid.
        """
        ...

    @property
    def config_file(self) -> str | None:
        """Path to the SSH config file, if configured."""
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
        """Create runner configuration.

        Args:
            plugin: Runner plugin name. Common values include `serial` and
                `threaded`; custom registered runners may also be used.
            worker_count: Optional maximum worker count for runners that support
                parallelism.
            max_task_depth: Optional maximum nested sub-task depth.
            max_connection_attempts: Optional maximum connection attempts per host.
            retry: Optional default retry behavior for task execution.
        """
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
        """Maximum nested sub-task depth."""
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
        """Create runner retry configuration.

        Args:
            allow: Whether retries are allowed by default.
            max_attempts: Maximum attempts for retryable tasks. Values lower than
                1 are normalized to 1 by the runtime.
            delay_ms: Delay in milliseconds before retry attempts.
        """
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
        """Delay in milliseconds before retry attempts."""
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
        """Create logging configuration.

        Args:
            enabled: Whether runtime logging is enabled.
            level: Logging level such as `info`, `debug`, or `trace`.
            log_file: Path to the runtime log file.
            to_console: Whether logs should also be emitted to the console.
            file_size: Maximum log file size before rotation.
            max_file_count: Maximum number of rotated log files to keep.
        """
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
        """Whether logs are also emitted to the console."""
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
    """Complete Genja runtime settings.

    Settings combine core behavior, inventory loading, SSH, runner, and logging
    configuration. They can be passed to `Genja.from_settings(...)`,
    `Genja.from_hosts(...)`, or `Genja.from_inventory(...)`.
    """

    def __init__(
        self,
        core: CoreConfig | None = None,
        inventory: InventoryConfig | None = None,
        ssh: SSHConfig | None = None,
        runner: RunnerConfig | None = None,
        logging: LoggingConfig | None = None,
    ) -> None:
        """Create settings from optional section configs.

        Omitted sections use runtime defaults.

        Args:
            core: Core runtime settings.
            inventory: Inventory loading settings.
            ssh: SSH settings.
            runner: Runner settings.
            logging: Logging settings.
        """
        ...

    @staticmethod
    def from_file(path: str) -> Settings:
        """Load settings from a YAML or JSON file.

        Args:
            path: Path to the settings file.

        Raises:
            ValueError: If the file cannot be read or parsed as valid settings.
        """
        ...

    def validate(self) -> None:
        """Validate settings.

        Raises:
            ValueError: If any setting is invalid.
        """
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

__all__: list[str]
