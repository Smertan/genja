#!/usr/bin/env python3
"""Check whether a crate version is visible on crates.io."""

from __future__ import annotations

import argparse
import sys
import time
import urllib.error
import urllib.request


def version_exists(crate: str, version: str) -> bool:
    url = f"https://crates.io/api/v1/crates/{crate}/{version}"
    request = urllib.request.Request(
        url,
        headers={
            "Accept": "application/json",
            "User-Agent": "genja-release-workflow",
        },
    )
    try:
        with urllib.request.urlopen(request, timeout=30):
            return True
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return False
        raise


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("crate")
    parser.add_argument("version")
    parser.add_argument("--wait", action="store_true")
    parser.add_argument("--timeout", type=int, default=600)
    parser.add_argument("--interval", type=int, default=15)
    args = parser.parse_args()

    deadline = time.monotonic() + args.timeout
    while True:
        if version_exists(args.crate, args.version):
            print(f"{args.crate}@{args.version} is visible on crates.io")
            return 0

        if not args.wait or time.monotonic() >= deadline:
            print(f"{args.crate}@{args.version} is not visible on crates.io", file=sys.stderr)
            return 1

        time.sleep(args.interval)


if __name__ == "__main__":
    raise SystemExit(main())
