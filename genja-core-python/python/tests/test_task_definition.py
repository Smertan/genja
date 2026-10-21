import time
from typing import cast

import genja
import pytest
from genja.task import (
    CustomTaskFactory,
    ExplicitInputSchema,
    Host,
    GenjaTaskProtocol,
    IdempotencyCheckResult,
    IdempotencyMode,
    PydanticInputSchema,
    RetryConfig,
    SessionVerificationConfig,
    TaskDescriptor,
    TaskFactory,
    TaskFailureKind,
    TaskFailureResult,
    TaskRuntimeContext,
    TaskMessageLevel,
    TaskInfo,
    TaskMessage,
    TaskRegistration,
    TaskRegistrationError,
    TaskStatus,
    TaskSuccessResult,
    create_registered_task,
    create_registered_task_by_identity,
    get_registered_task_descriptor,
    get_registered_task_descriptor_by_identity,
    list_registered_tasks,
    parse_task_identity,
    task,
)
from pydantic import BaseModel, ValidationError


@task(
    name="verify_backup",
    connection_plugin_name="ssh",
    processors=["audit"],
    options={"mode": "strict"},
)
class VerifyBackupTask:
    def start(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskRuntimeContext)
        assert task.processors == ["audit"]
        assert task.options == {"mode": "strict"}
        return TaskSuccessResult(
            summary=f"verified {host.hostname}",
            messages=[TaskMessage(level=TaskMessageLevel.INFO, text=task.name)],
        )


@task(
    name="backup_config",
    connection_plugin_name="ssh",
    sub_tasks=[VerifyBackupTask],
    options={"backup_path": "/tmp/configs", "compress": True},
)
class BackupConfigTask:
    def start(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskRuntimeContext)
        assert task.options == {"backup_path": "/tmp/configs", "compress": True}
        return TaskSuccessResult(
            changed=True,
            summary=f"backed up {host.hostname}",
            metadata={
                "sub_task_name": task.sub_tasks[0].name,
                "backup_path": task.options["backup_path"],
            },
        )


@task(
    name="verify_backup_plain",
    connection_plugin_name="ssh",
    options={"mode": "strict"},
)
class VerifyBackupPlainTask:
    def start(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskRuntimeContext)
        assert task.options == {"mode": "strict"}
        return TaskSuccessResult(
            summary=f"verified {host.hostname}",
            messages=[TaskMessage(level=TaskMessageLevel.INFO, text=task.name)],
        )


@task(
    name="backup_config_plain",
    connection_plugin_name="ssh",
    sub_tasks=[VerifyBackupPlainTask],
    options={"backup_path": "/tmp/configs", "compress": True},
)
class BackupConfigPlainTask:
    def start(self, task, host, context):
        assert isinstance(task, TaskInfo)
        assert isinstance(host, Host)
        assert isinstance(context, TaskRuntimeContext)
        assert task.options == {"backup_path": "/tmp/configs", "compress": True}
        return TaskSuccessResult(
            changed=True,
            summary=f"backed up {host.hostname}",
            metadata={
                "sub_task_name": task.sub_tasks[0].name,
                "backup_path": task.options["backup_path"],
            },
        )


def test_task_definition_from_python_class_extracts_metadata():
    task_definition = genja.TaskDefinition.from_python_class(BackupConfigTask)

    assert task_definition.name == "backup_config"
    assert task_definition.connection_plugin_name == "ssh"
    assert len(task_definition.sub_tasks) == 1
    assert task_definition.sub_tasks[0].name == "verify_backup"
    assert task_definition.to_dict()["options"] == {
        "backup_path": "/tmp/configs",
        "compress": True,
    }
    assert task_definition.sub_tasks[0].to_dict()["processors"] == ["audit"]
    assert task_definition.sub_tasks[0].to_dict()["options"] == {"mode": "strict"}


def test_registered_python_task_descriptor_matches_rust_contract_shape():
    @task(
        name="registered_descriptor_shape",
        connection_plugin_name="ssh",
        processors=["audit"],
        retry=RetryConfig(allow=True, max_attempts=3, delay_ms=500),
        registration=TaskRegistration(
            id="acme.tests.registered_descriptor_shape",
            version="1.0.0",
            description="Registered descriptor shape test",
            factory=TaskFactory.DEFAULT,
            input_schema=ExplicitInputSchema(
                value={
                    "type": "object",
                    "additionalProperties": False,
                }
            ),
        ),
    )
    class RegisteredDescriptorShapeTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="registered")

    descriptor = get_registered_task_descriptor_by_identity(
        "acme.tests.registered_descriptor_shape@1.0.0"
    )

    assert isinstance(descriptor, TaskDescriptor)
    assert descriptor.to_dict() == {
        "id": "acme.tests.registered_descriptor_shape",
        "id_source": "explicit",
        "name": "registered_descriptor_shape",
        "version": "1.0.0",
        "description": "Registered descriptor shape test",
        "execution_mode": "blocking",
        "connection_plugin_name": "ssh",
        "processor_names": ["audit"],
        "retry": {
            "allow": True,
            "max_attempts": 3,
            "delay_ms": 500,
        },
        "input_schema": {
            "type": "object",
            "additionalProperties": False,
        },
        "constructible": True,
    }
    assert (
        RegisteredDescriptorShapeTask.__genja_task_registration__["descriptor"]
        == descriptor.to_dict()
    )


