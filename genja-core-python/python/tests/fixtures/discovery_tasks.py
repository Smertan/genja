"""Registered task module with Python and native import-time output."""

import os

from genja.task import (
    IdempotencyCheckResult,
    IdempotencyMode,
    SessionVerificationConfig,
    TaskRegistration,
    TaskSuccessResult,
    task,
)

print("import message")
os.write(1, b"native import message\n")


@task(
    name="discovered",
    connection_plugin_name="ssh",
    idempotency=IdempotencyMode.CHECK_AND_VERIFY,
    session_verification=SessionVerificationConfig(max_attempts=2, delay_ms=500),
    registration=TaskRegistration(id="acme.discovered", version="1.0.0"),
)
class DiscoveredTask:
    def start(self, task, host, context):
        return TaskSuccessResult(summary="done")

    def check(self, task, host, context):
        return IdempotencyCheckResult.change_required()
