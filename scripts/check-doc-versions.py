#!/usr/bin/env python3
"""Find stale Genja package versions in user-facing Markdown documentation."""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path


PACKAGE_READMES = (
    "README.md",
    "genja/README.md",
    "genja-core/README.md",
    "genja-core-derive/README.md",
    "genja-plugin-manager/README.md",
    "genja-cli/README.md",
    "genja-core-python/README.md",
)
VERSION = r"[0-9]+\.[0-9]+\.[0-9]+"
SEMVER_NUMBER = r"(?:0|[1-9][0-9]*)"
SEMVER_CORE = rf"{SEMVER_NUMBER}\.{SEMVER_NUMBER}\.{SEMVER_NUMBER}"
VERSION_LINE = r"[0-9]+\.[0-9]+\.x"
PACKAGE = r"genja(?:-(?:core(?:-derive|-python)?|plugin-manager|cli|py))?"
TAG = re.compile(rf"^(?:rs|py)-v(?P<version>{VERSION})$")
BRANCH = re.compile(
    rf"^release/[0-9]+-v(?P<version>{SEMVER_CORE})"
    r"(?:-(?P<prerelease>[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$"
)
EXCEPTION = re.compile(
    rf"^\s*<!-- genja-version-check: allow-next (?P<version>{VERSION}|{VERSION_LINE}) "
    r'reason="(?P<reason>[^"\n]+)" -->\s*$'
)
EXCEPTION_MARKER = "genja-version-check:"
RANGE = re.compile(
    rf"(?<![\w-]){PACKAGE}(?:\[cli\])?\s*>=\s*"
    rf"(?P<lower>{VERSION})\s*,\s*<\s*(?P<upper>{VERSION})"
)
PACKAGE_VERSION = re.compile(
    rf"(?<![\w-]){PACKAGE}(?:\[cli\])?"
    rf"(?:\s*=\s*(?:\{{\s*version\s*=\s*)?[\"']?[=^~]?|\s*@|\s+v)"
    rf"(?P<version>{VERSION})(?![\d.])"
)
PREFIXED_VERSION = re.compile(
    rf"(?<![\w.@])(?:rs-|py-)?v(?P<version>{VERSION}|{VERSION_LINE})\b"
)
GENERATED_TASK = re.compile(
    rf"auto:task_browser::[^\n]*?\|\s*[`\"']?(?P<version>{VERSION})(?![\d.])"
)
BARE_VERSION = re.compile(rf"(?<![\w.@])(?P<version>{VERSION})(?![\d.])")
PACKAGE_CONTEXT = re.compile(r"\b(?:Genja|genja|release|package|crate|wheel)\b")
TASK_CONTEXT = re.compile(r"\btask\s+(?:describe|register|list|docs)\b")


@dataclass(frozen=True)
class Reference:
    """One release-dependent package version found on a Markdown line."""

    start: int
    end: int
    found: str
    expected: str
    exception_version: str


@dataclass(frozen=True)
class ExceptionDirective:
    """A reasoned exception for the next package version occurrence."""

    version: str
    line: int


def expected_range(version: str) -> tuple[str, str]:
    major, minor, _ = (int(part) for part in version.split("."))
    return f"{major}.{minor}.0", f"{major}.{minor + 1}.0"


def references(line: str, version: str, *, in_fence: bool) -> list[Reference]:
    """Recognize package versions without treating task versions as package data."""

    found: list[Reference] = []
    occupied: list[tuple[int, int]] = []

    def add(start: int, end: int, actual: str, expected: str, exception: str) -> None:
        if any(
            start < other_end and end > other_start
            for other_start, other_end in occupied
        ):
            return
        occupied.append((start, end))
        found.append(Reference(start, end, actual, expected, exception))

    lower_expected, upper_expected = expected_range(version)
    for match in RANGE.finditer(line):
        lower, upper = match.group("lower", "upper")
        add(
            match.start(),
            match.end(),
            f">={lower},<{upper}",
            f">={lower_expected},<{upper_expected}",
            lower,
        )

    for match in PACKAGE_VERSION.finditer(line):
        actual = match.group("version")
        add(match.start(), match.end(), actual, version, actual)

    for match in PREFIXED_VERSION.finditer(line):
        actual = match.group("version")
        expected = (
            f"{version.rsplit('.', 1)[0]}.x" if actual.endswith(".x") else version
        )
        add(match.start(), match.end(), actual, expected, actual)

    for match in GENERATED_TASK.finditer(line):
        actual = match.group("version")
        add(match.start(), match.end(), actual, version, actual)

    # Unprefixed prose is relevant only when its line names a package/release.
    if not in_fence and PACKAGE_CONTEXT.search(line) and not TASK_CONTEXT.search(line):
        for match in BARE_VERSION.finditer(line):
            actual = match.group("version")
            add(match.start(), match.end(), actual, version, actual)

    return sorted(found, key=lambda item: item.start)