def test_registered_python_tasks_can_be_listed_and_looked_up():
    @task(
        name="registered_lookup",
        registration=TaskRegistration(
            id="acme.tests.registered_lookup",
            version="2.0.0",
            factory=TaskFactory.DEFAULT,
        ),
    )
    class RegisteredLookupTask:
        """Lookup task docstring."""

        def start(self, task, host, context):
            return TaskSuccessResult(summary="lookup")

    descriptor = get_registered_task_descriptor(
        "acme.tests.registered_lookup",
        "2.0.0",
    )
    listed = list_registered_tasks()
    key = parse_task_identity("acme.tests.registered_lookup@2.0.0")

    assert descriptor.description == "Lookup task docstring."
    assert descriptor.identity == "acme.tests.registered_lookup@2.0.0"
    assert key.id == "acme.tests.registered_lookup"
    assert key.version == "2.0.0"
    assert descriptor in listed


def test_registered_python_task_rejects_duplicate_id_and_version():
    @task(
        name="duplicate_registration_a",
        registration=TaskRegistration(
            id="acme.tests.duplicate_registration",
            version="1.0.0",
            factory=TaskFactory.DEFAULT,
        ),
    )
    class DuplicateRegistrationTaskA:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="duplicate-a")

    with pytest.raises(
        TaskRegistrationError,
        match="duplicate task registration `acme.tests.duplicate_registration@1.0.0`",
    ):

        @task(
            name="duplicate_registration_b",
            registration=TaskRegistration(
                id="acme.tests.duplicate_registration",
                version="1.0.0",
                factory=TaskFactory.DEFAULT,
            ),
        )
        class DuplicateRegistrationTaskB:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="duplicate-b")


def test_registered_python_task_kwargs_factory_constructs_from_input():
    @task(
        name="kwargs_registered",
        registration=TaskRegistration(
            id="acme.tests.kwargs_registered",
            version="1.0.0",
            factory=TaskFactory.KWARGS,
        ),
    )
    class KwargsRegisteredTask:
        def __init__(self, acl_name: str, rules: list[dict[str, object]]) -> None:
            self.acl_name = acl_name
            self.rules = rules

        def start(self, task, host, context):
            return TaskSuccessResult(
                changed=True,
                summary=f"{self.acl_name}:{len(self.rules)}",
            )

    task_definition = create_registered_task_by_identity(
        "acme.tests.kwargs_registered@1.0.0",
        {"acl_name": "edge-inbound", "rules": [{"action": "permit"}]},
    )

    result = task_definition.run_on_host(Host(hostname="router1"))
    assert result.to_dict()["hosts"]["router1"]["outcome"]["Passed"]["summary"] == (
        "edge-inbound:1"
    )


def test_registered_python_task_default_factory_rejects_input():
    @task(
        name="default_registered",
        registration=TaskRegistration(
            id="acme.tests.default_registered",
            version="1.0.0",
            factory=TaskFactory.DEFAULT,
        ),
    )
    class DefaultRegisteredTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="default")

    task_definition = create_registered_task_by_identity(
        "acme.tests.default_registered@1.0.0"
    )
    result = task_definition.run_on_host(Host(hostname="router1"))

    assert result.to_dict()["hosts"]["router1"]["outcome"]["Passed"]["summary"] == (
        "default"
    )
    with pytest.raises(
        TaskRegistrationError,
        match="default factory does not accept input",
    ):
        create_registered_task_by_identity(
            "acme.tests.default_registered@1.0.0",
            {"unexpected": True},
        )


def test_registered_python_task_custom_factory_errors_include_identity():
    def custom_factory(input):
        raise RuntimeError("boom")

    @task(
        name="custom_registered",
        registration=TaskRegistration(
            id="acme.tests.custom_registered",
            version="1.0.0",
            factory=CustomTaskFactory(callable=custom_factory),
        ),
    )
    class CustomRegisteredTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="custom")

    with pytest.raises(
        TaskRegistrationError,
        match="factory failed for registered task `acme.tests.custom_registered@1.0.0`: boom",
    ):
        create_registered_task_by_identity(
            "acme.tests.custom_registered@1.0.0",
            {"value": 1},
        )


