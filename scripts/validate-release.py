#!/usr/bin/env python3
"""Validate Genja Rust crate metadata before CI and crates.io publishing."""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from typing import Any


PUBLISH_ORDER = [
    "genja-core-derive",
    "genja-core",
    "genja-plugin-manager",
    "genja",
]

VERSION_TAG = re.compile(r"^v(?P<version>[0-9]+\.[0-9]+\.[0-9]+)$")


def cargo_metadata() -> dict[str, Any]:
    output = subprocess.check_output(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        text=True,
    )
    return json.loads(output)


def is_publishable(package: dict[str, Any]) -> bool:
    return package.get("publish") != []


def normalize_req(req: str) -> str:
    return req.removeprefix("^")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--tag",
        help="Release tag to validate, formatted as vX.Y.Z.",
    )
    args = parser.parse_args()

    expected_version = None
    if args.tag is not None:
        match = VERSION_TAG.fullmatch(args.tag)
        if match is None:
            print(f"error: release tag must match vX.Y.Z: {args.tag}", file=sys.stderr)
            return 1
        expected_version = match.group("version")

    packages = {package["name"]: package for package in cargo_metadata()["packages"]}
    errors: list[str] = []

    missing = [crate for crate in PUBLISH_ORDER if crate not in packages]
    if missing:
        errors.append(f"publish order references missing workspace crates: {', '.join(missing)}")

    versions: dict[str, str] = {}
    for crate in PUBLISH_ORDER:
        if crate not in packages:
            continue

        package = packages[crate]
        if not is_publishable(package):
            errors.append(
                f"{crate} is in publish order but is not publishable in {package['manifest_path']}"
            )

        version = package["version"]
        versions[crate] = version

        if expected_version is not None and version != expected_version:
            errors.append(
                f"{crate} version {version} does not match release tag version {expected_version}"
            )

        for dep in package["dependencies"]:
            dep_name = dep["name"]
            if dep_name not in PUBLISH_ORDER:
                continue
            if "path" not in dep:
                errors.append(f"{crate} dependency {dep_name} is missing path")
            if dep["req"] == "*":
                errors.append(f"{crate} dependency {dep_name} is missing version")
            elif normalize_req(dep["req"]) != version:
                errors.append(
                    f"{crate} dependency {dep_name} version requirement {dep['req']} "
                    f"does not match {crate} version {version}"
                )

    unique_versions = sorted(set(versions.values()))
    if len(unique_versions) > 1:
        pairs = ", ".join(f"{crate}={version}" for crate, version in versions.items())
        errors.append(f"publishable crates must share one version: {pairs}")

    if errors:
        for error in errors:
            print(f"error: {error}", file=sys.stderr)
        return 1

    version = expected_version or unique_versions[0]
    print(f"release metadata valid for version {version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
