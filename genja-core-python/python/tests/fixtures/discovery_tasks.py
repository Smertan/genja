"""Registered task module with Python and native import-time output."""

import os

from genja.task import TaskRegistration, TaskSuccessResult, task

print("import message")
os.write(1, b"native import message\n")


@task(
    name="discovered",
    registration=TaskRegistration(id="acme.discovered", version="1.0.0"),
)
class DiscoveredTask:
    def start(self, task, host, context):
        return TaskSuccessResult(summary="done")