def test_task_registration_rejects_invalid_metadata():
    with pytest.raises(ValidationError, match="invalid task id"):
        TaskRegistration(
            id="Acme.Tests.Invalid",
            version="1.0.0",
            factory=TaskFactory.DEFAULT,
        )

    with pytest.raises(ValidationError, match="invalid task version"):
        TaskRegistration(
            id="acme.tests.invalid_version",
            version="1.0",
            factory=TaskFactory.DEFAULT,
        )

    with pytest.raises(ValidationError, match="factory"):
        TaskRegistration(
            id="acme.tests.invalid_factory",
            version="1.0.0",
            factory="custom",
        )

    with pytest.raises(ValidationError, match="model"):
        PydanticInputSchema(model=object)

    with pytest.raises(
        TypeError, match="registration must be TaskRegistration or None"
    ):

        @task(
            name="plain_mapping_registration",
            registration={
                "id": "acme.tests.plain_mapping_registration",
                "version": "1.0.0",
            },
        )
        class PlainMappingRegistrationTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="plain mapping")


def test_registered_python_task_uses_pydantic_input_schema():
    class ConfigureAclInput(BaseModel):
        acl_name: str
        rules: list[dict[str, object]]

    @task(
        name="pydantic_schema_registered",
        registration=TaskRegistration(
            id="acme.tests.pydantic_schema_registered",
            version="1.0.0",
            factory=TaskFactory.KWARGS,
            input_schema=PydanticInputSchema(model=ConfigureAclInput),
        ),
    )
    class PydanticSchemaRegisteredTask:
        def __init__(self, acl_name: str, rules: list[dict[str, object]]) -> None:
            self.acl_name = acl_name
            self.rules = rules

        def start(self, task, host, context):
            return TaskSuccessResult(summary=self.acl_name)

    descriptor = get_registered_task_descriptor_by_identity(
        "acme.tests.pydantic_schema_registered@1.0.0"
    )
    schema = descriptor.to_dict()["input_schema"]

    assert schema["type"] == "object"
    assert schema["required"] == ["acl_name", "rules"]
    assert schema["properties"]["acl_name"]["type"] == "string"
    assert schema["properties"]["rules"]["type"] == "array"


def test_registered_python_task_lookup_reports_ambiguous_and_missing_tasks():
    @task(
        name="ambiguous_registered_v1",
        registration=TaskRegistration(
            id="acme.tests.ambiguous_registered",
            version="1.0.0",
            factory=TaskFactory.DEFAULT,
        ),
    )
    class AmbiguousRegisteredTaskV1:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="v1")

    @task(
        name="ambiguous_registered_v2",
        registration=TaskRegistration(
            id="acme.tests.ambiguous_registered",
            version="2.0.0",
            factory=TaskFactory.DEFAULT,
        ),
    )
    class AmbiguousRegisteredTaskV2:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="v2")

    with pytest.raises(
        TaskRegistrationError,
        match="registered task `acme.tests.ambiguous_registered` has multiple versions",
    ):
        get_registered_task_descriptor("acme.tests.ambiguous_registered")

    with pytest.raises(
        TaskRegistrationError,
        match="registered task `acme.tests.missing_registered` was not found",
    ):
        get_registered_task_descriptor("acme.tests.missing_registered")

    with pytest.raises(
        TaskRegistrationError,
        match="registered task `acme.tests.ambiguous_registered@3.0.0` was not found",
    ):
        get_registered_task_descriptor("acme.tests.ambiguous_registered", "3.0.0")


def test_registered_python_task_rejects_invalid_construction_input():
    @task(
        name="invalid_input_registered",
        registration=TaskRegistration(
            id="acme.tests.invalid_input_registered",
            version="1.0.0",
            factory=TaskFactory.KWARGS,
        ),
    )
    class InvalidInputRegisteredTask:
        def __init__(self, value: str) -> None:
            self.value = value

        def start(self, task, host, context):
            return TaskSuccessResult(summary=self.value)

    with pytest.raises(
        TaskRegistrationError,
        match="input must be a mapping",
    ):
        create_registered_task(
            "acme.tests.invalid_input_registered",
            ["not", "a", "mapping"],
            "1.0.0",
        )

    with pytest.raises(
        TaskRegistrationError,
        match="input must be JSON-serializable",
    ):
        create_registered_task_by_identity(
            "acme.tests.invalid_input_registered@1.0.0",
            {"value": object()},
        )


def test_task_definition_run_on_host_executes_python_body():
    task_definition = genja.TaskDefinition.from_python_class(BackupConfigPlainTask)

    result = task_definition.run_on_host(Host(hostname="router1", platform="ios"))
    data = result.to_dict()

    assert result.passed_hosts == ["router1"]
    assert (
        data["hosts"]["router1"]["outcome"]["Passed"]["summary"] == "backed up router1"
    )
    assert (
        data["hosts"]["router1"]["outcome"]["Passed"]["metadata"]["sub_task_name"]
        == "verify_backup_plain"
    )
    assert (
        data["hosts"]["router1"]["outcome"]["Passed"]["metadata"]["backup_path"]
        == "/tmp/configs"
    )


