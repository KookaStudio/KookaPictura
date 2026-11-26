# Proposal

## Why

`qt-headless` rebuilds its CMake, Corrosion, generated-header, moc, and FetchContent tree on every run. A warm run still spent 258 seconds in Build with a 99% sccache hit rate, so compiler caching alone does not remove the setup and dependency work.

## What Changes

- Restore the complete `build/` tree used by `qt-headless`, including Corrosion's Cargo output, generated cxx-qt headers, moc/autogen files, and FetchContent sources.
- Key restores on the runner image, Rust and C++ toolchains, Qt, CMake, Ninja, Cargo lockfile and manifests, build-graph inputs, profile, and configure flags. Source content stays out of the key so Ninja can rebuild only changed outputs.
- Save cache entries only from `master`. Use a stable restore prefix and a unique save key per successful default-branch run.
- Validate the restored tree against a content manifest before configure. Reject missing or changed generated outputs, missing Qt Test executables, a mismatched toolchain fingerprint, or a build tree from another source directory.
- Normalize unchanged source timestamps from the manifest before Ninja runs. Changed and new files stay newer than restored outputs.
- Record cache state, change classes, restored and saved sizes, restore and upload times, sccache hit rate, Ninja edge count, and Build duration in the job summary and a JSON artifact.
- Keep precompiled headers disabled, keep the application's GPU feature enabled, and preserve the existing per-executable Qt Test JUnit contract.

## Capabilities

### New Capabilities

- `meta/ci-build-tree-cache`: CI restore, save, stale rejection, incremental rebuild, and measurement rules for the Qt headless build tree.

### Modified Capabilities

None.

## Impact

- `.github/workflows/ci.yml` gains cache identity, restore, validation, save, and measurement steps for `qt-headless`.
- A standard-library Python helper owns manifest creation, stale-tree rejection, source timestamp normalization, and build-log measurement.
- OpenSpec gains the change artifacts and test plan.
- No dependency, compiler, build system, PCH, GPU, or Qt Test registration change is required.
- This change does not claim a ten-minute `qt-headless` runtime. Cache quota and restore cost remain go/no-go measurements, and #1056 stays open until the broader cache path is measured.