def check_file(path: Path, root: Path, version: str) -> list[str]:
    """Return diagnostics for stale versions or invalid exception directives."""

    relative = path.relative_to(root).as_posix()
    errors: list[str] = []
    pending: ExceptionDirective | None = None
    in_fence = False

    for line_number, line in enumerate(
        path.read_text(encoding="utf-8").splitlines(), 1
    ):
        if pending is not None and line_number - pending.line > 3:
            errors.append(f"{relative}:{pending.line}: unused version exception")
            pending = None

        directive = EXCEPTION.fullmatch(line)
        if directive is not None and directive.group("reason").strip():
            if pending is not None:
                errors.append(f"{relative}:{pending.line}: unused version exception")
            pending = ExceptionDirective(directive.group("version"), line_number)
            continue
        if "<!--" in line and EXCEPTION_MARKER in line:
            errors.append(
                f"{relative}:{line_number}: invalid version exception; expected allow-next VERSION with a nonempty reason"
            )
            continue

        if line.lstrip().startswith(("```", "~~~")):
            in_fence = not in_fence
            continue

        for reference in references(line, version, in_fence=in_fence):
            if pending is not None:
                if pending.version != reference.exception_version:
                    errors.append(
                        f"{relative}:{pending.line}: version exception for {pending.version} "
                        f"does not match {reference.exception_version} at line {line_number}"
                    )
                pending = None
                continue
            if reference.found != reference.expected:
                errors.append(
                    f"{relative}:{line_number}: found {reference.found}; "
                    f"expected {reference.expected}"
                )

    if pending is not None:
        errors.append(f"{relative}:{pending.line}: unused version exception")
    return errors


def documentation_files(root: Path) -> list[Path]:
    """Select guides and package-root READMEs, excluding fixtures and examples."""

    files = set((root / "docs").rglob("*.md"))
    files.update(root / name for name in PACKAGE_READMES if (root / name).is_file())
    return sorted(files)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    choice = parser.add_mutually_exclusive_group(required=True)
    choice.add_argument("--version", help="Intended release version, X.Y.Z")
    choice.add_argument("--tag", help="Release tag, rs-vX.Y.Z or py-vX.Y.Z")
    choice.add_argument(
        "--branch", help="Release branch, release/ISSUE-vX.Y.Z[-PRERELEASE]"
    )
    parser.add_argument(
        "--root",
        type=Path,
        default=Path(__file__).resolve().parents[1],
        help="Repository root (defaults to the script's parent repository)",
    )
    args = parser.parse_args()

    if args.tag is not None:
        match = TAG.fullmatch(args.tag)
        if match is None:
            parser.error("--tag must be rs-vX.Y.Z or py-vX.Y.Z")
        version = match.group("version")
    elif args.branch is not None:
        match = BRANCH.fullmatch(args.branch)
        if match is None or any(
            len(identifier) > 1 and identifier.startswith("0") and identifier.isdigit()
            for identifier in (match.group("prerelease") or "").split(".")
        ):
            parser.error("--branch must be release/ISSUE-vX.Y.Z[-PRERELEASE]")
        version = match.group("version")
    else:
        version = args.version
        if re.fullmatch(VERSION, version) is None:
            parser.error("--version must be X.Y.Z")

    root = args.root.resolve()
    if not root.is_dir():
        parser.error(f"repository root does not exist: {root}")

    files = documentation_files(root)
    if not files:
        parser.error(f"no documentation files found under {root}")

    errors = [error for path in files for error in check_file(path, root, version)]
    for error in errors:
        print(f"error: {error}", file=sys.stderr)
    if errors:
        return 1

    print(f"documentation package versions valid for {version} ({len(files)} files)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