def test_python_backed_task_applies_retry_delay():
    attempts: list[float] = []
    current_attempts: list[int] = []

    @task(
        name="delayed_retry",
        retry=RetryConfig(allow=True, max_attempts=2, delay_ms=50),
    )
    class DelayedRetryTask:
        def start(self, task, host, context):
            attempts.append(time.monotonic())
            current_attempts.append(context.current_attempt)
            if len(attempts) == 1:
                return TaskFailureResult(
                    message=f"temporary failure on {host.hostname}",
                    kind=TaskFailureKind.EXTERNAL,
                    retryable=True,
                )
            return TaskSuccessResult(summary=f"retried {host.hostname}")

    task_definition = genja.TaskDefinition.from_python_class(DelayedRetryTask)
    result = task_definition.run_on_host(Host(hostname="router1"))

    assert result.passed_hosts == ["router1"]
    assert len(attempts) == 2
    assert current_attempts == [1, 2]
    assert attempts[1] - attempts[0] >= 0.04
    host_result = result.to_dict()["hosts"]["router1"]
    assert host_result["execution_metadata"]["attempts"] == 2
    assert host_result["execution_metadata"]["retried"] is True


def test_python_backed_task_dry_run_calls_dry_run_not_start():
    calls: list[str] = []

    @task(name="preview_backup", supports_dry_run=True)
    class PreviewBackupTask:
        def start(self, task, host, context):
            calls.append("start")
            return TaskSuccessResult(summary="started")

        def dry_run(self, task, host, context):
            calls.append("dry_run")
            assert task.supports_dry_run is True
            assert context.dry_run is True
            return TaskSuccessResult(
                changed=True,
                diff="- old\n+ new",
                summary=f"would update {host.hostname}",
            )

    task_definition = genja.TaskDefinition.from_python_class(PreviewBackupTask)
    result = task_definition.run_on_host(
        Host(hostname="router1"),
        run_options=genja.TaskRunOptions(dry_run=True),
    )

    assert task_definition.supports_dry_run is True
    assert calls == ["dry_run"]
    assert result.passed_hosts == ["router1"]
    host_result = result.to_dict()["hosts"]["router1"]
    assert host_result["outcome"]["Passed"]["changed"] is True
    assert host_result["execution_metadata"]["dry_run"] is True


def test_python_backed_task_can_return_passed_with_warnings():
    @task(name="warning_success")
    class WarningSuccessTask:
        def start(self, task, host, context):
            return TaskSuccessResult(
                status=TaskStatus.PASSED_WITH_WARNINGS,
                summary="state appears converged",
                warnings=["previous attempt may have skipped finalization"],
            )

    task_definition = genja.TaskDefinition.from_python_class(WarningSuccessTask)
    result = task_definition.run_on_host(Host(hostname="router1"))

    assert result.passed_hosts == ["router1"]
    host_result = result.to_dict()["hosts"]["router1"]
    assert host_result["outcome"]["PassedWithWarnings"]["summary"] == (
        "state appears converged"
    )
    assert host_result["outcome"]["PassedWithWarnings"]["warnings"] == [
        "previous attempt may have skipped finalization"
    ]


def test_idempotency_mode_and_check_result_are_rust_backed_exports():
    assert IdempotencyMode.CHECK.value == "check"
    assert str(IdempotencyMode.CHECK_AND_VERIFY) == "check_and_verify"
    assert repr(IdempotencyMode.DISABLED) == "IdempotencyMode.DISABLED"
    assert genja.IdempotencyMode.CHECK == IdempotencyMode.CHECK

    converged = IdempotencyCheckResult.converged(
        summary="already configured",
        details={"current": "desired"},
    )
    assert converged.status == "converged"
    assert converged.summary == "already configured"
    assert converged.diff is None
    assert converged.details == {"current": "desired"}
    assert converged.to_dict() == {
        "status": "converged",
        "summary": "already configured",
        "diff": None,
        "details": {"current": "desired"},
    }

    change_required = IdempotencyCheckResult.change_required(diff="+configured")
    assert change_required.status == "change_required"
    assert change_required.summary is None
    assert change_required.diff == "+configured"
    assert change_required.details is None


