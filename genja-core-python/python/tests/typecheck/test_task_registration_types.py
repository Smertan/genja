from __future__ import annotations

from collections.abc import Mapping
from typing import Any

from genja.task import (
    CustomTaskFactory,
    ExplicitInputSchema,
    Host,
    PydanticInputSchema,
    TaskDescriptor,
    TaskFactory,
    TaskRegistration,
    TaskRegistrationKey,
    TaskSuccessResult,
    create_registered_task,
    create_registered_task_by_identity,
    get_registered_task_descriptor,
    get_registered_task_descriptor_by_identity,
    list_registered_tasks,
    parse_task_identity,
    task,
)
from pydantic import BaseModel


class RegisteredInput(BaseModel):
    acl_name: str
    rules: list[dict[str, object]]


def build_custom_task(input: Mapping[str, Any]) -> CustomRegisteredTask:
    return CustomRegisteredTask(acl_name=str(input["acl_name"]))


@task(
    name="typecheck_registered_kwargs",
    registration=TaskRegistration(
        id="acme.typecheck.registered_kwargs",
        version="1.0.0",
        factory=TaskFactory.KWARGS,
        input_schema=PydanticInputSchema(model=RegisteredInput),
    ),
)
class KwargsRegisteredTask:
    def __init__(self, acl_name: str, rules: list[dict[str, object]]) -> None:
        self.acl_name = acl_name
        self.rules = rules

    def start(self, task, host: Host, context) -> TaskSuccessResult:
        return TaskSuccessResult(summary=f"{self.acl_name}:{len(self.rules)}")


@task(
    name="typecheck_registered_default",
    registration=TaskRegistration(
        id="acme.typecheck.registered_default",
        version="1.0.0",
        factory=TaskFactory.DEFAULT,
        input_schema=ExplicitInputSchema(value={"type": "object"}),
    ),
)
class DefaultRegisteredTask:
    def start(self, task, host: Host, context) -> TaskSuccessResult:
        return TaskSuccessResult(summary=host.hostname)


@task(
    name="typecheck_registered_custom",
    registration=TaskRegistration(
        id="acme.typecheck.registered_custom",
        version="1.0.0",
        factory=CustomTaskFactory(callable=build_custom_task),
    ),
)
class CustomRegisteredTask:
    def __init__(self, acl_name: str) -> None:
        self.acl_name = acl_name

    def start(self, task, host: Host, context) -> TaskSuccessResult:
        return TaskSuccessResult(summary=self.acl_name)


def check_task_registration_types() -> None:
    descriptors: list[TaskDescriptor] = list_registered_tasks()
    descriptor: TaskDescriptor = get_registered_task_descriptor(
        "acme.typecheck.registered_kwargs",
        "1.0.0",
    )
    descriptor_by_identity: TaskDescriptor = get_registered_task_descriptor_by_identity(
        "acme.typecheck.registered_kwargs@1.0.0"
    )
    key: TaskRegistrationKey = parse_task_identity(
        "acme.typecheck.registered_kwargs@1.0.0"
    )

    kwargs_task = create_registered_task(
        "acme.typecheck.registered_kwargs",
        {"acl_name": "edge-inbound", "rules": []},
        "1.0.0",
    )
    custom_task = create_registered_task_by_identity(
        "acme.typecheck.registered_custom@1.0.0",
        {"acl_name": "edge-inbound"},
    )

    _ = descriptors
    _ = descriptor
    _ = descriptor_by_identity
    _ = key
    _ = kwargs_task
    _ = custom_task
