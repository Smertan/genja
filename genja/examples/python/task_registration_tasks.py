from genja.task import (
    ExplicitInputSchema,
    TaskFactory,
    TaskRegistration,
    TaskSuccessResult,
    task,
)


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