def test_python_backed_idempotent_task_converged_check_skips_start():
    calls: list[str] = []

    @task(name="idempotent_converged", idempotency=IdempotencyMode.CHECK)
    class IdempotentConvergedTask:
        def check(self, task, host, context):
            calls.append("check")
            assert task.idempotency == IdempotencyMode.CHECK
            assert task.to_dict()["idempotency"] == "check"
            return IdempotencyCheckResult.converged(
                summary=f"{host.hostname} already configured",
                details={"current": "desired"},
            )

        def start(self, task, host, context):
            calls.append("start")
            return TaskSuccessResult(changed=True, summary="started")

    task_definition = genja.TaskDefinition.from_python_class(IdempotentConvergedTask)
    result = task_definition.run_on_host(Host(hostname="router1"))
    host_result = result.to_dict()["hosts"]["router1"]

    assert task_definition.idempotency == IdempotencyMode.CHECK
    assert task_definition.to_dict()["idempotency"] == "check"
    assert calls == ["check"]
    assert host_result["outcome"]["Passed"]["changed"] is False
    assert host_result["outcome"]["Passed"]["summary"] == "router1 already configured"
    assert host_result["outcome"]["Passed"]["metadata"] == {
        "idempotency": {
            "state": "converged",
            "details": {"current": "desired"},
        }
    }


def test_python_backed_idempotent_task_change_required_invokes_start():
    calls: list[str] = []

    @task(name="idempotent_change", idempotency=IdempotencyMode.CHECK)
    class IdempotentChangeTask:
        def check(self, task, host, context):
            calls.append("check")
            return IdempotencyCheckResult.change_required(diff="+configured")

        def start(self, task, host, context):
            calls.append("start")
            return TaskSuccessResult(changed=True, summary="applied")

    task_definition = genja.TaskDefinition.from_python_class(IdempotentChangeTask)
    result = task_definition.run_on_host(Host(hostname="router1"))

    assert calls == ["check", "start"]
    assert (
        result.to_dict()["hosts"]["router1"]["outcome"]["Passed"]["summary"]
        == "applied"
    )


def test_python_backed_check_and_verify_preserves_applied_result_when_converged():
    calls: list[str] = []

    @task(
        name="idempotent_verified",
        idempotency=IdempotencyMode.CHECK_AND_VERIFY,
    )
    class IdempotentVerifiedTask:
        def check(self, task, host, context):
            calls.append("check")
            if calls.count("check") == 1:
                return IdempotencyCheckResult.change_required(diff="+configured")
            return IdempotencyCheckResult.converged(summary="now converged")

        def start(self, task, host, context):
            calls.append("start")
            return TaskSuccessResult(changed=True, summary="applied")

    task_definition = genja.TaskDefinition.from_python_class(IdempotentVerifiedTask)
    result = task_definition.run_on_host(Host(hostname="router1"))
    host_result = result.to_dict()["hosts"]["router1"]

    assert calls == ["check", "start", "check"]
    assert result.passed_hosts == ["router1"]
    assert host_result["outcome"]["Passed"]["changed"] is True
    assert host_result["outcome"]["Passed"]["summary"] == "applied"


def test_python_backed_check_and_verify_fails_when_post_check_requires_change():
    calls: list[str] = []

    @task(
        name="idempotent_not_verified",
        idempotency=IdempotencyMode.CHECK_AND_VERIFY,
    )
    class IdempotentNotVerifiedTask:
        def check(self, task, host, context):
            calls.append("check")
            return IdempotencyCheckResult.change_required(
                diff="+still missing",
                details={"remaining": "ntp"},
            )

        def start(self, task, host, context):
            calls.append("start")
            return TaskSuccessResult(changed=True, summary="applied")

    task_definition = genja.TaskDefinition.from_python_class(IdempotentNotVerifiedTask)
    result = task_definition.run_on_host(Host(hostname="router1"))
    host_result = result.to_dict()["hosts"]["router1"]

    assert calls == ["check", "start", "check"]
    assert result.failed_hosts == ["router1"]
    failure = host_result["outcome"]["Failed"]
    assert failure["kind"] == "Validation"
    assert failure["message"] == "Configuration did not converge after application"
    assert failure["details"] == {
        "application_completed": True,
        "configuration_may_have_changed": True,
        "remaining_diff": "+still missing",
        "idempotency": {
            "state": "change_required",
            "details": {"remaining": "ntp"},
        },
    }


def test_python_backed_idempotent_pre_check_exception_fails_only_affected_host():
    calls: list[str] = []

    @task(name="idempotent_check_exception", idempotency=IdempotencyMode.CHECK)
    class IdempotentCheckExceptionTask:
        def check(self, task, host, context):
            calls.append(f"check:{host.hostname}")
            if host.hostname == "10.0.0.1":
                raise ValueError("inspection failed")
            return IdempotencyCheckResult.converged(summary="already configured")

        def start(self, task, host, context):
            calls.append(f"start:{host.hostname}")
            return TaskSuccessResult(changed=True, summary="started")

    runtime = genja.Genja.from_hosts({
        "router1": Host(hostname="10.0.0.1"),
        "router2": Host(hostname="10.0.0.2"),
    }).with_runner("serial")
    result = runtime.run_task(IdempotentCheckExceptionTask)
    data = result.to_dict()

    assert calls == ["check:10.0.0.1", "check:10.0.0.2"]
    assert result.failed_hosts == ["router1"]
    assert result.passed_hosts == ["router2"]
    assert (
        "inspection failed" in data["hosts"]["router1"]["outcome"]["Failed"]["message"]
    )
    assert data["hosts"]["router2"]["outcome"]["Passed"]["changed"] is False


