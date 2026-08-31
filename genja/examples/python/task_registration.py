import json

import task_registration_tasks
from genja.task import (
    Host,
    create_registered_task_by_identity,
    list_registered_tasks,
)


TASK_IDENTITY = "acme.examples.backup_config@1.0.0"


def main() -> None:
    print(f"Imported task module: {task_registration_tasks.__name__}")

    descriptors = [
        descriptor.to_dict()
        for descriptor in list_registered_tasks()
        if descriptor.id.startswith("acme.examples.")
    ]

    print("\nRegistered Python task descriptors:")
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
