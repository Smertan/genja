import copy
from datetime import datetime, timezone
import importlib

import genja
from pydantic import ValidationError
import pytest
from genja.task import (
    TaskFailureKind,
    TaskFailureResult,
    TaskMessage,
    TaskMessageLevel,
    TaskSkipResult,
    TaskSuccessResult,
)

genja_task = importlib.import_module("genja.task")


def test_task_result_enums_are_rust_backed_exports():
    assert genja.TaskFailureKind is genja_task.TaskFailureKind
    assert genja.TaskMessageLevel is genja_task.TaskMessageLevel
    assert genja.TaskFailureKind.CONNECTION == TaskFailureKind.CONNECTION
    assert genja.TaskMessageLevel.WARNING == TaskMessageLevel.WARNING

    assert TaskFailureKind.CONNECTION.value == "connection"
    assert TaskMessageLevel.WARNING.value == "warning"
    assert str(TaskFailureKind.CONNECTION) == "connection"
    assert str(TaskMessageLevel.WARNING) == "warning"
    assert repr(TaskFailureKind.CONNECTION) == "TaskFailureKind.CONNECTION"
    assert repr(TaskMessageLevel.WARNING) == "TaskMessageLevel.WARNING"

    assert copy.copy(TaskFailureKind.EXTERNAL) == TaskFailureKind.EXTERNAL
    assert copy.deepcopy(TaskFailureKind.EXTERNAL) == TaskFailureKind.EXTERNAL
    assert copy.copy(TaskMessageLevel.INFO) == TaskMessageLevel.INFO
    assert copy.deepcopy(TaskMessageLevel.INFO) == TaskMessageLevel.INFO


def test_host_task_result_from_python_success_result_round_trips():
    result = TaskSuccessResult(
        changed=True,
        summary="backup complete",
        warnings=["using fallback path"],
        messages=[
            TaskMessage(
                level=TaskMessageLevel.INFO,
                text="backup complete",
                code="BACKUP_DONE",
                timestamp=datetime(2026, 4, 29, 12, 0, tzinfo=timezone.utc),
            )
        ],
        metadata={"backup_file": "/tmp/router1.cfg"},
    )

    host_result = genja.HostTaskResult.from_python_result(result)
    data = host_result.to_dict()

    assert host_result.status == "passed"
    assert "Passed" in data["outcome"]
    assert data["outcome"]["Passed"]["changed"] is True
    assert data["outcome"]["Passed"]["summary"] == "backup complete"
    assert data["outcome"]["Passed"]["warnings"] == ["using fallback path"]
    assert data["outcome"]["Passed"]["messages"][0]["level"] == "Info"
    assert data["outcome"]["Passed"]["messages"][0]["text"] == "backup complete"
    assert data["outcome"]["Passed"]["messages"][0]["code"] == "BACKUP_DONE"
    assert data["outcome"]["Passed"]["metadata"]["backup_file"] == "/tmp/router1.cfg"
    assert data["execution_metadata"]["attempts"] == 1


def test_host_task_result_from_python_failure_result_round_trips():
    result = TaskFailureResult(
        message="connection timeout",
        kind=TaskFailureKind.TIMEOUT,
        retryable=True,
        details={"timeout_seconds": 30},
        warnings=["slow link detected"],
        messages=[TaskMessage(level=TaskMessageLevel.ERROR, text="failed to connect")],
    )

    host_result = genja.HostTaskResult.from_python_result(result)
    data = host_result.to_dict()

    assert host_result.status == "failed"
    assert data["outcome"]["Failed"]["kind"] == "Timeout"
    assert data["outcome"]["Failed"]["message"] == "connection timeout"
    assert data["outcome"]["Failed"]["retryable"] is True
    assert data["outcome"]["Failed"]["details"]["timeout_seconds"] == 30


def test_task_success_result_rejects_non_json_serializable_metadata():
    with pytest.raises(TypeError, match="metadata must be JSON-serializable"):
        TaskSuccessResult(metadata={"callback": lambda: None})


def test_task_success_result_supports_dict_style_access_and_missing_keys_raise_key_error():
    result = TaskSuccessResult(summary="backup complete")

    assert result["summary"] == "backup complete"
    with pytest.raises(KeyError, match="missing_field"):
        result["missing_field"]


def test_task_failure_result_rejects_non_json_serializable_details():
    with pytest.raises(TypeError, match="details must be JSON-serializable"):
        TaskFailureResult(message="boom", details={"callback": lambda: None})


def test_task_message_rejects_invalid_message_level():
    with pytest.raises(ValidationError):
        TaskMessage(level="verbose", text="unexpected level")


def test_task_message_rejects_raw_message_level_value():
    with pytest.raises(ValidationError):
        TaskMessage(level="info", text="unexpected level")


def test_task_failure_result_rejects_invalid_failure_kind():
    with pytest.raises(ValidationError):
        TaskFailureResult(message="boom", kind="network")


def test_task_failure_result_rejects_raw_failure_kind_value():
    with pytest.raises(ValidationError):
        TaskFailureResult(message="boom", kind="external")


def test_task_success_result_rejects_invalid_status_literal():
    with pytest.raises(ValidationError):
        TaskSuccessResult(status="failed")


def test_task_failure_result_rejects_invalid_status_literal():
    with pytest.raises(ValidationError):
        TaskFailureResult(message="boom", status="passed")


def test_task_skip_result_rejects_invalid_status_literal():
    with pytest.raises(ValidationError):
        TaskSkipResult(status="failed")


def test_host_task_result_from_python_skip_result_round_trips():
    result = TaskSkipResult(
        reason="maintenance_mode",
        message="host is in maintenance mode",
    )

    host_result = genja.HostTaskResult.from_python_result(result)
    data = host_result.to_dict()

    assert host_result.status == "skipped"
    assert data["outcome"]["Skipped"]["reason"] == "maintenance_mode"
    assert data["outcome"]["Skipped"]["message"] == "host is in maintenance mode"


def test_host_task_result_from_python_result_rejects_missing_status():
    with pytest.raises(ValueError, match="missing 'status'"):
        genja.HostTaskResult.from_python_result({"summary": "backup complete"})


def test_host_task_result_from_python_result_rejects_unknown_status():
    with pytest.raises(
        ValueError, match="unsupported python task result status 'unknown'"
    ):
        genja.HostTaskResult.from_python_result({"status": "unknown"})
