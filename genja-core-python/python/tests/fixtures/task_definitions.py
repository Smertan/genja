def passed_start(self, task, host, context):
    return {"status": "passed"}


async def passed_start_async(self, task, host, context):
    return {"status": "passed"}


class DecoratedBackup:
    def start(self, task, host, context):
        return {"status": "passed"}


class AsyncRuntimeTask:
    __genja_task_info__ = {
        "name": "async_runtime_task",
        "connection_plugin_name": None,
        "processors": [],
        "retry": None,
        "options": None,
        "sub_tasks": [],
    }

    async def start_async(self, task, host, context):
        return {
            "status": "passed",
            "changed": True,
            "summary": f"async handled {host.hostname}",
            "messages": [{"level": "info", "text": task.name}],
            "metadata": {"has_connection": context.has_connection()},
        }
