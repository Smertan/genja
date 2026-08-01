import time
from typing import cast

import genja
import pytest
from genja.task import (
    Host,
    GenjaTaskProtocol,
    IdempotencyCheckResult,
    IdempotencyMode,
    RetryConfig,
    TaskFailureResult,
    TaskRuntimeContext,
    TaskMessageLevel,
    TaskInfo,
    TaskMessage,
    TaskStatus,
    TaskSuccessResult,
    task,
)
from pydantic import ValidationError


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
                    kind="external",
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
