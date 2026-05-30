class AuditProcessor:
    def __init__(self):
        self.events = []

    name = "audit"

    group = "ProcessorPlugin"

    def on_task_start(self, context, results):
        self.events.append(("task_start", context.task_name, context.hostname))

    def on_task_finish(self, context, results):
        self.events.append(("task_finish", context.task_name, context.hostname))
        data = results.to_dict()
        data["summary"] = "processed by audit"
        return data

    def on_instance_start(self, context):
        self.events.append(("instance_start", context.task_name, context.hostname))

    def on_instance_finish(self, context, result):
        self.events.append(("instance_finish", context.task_name, context.hostname))
        data = result.to_dict()
        data["metadata"] = {
            **(data.get("metadata") or {}),
            "processor": "audit",
            "hostname": context.hostname,
        }
        return data


class MinimalAuditProcessor:
    name = "audit"

    group = "ProcessorPlugin"


class UnsupportedGroupPlugin:
    name = "unsupported"

    group = "UnknownPlugin"


class MissingIdentityPlugin:
    pass
