"""Public Python async wrappers for the native runtime awaitables.

These wrappers preserve normal Python coroutine semantics: callers can create
the coroutine object first and await it later, while the Rust-native async
bridge is only entered once the coroutine is actually awaited.
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any, cast

if TYPE_CHECKING:
    from .genja import Genja, PluginManager, Settings, TaskResults, Tasks
    from .task import GenjaTaskProtocol


async def from_settings_async(
    runtime_class: type["Genja"],
    settings: "Settings",
    plugin_manager: "PluginManager | None" = None,
) -> "Genja":
    """Build a runtime from programmatic settings using async inventory loading."""
    runtime_class_any = cast(Any, runtime_class)
    return await runtime_class_any._from_settings_async_native(
        settings,
        plugin_manager=plugin_manager,
    )


async def from_settings_file_async(
    runtime_class: type["Genja"],
    path: str,
    plugin_manager: "PluginManager | None" = None,
) -> "Genja":
    """Build a runtime from a settings file using async inventory loading."""
    runtime_class_any = cast(Any, runtime_class)
    return await runtime_class_any._from_settings_file_async_native(
        path,
        plugin_manager=plugin_manager,
    )


async def run_task_async(
    runtime: "Genja",
    task_class: type["GenjaTaskProtocol"],
    run_options: Any | None = None,
    max_depth: int | None = None,
    dry_run: bool | None = None,
) -> "TaskResults":
    runtime_any = cast(Any, runtime)
    return await runtime_any._run_task_async_native(
        task_class,
        run_options,
        max_depth=max_depth,
        dry_run=dry_run,
    )


async def run_tasks_async(
    runtime: "Genja",
    tasks: "Tasks",
    run_options: Any | None = None,
    max_depth: int | None = None,
    dry_run: bool | None = None,
) -> list["TaskResults"]:
    runtime_any = cast(Any, runtime)
    return await runtime_any._run_tasks_async_native(
        tasks,
        run_options,
        max_depth=max_depth,
        dry_run=dry_run,
    )
