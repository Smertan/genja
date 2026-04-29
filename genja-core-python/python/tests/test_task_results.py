from datetime import datetime, timezone

import genja_core
from genja_core.task import (
    TaskFailureResult,
    TaskMessage,
    TaskSkipResult,
    TaskSuccessResult,
)


def test_host_task_result_from_python_success_result_round_trips():
    result = TaskSuccessResult(
        changed=True,
        summary="backup complete",
        warnings=["using fallback path"],
        messages=[
            TaskMessage(
                level="info",
                text="backup complete",
                code="BACKUP_DONE",
                timestamp=datetime(2026, 4, 29, 12, 0, tzinfo=timezone.utc),
            )
        ],
        metadata={"backup_file": "/tmp/router1.cfg"},
    )

    host_result = genja_core.HostTaskResult.from_python_result(result)
    data = host_result.to_dict()

    assert host_result.status == "passed"
    assert data["status"] == "passed"
    assert data["changed"] is True
    assert data["summary"] == "backup complete"
    assert data["warnings"] == ["using fallback path"]
    assert data["messages"][0]["level"] == "info"
    assert data["messages"][0]["text"] == "backup complete"
    assert data["messages"][0]["code"] == "BACKUP_DONE"
    assert data["metadata"]["backup_file"] == "/tmp/router1.cfg"


def test_host_task_result_from_python_failure_result_round_trips():
    result = TaskFailureResult(
        message="connection timeout",
        kind="timeout",
        retryable=True,
        details={"timeout_seconds": 30},
        warnings=["slow link detected"],
        messages=[TaskMessage(level="error", text="failed to connect")],
    )

    host_result = genja_core.HostTaskResult.from_python_result(result)
    data = host_result.to_dict()

    assert host_result.status == "failed"
    assert data["kind"] == "timeout"
    assert data["message"] == "connection timeout"
    assert data["retryable"] is True
    assert data["details"]["timeout_seconds"] == 30


def test_host_task_result_from_python_skip_result_round_trips():
    result = TaskSkipResult(
        reason="maintenance_mode",
        message="host is in maintenance mode",
    )

    host_result = genja_core.HostTaskResult.from_python_result(result)
    data = host_result.to_dict()

    assert host_result.status == "skipped"
    assert data["reason"] == "maintenance_mode"
    assert data["message"] == "host is in maintenance mode"
