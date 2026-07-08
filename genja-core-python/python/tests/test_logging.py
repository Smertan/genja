import logging

import genja


def test_rust_log_records_are_forwarded_to_python_logging(caplog, monkeypatch):
    monkeypatch.setenv("GENJA_LOGGING_TO_CONSOLE", "not-a-bool")

    with caplog.at_level(logging.WARNING):
        genja.Settings()

    assert any(
        record.levelno == logging.WARNING
        and record.name == "genja_core.settings.env_defaults"
        and 'Invalid GENJA_LOGGING_TO_CONSOLE value "not-a-bool"; using default false'
        in record.message
        for record in caplog.records
    )
