import json

from genja.task import (
    ExplicitInputSchema,
    Host,
    TaskFactory,
    TaskRegistration,
    TaskSuccessResult,
    create_registered_task_by_identity,
    list_registered_tasks,
    task,
)


TASK_IDENTITY = "acme.examples.backup_config@1.0.0"


@task(
    name="backup_config",
    connection_plugin_name="ssh",
    registration=TaskRegistration(
        id="acme.examples.backup_config",
        version="1.0.0",
        description="Backs up selected paths from a network device",
        factory=TaskFactory.KWARGS,
        input_schema=ExplicitInputSchema(
            value={
                "type": "object",
                "required": ["backup_path", "compress"],
                "properties": {
                    "backup_path": {"type": "string"},
                    "compress": {"type": "boolean"},
                },
            },
        ),
    ),
)
class BackupConfig:
    def __init__(self, backup_path: str, compress: bool) -> None:
        self.backup_path = backup_path
        self.compress = compress

    def start(self, task, host, context):
        return TaskSuccessResult(
            changed=True,
            summary=f"backing up {host.hostname} to {self.backup_path}",
            metadata={"compress": self.compress},
        )


@task(
    name="collect_facts",
    registration=TaskRegistration(
        id="acme.examples.collect_facts",
        version="1.0.0",
        description="Collects basic host facts",
        factory=TaskFactory.DEFAULT,
    ),
)
class CollectFacts:
    def start(self, task, host, context):
        return TaskSuccessResult(
            summary=f"collected facts from {host.hostname}",
            metadata={"hostname": host.hostname},
        )


def main() -> None:
    descriptors = [
        descriptor.to_dict()
        for descriptor in list_registered_tasks()
        if descriptor.id.startswith("acme.examples.")
    ]

    print("Registered Python task descriptors:")
    print(json.dumps(descriptors, indent=2))

    task_definition = create_registered_task_by_identity(
        TASK_IDENTITY,
        {
            "backup_path": "/tmp/configs",
            "compress": True,
        },
    )
    result = task_definition.run_on_host(Host(hostname="router1"))

    print("\nConstructed task result:")
    print(result.to_json(pretty=True))


if __name__ == "__main__":
    main()
