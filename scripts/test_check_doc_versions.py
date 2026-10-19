"""Behavior tests for the documentation package-version checker."""

from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


CHECKER = Path(__file__).with_name("check-doc-versions.py")


class CheckDocVersionsTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)

    def write(self, name: str, content: str) -> None:
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")

    def run_checker(self, *args: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(CHECKER), "--root", str(self.root), *args],
            text=True,
            capture_output=True,
            check=False,
        )

    def test_current_package_versions_pass_without_flagging_task_data(self) -> None:
        self.write(
            "docs/guide.md",
            'genja = "0.5.0"\n'
            'python -m pip install "genja-py[cli]>=0.5.0,<0.6.0"\n'
            "genja task describe example.hello@0.5.0\n"
            'task version="0.5.0"\n'
            "hostname: 10.0.0.1\n",
        )

        result = self.run_checker("--version", "0.5.0")

        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("documentation package versions valid", result.stdout)

    def test_stale_versions_report_file_line_found_and_expected(self) -> None:
        self.write("docs/guide.md", "# Install\n\ncargo add genja@0.5.0\n")
        self.write("README.md", "The Genja v0.5.0 release is available.\n")
        self.write("genja-core/README.md", 'genja-core = "0.5.0"\n')
        self.write("genja/examples/python/README.md", 'genja = "0.5.0"\n')

        result = self.run_checker("--version", "0.6.0")

        self.assertEqual(result.returncode, 1)
        self.assertIn("docs/guide.md:3: found 0.5.0; expected 0.6.0", result.stderr)
        self.assertIn("README.md:1: found 0.5.0; expected 0.6.0", result.stderr)
        self.assertIn(
            "genja-core/README.md:1: found 0.5.0; expected 0.6.0", result.stderr
        )
        self.assertNotIn("genja/examples/python/README.md", result.stderr)

    def test_minor_range_stays_valid_for_patch_release(self) -> None:
        self.write(
            "docs/guide.md",
            'python -m pip install "genja-py[cli]>=0.5.0,<0.6.0"\n',
        )

        self.assertEqual(self.run_checker("--version", "0.5.1").returncode, 0)
        result = self.run_checker("--version", "0.6.0")
        self.assertEqual(result.returncode, 1)
        self.assertIn("found >=0.5.0,<0.6.0; expected >=0.6.0,<0.7.0", result.stderr)

    def test_pip_pin_and_cargo_version_operator_are_checked(self) -> None:
        self.write(
            "docs/guide.md",
            'python -m pip install "genja-py==0.5.0"\ngenja = { version = "^0.5.0" }\n',
        )

        result = self.run_checker("--version", "0.6.0")

        self.assertEqual(result.returncode, 1)
        self.assertIn("docs/guide.md:1: found 0.5.0; expected 0.6.0", result.stderr)
        self.assertIn("docs/guide.md:2: found 0.5.0; expected 0.6.0", result.stderr)

    def test_reasoned_exception_applies_to_one_occurrence_in_next_block(self) -> None:
        self.write(
            "docs/guide.md",
            '<!-- genja-version-check: allow-next 0.5.0 reason="First CLI release" -->\n'
            "```bash\n"
            "cargo add genja@0.5.0\n"
            "cargo add genja@0.5.0\n"
            "```\n",
        )

        result = self.run_checker("--version", "0.6.0")

        self.assertEqual(result.returncode, 1)
        self.assertNotIn("docs/guide.md:3:", result.stderr)
        self.assertIn("docs/guide.md:4: found 0.5.0; expected 0.6.0", result.stderr)

    def test_invalid_mismatched_and_unused_exceptions_fail(self) -> None:
        cases = (
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
        )
        for content, diagnostic in cases:
            with self.subTest(diagnostic=diagnostic):
                self.write("docs/guide.md", content)
                result = self.run_checker("--version", "0.6.0")
                self.assertEqual(result.returncode, 1)
                self.assertIn(diagnostic, result.stderr)

    def test_release_tags_and_generated_release_example(self) -> None:
        self.write(
            "docs/guide.md",
            "git checkout rs-v0.5.0\n"
            "auto:task_browser::CollectFacts | 0.5.0 | Rust\n"
            "example.hello@0.5.0\n",
        )

        current = self.run_checker("--tag", "py-v0.5.0")
        next_release = self.run_checker("--tag", "rs-v0.6.0")
        invalid = self.run_checker("--tag", "v0.6.0")

        self.assertEqual(current.returncode, 0, current.stderr)
        self.assertEqual(next_release.returncode, 1)
        self.assertIn(
            "docs/guide.md:1: found 0.5.0; expected 0.6.0", next_release.stderr
        )
        self.assertIn(
            "docs/guide.md:2: found 0.5.0; expected 0.6.0", next_release.stderr
        )
        self.assertNotIn("docs/guide.md:3:", next_release.stderr)
        self.assertNotEqual(invalid.returncode, 0)
        self.assertIn("--tag must be rs-vX.Y.Z or py-vX.Y.Z", invalid.stderr)


if __name__ == "__main__":
    unittest.main()
