"""Internal asyncio bridge for observing coroutine completion after cancellation."""

from __future__ import annotations

from collections.abc import Awaitable, Callable
from typing import Any


async def run_with_completion(
    awaitable: Awaitable[Any],
    completed: Callable[[bool, Any], None],
) -> None:
    """Report the final coroutine outcome, including cancellation acknowledgement."""
    try:
        result = await awaitable
    except BaseException as error:
        completed(False, error)
    else:
        completed(True, result)
