"""Second registered task module for multi-module discovery."""

from genja.task import TaskRegistration, TaskSuccessResult, task


@task(
    name="other",
    registration=TaskRegistration(id="acme.other", version="1.0.0"),
)
class OtherTask:
    """Provide another descriptor when imported by discovery."""

    def start(self, task, host, context):
        return TaskSuccessResult(summary="other")
