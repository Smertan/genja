class AsyncRuntimeTask:
    __genja_task_info__ = {
        "name": "async_runtime_task",
        "connection_plugin_name": None,
        "processors": [],
        "options": None,
        "sub_task": None,
    }

    async def start(self, task, host, context):
        return {
            "status": "passed",
            "changed": True,
            "summary": f"async handled {host.hostname}",
            "messages": [{"level": "info", "text": task.name}],
            "metadata": {
                "current_depth": context.current_depth,
                "max_depth": context.max_depth,
            },
        }
