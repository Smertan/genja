---
name: Release
about: Track preparation, publication, and verification of a Genja release.
title: "Release vX.Y.Z"
---

## Release scope

- Version: `X.Y.Z`
- Packages: <!-- Rust crates, Python package, or both -->
- Parent or related issues: <!-- Link any release blockers -->

Replace `X.Y.Z` and remove steps that do not apply to this release.

## Preparation

- [ ] Confirm required feature and release-blocking issues are complete.
- [ ] Create `release/X.Y.Z` from `develop`.
- [ ] Align versions and internal dependency requirements for every publishable Rust crate in this release.
- [ ] Align the Python package, binding crate, and optional CLI dependency versions if publishing Python packages.
- [ ] Finalize the top-level changelog, guides, package READMEs, and release notes.
- [ ] Confirm access and trusted publisher settings for each package being published.

## Validation

- [ ] Run `python scripts/validate-release.py --tag rs-vX.Y.Z` if publishing Rust crates.
- [ ] Run `python scripts/check-doc-versions.py --version X.Y.Z` after updating guides and package READMEs.
- [ ] Run `cargo fmt --all --check`.
- [ ] Run `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
- [ ] Run `cargo test --workspace --exclude genja-core-python --all-features`.
- [ ] Run `cargo check -p genja --no-default-features`.
- [ ] From `genja-core-python`, run `pdm run lint`, `pdm run typecheck`, and `pdm run check-stubs`.
- [ ] From `genja-core-python`, run `pdm run test` and `pdm run test-rust` in an appropriate Python environment.
- [ ] Build the Zensical documentation.
- [ ] Confirm the release pull request and required CI checks pass.

## Publication

- [ ] Merge `release/X.Y.Z` into `main`.
- [ ] Confirm required CI checks passed for the exact `main` commit being tagged.
- [ ] Create `rs-vX.Y.Z` on that commit and verify the crates.io publication if releasing Rust crates.
- [ ] Create `py-vX.Y.Z` on that commit and verify the PyPI publication if releasing Python packages.
- [ ] Create or verify GitHub release entries and release notes for the published tags.

## Post-release

- [ ] Verify published package versions and installation instructions using clean environments.
- [ ] Verify the release documentation and package links.
- [ ] Bring the released `main` commit back into `develop` using the documented branch flow.
- [ ] Record follow-up issues and close this release issue.
