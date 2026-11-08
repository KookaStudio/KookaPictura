# Proposal

## Why

`CMakeLists.txt` hand-lists 558 shell entries (556 unique after removing two
duplicate lines) and 76 self-test sources in two append-only blocks. Every
feature that adds a C++ file edits the same pinned region, so parallel branches
collide on adjacent lines: the file has been touched by 197 commits, 95 of them
in the last 30 days, across three contributors. The split-out Qt Test list
(`crates/pictura-app/cpp/tests/CMakeLists.txt`) is far less conflict-prone and
is the pattern to scale up.

## What Changes

- Add `cmake/sources/*.cmake` fragments grouped along seams already present in
  the tree: `api`, `acr`, `panels`, `frame`, `tools`, `options_bar`,
  `image_view`, `control_server`, `update`, `documents`, `dialogs`, `filters`,
  `adjustments`, `shell`, and `selftest`.
- Each fragment appends to `PICTURA_SHELL_SOURCES` (or
  `PICTURA_SELF_TEST_SOURCES` for `selftest.cmake`) with `list(APPEND ...)`.
- Root `CMakeLists.txt` replaces the two `set(...)` blocks with `include()`
  calls before the existing `add_library` / `add_executable` calls.
- Add `scripts/check-cmake-source-fragments.py` (with `--self-test`) and wire it
  into `scripts/verify-fast.sh` and `.github/workflows/guards.yml`; it fails on
  a duplicate path, a listed path missing from disk, an unlisted non-test
  non-Rust-built `.cpp`, or a self-test source in a shell fragment.
- Update `AGENTS.md` and `DEVELOPING.md` to point new sources at the matching
  fragment instead of the root file, and Qt Test sources at
  `crates/pictura-app/cpp/tests/CMakeLists.txt`.
- Drop `.github/CODEOWNERS` from the original issue scope: with three
  contributors it adds review friction without branch protection requiring it.
- No new targets, no `cpp/` directory moves, no globbing, no `target_sources()`
  per fragment (that would break `pictura_depend_on_bridge_stamp`, which
  consumes the whole list).

## Capabilities

### New Capabilities

- `meta/cmake-source-fragments`: the repository's C++ source lists are split
  into per-area fragments, each translation unit appears exactly once (headers
  may be listed but are not required), the root `CMakeLists.txt` keeps the
  target definitions and includes the fragments, a repository check enforces
  the fragment invariants, and the target graph and compiled translation-unit
  set stay identical.

### Modified Capabilities

None.

## Impact

- `CMakeLists.txt` (two source-list blocks become `include()` lines).
- New `cmake/sources/*.cmake` (15 files).
- New `scripts/check-cmake-source-fragments.py`, wired into
  `scripts/verify-fast.sh` and `.github/workflows/guards.yml`.
- `AGENTS.md`, `DEVELOPING.md` (source-addition guidance).
- No application behavior, Rust code, target graph, or release artifact change.
- File-size guard does not apply to `.cmake`; the release-please version marker
  stays in the root `CMakeLists.txt`.
