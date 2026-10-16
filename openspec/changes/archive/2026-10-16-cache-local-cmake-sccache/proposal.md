# Proposal

## Why

CI caches C++ compilation with `sccache`, but a local, plain
`cmake -S . -B build` configures no compiler launcher, so every
`rm -rf build` or fresh worktree recompiles every C++ translation unit cold.
`sccache` is already installed and already the CI choice, so wiring it in
costs nothing and removes the largest avoidable local rebuild cost.

## What Changes

- `CMakeLists.txt` defaults `CMAKE_CXX_COMPILER_LAUNCHER` to `sccache` when
  `sccache` is on `PATH` and the user did not already set the launcher,
  guarding the assignment so a missing `sccache` never produces an unresolved
  launcher command. An explicit `-D` or the `CMAKE_CXX_COMPILER_LAUNCHER`
  environment variable still wins.
- `scripts/verify-full.sh` drops its now-redundant conditional
  `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache`, letting the CMake default apply.
- `DEVELOPING.md` documents that a plain configure picks up `sccache`, the
  persisted default cache location and size cap, how to override them, and the
  `SCCACHE_IGNORE_SERVER_IO_ERROR` escape hatch.
- A new self-skipping check, `scripts/check-local-build-caching.sh`, proves the
  default, the override, the no-sccache case, and a clean rebuild's cache hits
  against a real configure and build. It is wired into `verify-fast.sh` and the
  guards workflow, with the rebuild case run by `verify-full.sh`.

Out of scope: precompiled headers (incompatible with `sccache`) and wrapping
`rustc` locally (it defeats incremental Rust builds). Cross-worktree cache
sharing via `SCCACHE_BASEDIRS` is documented, not automated.

## Capabilities

### New Capabilities
- `meta/local-build-caching`: the local build defaults the C++ compiler
  launcher to `sccache` when present and unset, honours an explicit override,
  configures cleanly when `sccache` is absent, reuses cached objects on a clean
  rebuild, and documents the cache location and bound.

### Modified Capabilities
- `verification/verification-harness`: the "Local verification fast path and
  compiler caching" requirement is split. Its compiler-caching clause and the
  two sccache scenarios move to `meta/local-build-caching`; the fast-path
  contract is re-added as "Local verification fast path". `scripts/verify-full.sh`
  no longer passes a compiler-launcher flag.

## Impact

- `CMakeLists.txt`: guarded `find_program` + `set` block immediately after
  `project()`.
- `scripts/verify-full.sh`: remove the launcher conditional; add the rebuild
  check.
- `scripts/verify-fast.sh`, `.github/workflows/guards.yml`: run the new check.
- `DEVELOPING.md`: extend the "Speeding up local builds" section.
- `scripts/check-local-build-caching.sh`: new check script.
- No new dependencies; no new runtime code. CI's explicit
  `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` keeps working unchanged.
