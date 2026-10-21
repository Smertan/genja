from genja.task import RetryConfig, task


def passed_start(self, task, host, context):
    return {"status": "passed"}


async def passed_start_async(self, task, host, context):
    return {"status": "passed"}


class DecoratedBackup:
    def start(self, task, host, context):
        return {"status": "passed"}


@task(name="fixture_verify_backup", connection_plugin_name="ssh")
class FixtureVerifyBackupTask:
    def start(self, task, host, context):
        return {"status": "passed"}


@task(
    name="fixture_backup_config",
    connection_plugin_name="ssh",
    sub_tasks=[FixtureVerifyBackupTask],
)
class FixtureBackupConfigTask:
    def start(self, task, host, context):
        return {"status": "passed"}


@task(
    name="fixture_options_task",
    connection_plugin_name="ssh",
    options={"backup_path": "/tmp/configs", "compress": True},
)
class FixtureOptionsTask:
    def start(self, task, host, context):
        return {"status": "passed"}


@task(
    name="fixture_retry_task",
    connection_plugin_name="ssh",
    retry=RetryConfig(allow=True, max_attempts=3, delay_ms=500),
)
class FixtureRetryTask:
    def start(self, task, host, context):
        return {"status": "passed"}


@task(name="fixture_no_connection_task")
class FixtureNoConnectionTask:
    def start(self, task, host, context):
        return {"status": "passed"}


@task(name="async_runtime_task")
class AsyncRuntimeTask:
    async def start_async(self, task, host, context):
        return {
            "status": "passed",
            "changed": True,
            "summary": f"async handled {host.hostname}",
            "messages": [{"level": "info", "text": task.name}],
            "metadata": {"has_connection": context.has_connection()},
        }
