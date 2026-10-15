# Python CLI wheel contract (#156)

This records the package and release decisions for the optional `genja-py[cli]`
installation. It is a development contract: the extra and local wheel builds
exist, but no CLI wheel has been published. Issue #155 is present on this branch.

Remove this temporary file before merging #156, after its enduring rules have
moved into release automation and user-facing documentation. Publishing the
wheels to PyPI belongs to the later release process.

## Package names and artifacts

| Name | Role |
| --- | --- |
| `genja-py` | PyPI distribution containing the Python `genja` import package and discovery helper. Its base installation does not depend on the CLI. |
| `genja-cli` | Proposed PyPI distribution containing the existing Rust CLI binary, built with `tui`. This is separate from the Cargo crate of the same name. |
| `genja` | Executable installed into the selected Python environment's scripts directory by the CLI wheel. |

On 2026-10-13, the [PyPI JSON endpoint for `genja-cli`](https://pypi.org/pypi/genja-cli/json)
and the [fallback endpoint for `genja-py-cli`](https://pypi.org/pypi/genja-py-cli/json)
both returned HTTP 404. A 404 only means there is no public release; a name
may still be reserved according to [PyPI's guidance](https://pypi.org/help/#project-name).
Confirm that `genja-cli` can be created under the Genja publisher account
before finalising package metadata. If it cannot, use `genja-py-cli`
consistently in the wheel metadata, extra, release workflow, and documentation.
Do not publish or claim a name merely to test this contract.

The CLI distribution will publish platform wheels only, with no source
distribution. An unsupported platform should fail installation with no
attempt to compile the CLI from source. Use
[Maturin's `bin` bindings](https://www.maturin.rs/bindings.html#bin) to place the
existing `genja` binary in the wheel's scripts directory. Because
`genja-cli` has both a library and binary target, select `bin` explicitly.

## Initial wheel support

Start with the wheel-only coverage of [published `genja-py` 0.4.0](https://pypi.org/project/genja-py/0.4.0/#files)
and add Intel macOS wheels for both packages in this release:

| Operating system | Architecture | Target wheel tag |
| --- | --- | --- |
| Linux | x86_64 | `manylinux_2_17_x86_64` |
| macOS | arm64 | `macosx_11_0_arm64` |
| macOS | x86_64 | `macosx_10_12_x86_64` |
| Windows | x86_64 | `win_amd64` |

The Intel build pins the macOS deployment target to 10.12. CI checks the wheel
tags and installs on a macOS 15 Intel runner; running on macOS 10.12 itself
still needs a separate compatibility check before claiming runtime support
there.

The Python package builds CPython 3.10 through 3.14 wheels for each row.
The CLI binary has no Python extension ABI, so aim for one compatible CLI
wheel per platform/architecture rather than one per Python version; inspect
the produced tags and test installation with every supported Python version.
Do not claim wheel-only support for Linux arm64, Windows arm64, or PyPy until
matching `genja-py` wheels and installation tests exist.

## Version and release rule

Keep the CLI wheel version aligned with its `genja-cli` Cargo crate and with
the `genja-py` release version. Pin the `cli` extra to the matching minor
release line: for example, a 0.5 release would require
`genja-cli>=0.5.0,<0.6.0`, or the fallback distribution with the same range.
This accepts compatible patch releases but does not silently cross a minor
release line. Raise the lower bound if a later patch introduces a required
helper protocol change.
Update the extra's version range when the release branch bumps package
versions, and verify it still matches the CLI wheel built from that branch.

Build and validate both packages from the same release commit. Publish the
CLI wheels before publishing the `genja-py` release that names them in its
extra. The release workflow must verify versions, wheel tags, and metadata
before either upload.

## Interpreter rule

An explicit `--python PATH` always wins. When the wheel-installed `genja`
binary discovers configured Python modules without that override, it must
use the interpreter from the environment into which the wheel was installed,
even if another `python` appears on `PATH`. The preferred implementation is a
wheel-specific default that checks for the companion interpreter beside the
installed executable (`bin/python` on Unix or `Scripts/python.exe` on Windows).
If that interpreter cannot be located, fail with a clear `--python` remedy
instead of silently using another environment. Cargo-installed and
project-local binaries retain their existing `python`-from-`PATH` default.

Phase 3 must test virtual environments, direct invocation by executable path,
an unrelated `python` earlier on `PATH`, Windows script paths, and the
`--python` override before treating the companion-interpreter approach as
final.