def test_python_backed_idempotent_task_dry_run_does_not_call_check():
    calls: list[str] = []

    @task(
        name="idempotent_dry_run",
        idempotency=IdempotencyMode.CHECK,
        supports_dry_run=True,
    )
    class IdempotentDryRunTask:
        def check(self, task, host, context):
            calls.append("check")
            return IdempotencyCheckResult.converged()

        def start(self, task, host, context):
            calls.append("start")
            return TaskSuccessResult(changed=True, summary="started")

        def dry_run(self, task, host, context):
            calls.append("dry_run")
            return TaskSuccessResult(changed=True, summary="would change")

    task_definition = genja.TaskDefinition.from_python_class(IdempotentDryRunTask)
    result = task_definition.run_on_host(
        Host(hostname="router1"),
        run_options=genja.TaskRunOptions(dry_run=True),
    )

    assert calls == ["dry_run"]
    assert result.to_dict()["hosts"]["router1"]["execution_metadata"]["dry_run"] is True


def test_python_backed_task_dry_run_fails_unsupported_without_start():
    calls: list[str] = []

    @task(name="unsupported_preview")
    class UnsupportedPreviewTask:
        def start(self, task, host, context):
            calls.append("start")
            return TaskSuccessResult(summary="started")

    task_definition = genja.TaskDefinition.from_python_class(UnsupportedPreviewTask)
    result = task_definition.run_on_host(
        Host(hostname="router1"),
        run_options=genja.TaskRunOptions(dry_run=True),
    )

    assert task_definition.supports_dry_run is False
    assert calls == []
    host_result = result.to_dict()["hosts"]["router1"]
    assert "does not support dry-run" in host_result["outcome"]["Failed"]["message"]
    assert host_result["execution_metadata"]["dry_run"] is True


def test_task_decorator_requires_dry_run_method_when_supported():
    with pytest.raises(
        TypeError,
        match=(
            "is a sync task with supports_dry_run=True.*"
            r"dry_run\(self, task, host, context\)"
        ),
    ):

        @task(name="missing_preview", supports_dry_run=True)
        class MissingDryRunTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="started")


def test_task_decorator_requires_async_dry_run_method_when_supported():
    with pytest.raises(
        TypeError,
        match=(
            "is an async task with supports_dry_run=True.*"
            r"dry_run_async\(self, task, host, context\)"
        ),
    ):

        @task(name="missing_async_preview", supports_dry_run=True)
        class MissingAsyncDryRunTask:
            async def start_async(self, task, host, context):
                return TaskSuccessResult(summary="started")


def test_task_decorator_requires_idempotency_mode_enum():
    with pytest.raises(TypeError, match="idempotency must be IdempotencyMode"):

        @task(name="bad_idempotency", idempotency="check")
        class BadIdempotencyTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="started")


def test_task_decorator_requires_check_method_when_idempotency_enabled():
    with pytest.raises(
        TypeError,
        match=(
            "is a sync task with idempotency enabled.*"
            r"check\(self, task, host, context\)"
        ),
    ):

        @task(name="missing_check", idempotency=IdempotencyMode.CHECK)
        class MissingCheckTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="started")


def test_task_decorator_requires_async_check_method_when_idempotency_enabled():
    with pytest.raises(
        TypeError,
        match=(
            "is an async task with idempotency enabled.*"
            r"check_async\(self, task, host, context\)"
        ),
    ):

        @task(name="missing_async_check", idempotency=IdempotencyMode.CHECK)
        class MissingAsyncCheckTask:
            async def start_async(self, task, host, context):
                return TaskSuccessResult(summary="started")


def test_task_decorator_rejects_wrong_check_method_for_sync_task():
    with pytest.raises(TypeError, match="sync task.*not 'check_async'"):

        @task(name="wrong_sync_check", idempotency=IdempotencyMode.CHECK)
        class WrongSyncCheckTask:
            async def check_async(self, task, host, context):
                return IdempotencyCheckResult.converged()

            def start(self, task, host, context):
                return TaskSuccessResult(summary="started")


def test_task_decorator_rejects_wrong_check_method_for_async_task():
    with pytest.raises(TypeError, match="async task.*not 'check'"):

        @task(name="wrong_async_check", idempotency=IdempotencyMode.CHECK)
        class WrongAsyncCheckTask:
            def check(self, task, host, context):
                return IdempotencyCheckResult.converged()

            async def start_async(self, task, host, context):
                return TaskSuccessResult(summary="started")


def test_task_decorator_rejects_non_callable_check_method():
    with pytest.raises(TypeError, match="attribute 'check' must be callable"):

        @task(name="non_callable_check", idempotency=IdempotencyMode.CHECK)
        class NonCallableCheckTask:
            check = "not callable"

            def start(self, task, host, context):
                return TaskSuccessResult(summary="started")


