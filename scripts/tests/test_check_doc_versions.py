"""Behavior tests for the documentation package-version checker."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import pytest


CHECKER = Path(__file__).resolve().parents[1] / "check-doc-versions.py"


def write(root: Path, name: str, content: str) -> None:
    path = root / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8")


def run_checker(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(CHECKER), "--root", str(root), *args],
        text=True,
        capture_output=True,
        check=False,
    )


def test_current_package_versions_pass_without_flagging_task_data(
    tmp_path: Path,
) -> None:
    write(
        tmp_path,
        "docs/guide.md",
        'genja = "0.5.0"\n'
        'python -m pip install "genja-py[cli]>=0.5.0,<0.6.0"\n'
        "genja task describe example.hello@0.5.0\n"
        'task version="0.5.0"\n'
        "hostname: 10.0.0.1\n",
    )

    result = run_checker(tmp_path, "--version", "0.5.0")

    assert result.returncode == 0, result.stderr
    assert "documentation package versions valid" in result.stdout


def test_stale_versions_report_file_line_found_and_expected(tmp_path: Path) -> None:
    write(tmp_path, "docs/guide.md", "# Install\n\ncargo add genja@0.5.0\n")
    write(tmp_path, "README.md", "The Genja v0.5.0 release is available.\n")
    write(tmp_path, "genja-core/README.md", 'genja-core = "0.5.0"\n')
    write(tmp_path, "genja/examples/python/README.md", 'genja = "0.5.0"\n')

    result = run_checker(tmp_path, "--version", "0.6.0")

    assert result.returncode == 1
    assert "docs/guide.md:3: found 0.5.0; expected 0.6.0" in result.stderr
    assert "README.md:1: found 0.5.0; expected 0.6.0" in result.stderr
    assert "genja-core/README.md:1: found 0.5.0; expected 0.6.0" in result.stderr
    assert "genja/examples/python/README.md" not in result.stderr


def test_minor_range_stays_valid_for_patch_release(tmp_path: Path) -> None:
    write(
        tmp_path,
        "docs/guide.md",
        'python -m pip install "genja-py[cli]>=0.5.0,<0.6.0"\n',
    )

    assert run_checker(tmp_path, "--version", "0.5.1").returncode == 0
    result = run_checker(tmp_path, "--version", "0.6.0")
    assert result.returncode == 1
    assert "found >=0.5.0,<0.6.0; expected >=0.6.0,<0.7.0" in result.stderr


def test_pip_pin_and_cargo_version_operator_are_checked(tmp_path: Path) -> None:
    write(
        tmp_path,
        "docs/guide.md",
        'python -m pip install "genja-py==0.5.0"\ngenja = { version = "^0.5.0" }\n',
    )

    result = run_checker(tmp_path, "--version", "0.6.0")

    assert result.returncode == 1
    assert "docs/guide.md:1: found 0.5.0; expected 0.6.0" in result.stderr
    assert "docs/guide.md:2: found 0.5.0; expected 0.6.0" in result.stderr


def test_reasoned_exception_applies_to_one_occurrence_in_next_block(
    tmp_path: Path,
) -> None:
    write(
        tmp_path,
        "docs/guide.md",
        '<!-- genja-version-check: allow-next 0.5.0 reason="First CLI release" -->\n'
        "```bash\n"
        "cargo add genja@0.5.0\n"
        "cargo add genja@0.5.0\n"
        "```\n",
    )

    result = run_checker(tmp_path, "--version", "0.6.0")

    assert result.returncode == 1
    assert "docs/guide.md:3:" not in result.stderr
    assert "docs/guide.md:4: found 0.5.0; expected 0.6.0" in result.stderr


@pytest.mark.parametrize(
    ("content", "diagnostic"),
    [
        (
            '<!-- genja-version-check: allow-next 0.5.0 reason="" -->\n',
            "invalid version exception",
        ),
        (
            '<!-- genja-version-check: allow-next 0.5.0 reason="   " -->\n',
            "invalid version exception",
        ),
        (
            '<!-- genja-version-check: allow-next 0.4.0 reason="Historical" -->\n'
            'genja = "0.5.0"\n',
            "does not match 0.5.0",
        ),
        (
            '<!-- genja-version-check: allow-next 0.5.0 reason="Historical" -->\n'
            "example.hello@1.0.0\n",
            "unused version exception",
        ),
        (
            '<!-- genja-version-check: allow-next 0.5.0 reason="Historical" -->\n'
            "\n\n\n"
            'genja = "0.5.0"\n',
            "unused version exception",
        ),
    ],
)
def test_invalid_mismatched_and_unused_exceptions_fail(
    tmp_path: Path, content: str, diagnostic: str
) -> None:
    write(tmp_path, "docs/guide.md", content)

    result = run_checker(tmp_path, "--version", "0.6.0")

    assert result.returncode == 1
    assert diagnostic in result.stderr


def test_release_tags_and_generated_release_example(tmp_path: Path) -> None:
    write(
        tmp_path,
        "docs/guide.md",
        "git checkout rs-v0.5.0\n"
        "auto:task_browser::CollectFacts | 0.5.0 | Rust\n"
        "example.hello@0.5.0\n",
    )

    current = run_checker(tmp_path, "--tag", "py-v0.5.0")
    next_release = run_checker(tmp_path, "--tag", "rs-v0.6.0")
    invalid = run_checker(tmp_path, "--tag", "v0.6.0")

    assert current.returncode == 0, current.stderr
    assert next_release.returncode == 1
    assert "docs/guide.md:1: found 0.5.0; expected 0.6.0" in next_release.stderr
    assert "docs/guide.md:2: found 0.5.0; expected 0.6.0" in next_release.stderr
    assert "docs/guide.md:3:" not in next_release.stderr
    assert invalid.returncode != 0
    assert "--tag must be rs-vX.Y.Z or py-vX.Y.Z" in invalid.stderr
