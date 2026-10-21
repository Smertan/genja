"""Task-like classes that do not register discovery descriptors."""

from genja.task import TaskSuccessResult, task


class UndecoratedTask:
    def start(self, task, host, context):
        return TaskSuccessResult(summary="not decorated")


@task(name="unregistered")
class UnregisteredTask:
    def start(self, task, host, context):
        return TaskSuccessResult(summary="not registered")