def test_task_definition_from_python_class_requires_decorator_metadata():
    class MissingMetadataTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    with pytest.raises(ValueError, match="missing __genja_task_info__"):
        genja.TaskDefinition.from_python_class(MissingMetadataTask)


def test_task_definition_from_python_class_allows_missing_connection_plugin_name():
    @task(name="backup_config")
    class NoConnectionTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    task_definition = genja.TaskDefinition.from_python_class(NoConnectionTask)

    assert task_definition.connection_plugin_name is None
    assert task_definition.to_dict()["connection_plugin_name"] is None


def test_retry_config_accepts_valid_values():
    retry = RetryConfig(allow=True, max_attempts=3, delay_ms=500)

    assert retry.to_dict() == {
        "allow": True,
        "max_attempts": 3,
        "delay_ms": 500,
    }


def test_retry_config_rejects_invalid_values():
    with pytest.raises(ValidationError, match="max_attempts"):
        RetryConfig(max_attempts=0)

    with pytest.raises(ValidationError, match="delay_ms"):
        RetryConfig(delay_ms=-1)


def test_task_decorator_stores_nested_retry_metadata():
    @task(
        name="retryable_backup",
        connection_plugin_name="ssh",
        retry=RetryConfig(allow=True, max_attempts=3, delay_ms=500),
    )
    class RetryableTask:
        def start(self, task, host, context):
            assert task.retry == RetryConfig(allow=True, max_attempts=3, delay_ms=500)
            return TaskSuccessResult(summary="noop")

    metadata = cast(type[GenjaTaskProtocol], RetryableTask).__genja_task_info__
    assert metadata["retry"] == {
        "allow": True,
        "max_attempts": 3,
        "delay_ms": 500,
    }

    task_definition = genja.TaskDefinition.from_python_class(RetryableTask)
    assert task_definition.retry == {
        "allow": True,
        "max_attempts": 3,
        "delay_ms": 500,
    }
    assert task_definition.to_dict()["retry"] == {
        "allow": True,
        "max_attempts": 3,
        "delay_ms": 500,
    }


def test_session_verification_config_accepts_valid_values():
    config = SessionVerificationConfig(max_attempts=3, delay_ms=500)

    assert config.max_attempts == 3
    assert config.delay_ms == 500
    assert config.to_dict() == {
        "max_attempts": 3,
        "delay_ms": 500,
    }
    assert SessionVerificationConfig().to_dict() == {
        "max_attempts": 1,
        "delay_ms": 0,
    }


def test_session_verification_config_rejects_invalid_values():
    with pytest.raises(ValueError, match="max_attempts must be greater than 0"):
        SessionVerificationConfig(max_attempts=0)

    with pytest.raises(ValueError, match="delay_ms must be greater than or equal to 0"):
        SessionVerificationConfig(delay_ms=-1)

    with pytest.raises(ValueError, match="max_attempts must be an integer"):
        SessionVerificationConfig(max_attempts=True)

    with pytest.raises(TypeError, match="unexpected keyword argument"):
        SessionVerificationConfig(attempts=1)


def test_task_decorator_stores_session_verification_metadata():
    @task(
        name="verified_backup",
        connection_plugin_name="ssh",
        session_verification=SessionVerificationConfig(max_attempts=3, delay_ms=500),
    )
    class SessionVerifiedTask:
        def start(self, task, host, context):
            assert task.session_verification == SessionVerificationConfig(
                max_attempts=3,
                delay_ms=500,
            )
            return TaskSuccessResult(summary="noop")

    metadata = cast(type[GenjaTaskProtocol], SessionVerifiedTask).__genja_task_info__
    assert metadata["session_verification"] == SessionVerificationConfig(
        max_attempts=3,
        delay_ms=500,
    )

    task_definition = genja.TaskDefinition.from_python_class(SessionVerifiedTask)
    assert task_definition.session_verification == SessionVerificationConfig(
        max_attempts=3,
        delay_ms=500,
    )
    assert task_definition.session_verification.to_dict() == {
        "max_attempts": 3,
        "delay_ms": 500,
    }
    assert task_definition.to_dict()["session_verification"] == {
        "max_attempts": 3,
        "delay_ms": 500,
    }


def test_task_decorator_rejects_invalid_session_verification_type():
    with pytest.raises(
        TypeError,
        match="session_verification must be SessionVerificationConfig or None",
    ):

        @task(
            name="backup_config",
            connection_plugin_name="ssh",
            session_verification={"max_attempts": 1},
        )
        class InvalidTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_decorator_rejects_session_verification_without_connection():
    with pytest.raises(
        TypeError,
        match="session_verification requires connection_plugin_name",
    ):

        @task(
            name="backup_config",
            session_verification=SessionVerificationConfig(),
        )
        class InvalidTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_definition_rejects_invalid_session_verification_metadata():
    @task(
        name="backup_config",
        connection_plugin_name="ssh",
        session_verification=SessionVerificationConfig(),
    )
    class InvalidTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    metadata = cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__
    metadata["session_verification"] = {"max_attempts": 1, "delay_ms": 0}

    with pytest.raises(
        ValueError,
        match="session_verification.*must be SessionVerificationConfig or None",
    ):
        genja.TaskDefinition.from_python_class(InvalidTask)


