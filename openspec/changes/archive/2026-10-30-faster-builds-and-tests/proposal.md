# Proposal

## Why

The edit-build-test loop is slow, but running the tests is not the cause: nextest runs 3543
tests in 34 s. The time goes to building the same code repeatedly:

- A default CMake build compiles the Rust library with `--release`, at opt-level 3 and
  without incremental compilation, so a one-line edit in a non-bridge app file costs 13.2 s
  of cargo time. An edit in any of the 77 bridge files costs about 59 s (58.7 s, #1042), but
  that is the cxx-qt build script regenerating and recompiling every bridge, which no Cargo
  profile reaches.
- Any byte changed in any of the 77 bridge files rebuilds 331 C++ translation units (about
  5000 CPU-s), even when the edit only touches a function body.
- CI compiles the workspace about five times per pull request: `lint` (clippy), `test`
  (nextest, then a doctest build), `qt-headless` (the CMake build) and `oracles` (the
  workspace again, plus its doctests), with `gpu-parity` compiling `pictura-render` and its
  dependencies on the side.

Measurements are from issue #1042 on master @ 1f730219. The Qt Test fixture and C++
compile-flag work from that issue moved to #1043.

## What Changes

- Optimized development CMake builds (`BUILD_TESTING` on, a single-config generator,
  `CMAKE_BUILD_TYPE` Release or RelWithDebInfo) compile the Rust library with a new `dev-app`
  Cargo profile. It keeps release semantics but compiles incrementally with more codegen
  units and line-table debug info. Release and packaging builds (`BUILD_TESTING` off) keep
  `release`, and a Debug configure keeps Corrosion's `dev` mapping.
- The bridge stamp records only the `#[cxx_qt::bridge]` module of each bridge file, with the
  outer attributes and doc comments directly above it, not the whole file. An edit outside
  that module (an `impl` body, a helper, a test) no longer rebuilds C++. The extracted text
  is checked before use, and a file whose module cannot be extracted cleanly is hashed whole.
- CMake re-runs its configure step only when a bridge file, a file `build.rs` lists,
  `crates/pictura-app/build.rs`, or the set of `.rs` files changes, not on every Rust edit.
- The Rust doctest runs are removed from `scripts/test-report.sh` and CI, because no runnable
  doctest exists. Every workspace library sets `doctest = false`, so a doc example cannot
  silently become a test that no gate runs.
- The CI `oracles` job is removed. The `test` job installs ImageMagick (its AppImage cached),
  `psd-tools` and a pinned `ag-psd`, so the oracle tests run once in the full workspace run.
- The `gpu-parity` job's `software-vulkan` clippy moves into the `lint` job.
- A CMake configure that keeps an explicit `-fuse-ld=` in its cached linker flags while mold
  is installed and accepted prints a warning once per build tree, naming the cached flag and
  the `-U` command that clears it. Stale build trees configured before the mold change
  otherwise keep linking with lld without saying so.
- **BREAKING (test report):** the unified test report has no doctest line, and
  `report_tests.py` drops its `--doctests` input. CI loses its `oracles` check; no required
  status check names it (verified: `master` has no branch protection and no active ruleset
  with required checks), so nothing waits on it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `meta/build-performance`: adds the development Rust profile requirement. Changes the
  bridge-stamp requirement to hash only bridge modules, with a checked fallback to the whole
  file. Adds a one-time warning scenario to the mold requirement.
- `verification/verification-harness`: the CI and local gates drop the doctest run; the
  oracle tests run in the `test` job and the `oracles` job is removed from the job-skipping
  rules.
- `verification/test-reporting`: the unified report drops its doctest layer.
- `verification/qt-test-harness`: the CTest harness requirement stops naming doctests.
- `verification/gpu-parity-ci`: the `software-vulkan` clippy runs in the `lint` job instead of
  `gpu-parity`.

## Impact

- `Cargo.toml` (new `[profile.dev-app]`, and the profile comment above
  `[profile.dev.package.*]`), each workspace crate's `Cargo.toml` (`[lib] doctest = false`).
- `CMakeLists.txt`: Corrosion `PROFILE` selection, bridge-module stamp, configure
  dependencies, linker warning.
- `crates/pictura-app/src/`: one `#[cfg(test)]` module with three tests.
- `scripts/test-report.sh`, `scripts/report_tests.py`, `scripts/verify-fast.sh` comments.
- `.github/workflows/ci.yml`: the `test`, `lint`, `gpu-parity` and `oracles` jobs. The
  `changes` job's classifier does not name `oracles`, so it is unchanged.
- `AGENTS.md`, `DEVELOPING.md`, `.serena/memories/suggested_commands.md`,
  `.serena/memories/conventions.md`, `.serena/memories/tech_stack.md`: doctest and
  `oracles` references, the build-profile note.
- `docs/dev/testing-conventions.md`, `docs/dev/STATE.md` (the change entry and the Commands
  block) and `docs/dev/opencv-feature-mapping.md`: doctest and `oracles` job references, in
  a separate commit carrying `TASK-ALLOWS-DOCS`.
- No new dependencies. No application behaviour change: `dev-app` inherits `release`, so
  overflow checks and debug assertions match the shipped build.