def test_task_definition_rejects_session_verification_without_connection_in_metadata():
    @task(
        name="backup_config",
        connection_plugin_name="ssh",
        session_verification=SessionVerificationConfig(),
    )
    class InvalidTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    metadata = cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__
    metadata["connection_plugin_name"] = None

    with pytest.raises(
        ValueError,
        match="session_verification.*requires 'connection_plugin_name'",
    ):
        genja.TaskDefinition.from_python_class(InvalidTask)


def test_task_decorator_rejects_flat_retry_kwargs():
    with pytest.raises(TypeError, match=r"did you mean retry=RetryConfig\(allow="):

        @task(name="backup_config", allow_retries=True)
        class InvalidAllowTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")

    with pytest.raises(
        TypeError, match=r"did you mean retry=RetryConfig\(max_attempts="
    ):

        @task(name="backup_config", max_task_attempts=3)
        class InvalidMaxAttemptsTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")

    with pytest.raises(TypeError, match=r"did you mean retry=RetryConfig\(delay_ms="):

        @task(name="backup_config", delay_ms=500)
        class InvalidDelayTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_definition_rejects_invalid_retry_metadata():
    @task(name="backup_config", connection_plugin_name="ssh")
    class InvalidTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    metadata = cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__
    metadata["retry"] = {"max_attempts": 0}

    with pytest.raises(ValueError, match=r"retry\.max_attempts.*at least 1"):
        genja.TaskDefinition.from_python_class(InvalidTask)


def test_task_decorator_rejects_empty_connection_plugin_name():
    with pytest.raises(
        TypeError, match="connection_plugin_name must be a non-empty string or None"
    ):

        @task(name="backup_config", connection_plugin_name="")
        class InvalidTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_decorator_rejects_empty_name():
    with pytest.raises(TypeError, match="name must be a non-empty string"):

        @task(name="   ", connection_plugin_name="ssh")
        class InvalidTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_decorator_rejects_non_json_serializable_options():
    with pytest.raises(TypeError, match="options must be JSON-serializable"):

        @task(
            name="backup_config",
            connection_plugin_name="ssh",
            options={"callback": lambda: None},
        )
        class InvalidTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_definition_from_python_class_rejects_empty_connection_plugin_name_in_metadata():
    @task(name="backup_config", connection_plugin_name="ssh")
    class InvalidTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__[
        "connection_plugin_name"
    ] = ""

    with pytest.raises(ValueError, match="connection_plugin_name.*must not be empty"):
        genja.TaskDefinition.from_python_class(InvalidTask)


def test_task_definition_from_python_class_rejects_empty_name_in_metadata():
    @task(name="backup_config", connection_plugin_name="ssh")
    class InvalidTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__["name"] = "   "

    with pytest.raises(ValueError, match="field 'name' must not be empty"):
        genja.TaskDefinition.from_python_class(InvalidTask)


def test_task_definition_from_python_class_rejects_non_json_serializable_options_in_metadata():
    @task(name="backup_config", connection_plugin_name="ssh")
    class InvalidTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    cast(type[GenjaTaskProtocol], InvalidTask).__genja_task_info__["options"] = {
        "callback": lambda: None
    }

    with pytest.raises(TypeError, match="not JSON serializable"):
        genja.TaskDefinition.from_python_class(InvalidTask)


def test_task_decorator_requires_exactly_one_entrypoint():
    with pytest.raises(
        TypeError, match="must define exactly one of 'start' or 'start_async'"
    ):

        @task(name="backup_config", connection_plugin_name="ssh")
        class InvalidTask:
            pass


def test_task_decorator_rejects_both_entrypoints():
    with pytest.raises(
        TypeError, match="must define exactly one of 'start' or 'start_async'"
    ):

        @task(name="backup_config", connection_plugin_name="ssh")
        class InvalidTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")

            async def start_async(self, task, host, context):
                return TaskSuccessResult(summary="noop")


def test_task_decorator_rejects_undecorated_sub_task():
    class PlainSubTask:
        def start(self, task, host, context):
            return TaskSuccessResult(summary="noop")

    with pytest.raises(TypeError, match="must also be decorated with @task"):

        @task(
            name="backup_config",
            connection_plugin_name="ssh",
            sub_tasks=[PlainSubTask],
        )
        class InvalidTask:
            def start(self, task, host, context):
                return TaskSuccessResult(summary="noop")
