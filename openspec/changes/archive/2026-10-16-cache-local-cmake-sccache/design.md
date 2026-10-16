# Design

## Context

See `proposal.md` for motivation. Current state that shapes the approach:

- `CMakeLists.txt` enables `CMAKE_EXPORT_COMPILE_COMMANDS ON` and links
  `compile_commands.json` to the repo root. **This does not expose the compiler
  launcher**: CMake's Ninja generator applies a launcher through a build-rule
  variable (`build.ninja`: `LAUNCHER = …`), not the per-file command, so
  `compile_commands.json` looks identical with and without a launcher.
- `scripts/verify-full.sh` and CI pass `-DCMAKE_CXX_COMPILER_LAUNCHER=sccache`
  explicitly; neither exercises a CMake default.
- `CMakeLists.txt` enables `CMAKE_AUTOMOC ON` and mixes Rust (via Corrosion /
  CXX-Qt) with the C++ shell. The launcher wraps only the C++ compiler.
- `find_program` without `REQUIRED` leaves the variable as
  `<NAME>-NOTFOUND`; assigning that to `CMAKE_CXX_COMPILER_LAUNCHER` produces a
  build rule that runs the literal string `SCCACHE_EXECUTABLE-NOTFOUND`.
- `scripts/check-worktree-hook.sh` is the existing model for a self-skipping
  check wired into `verify-fast.sh` and the guards workflow.

## Goals / Non-Goals

**Goals:**
- A plain configure and any `rm -rf build` rebuild reuse cached C++ objects.
- An explicit launcher choice by the user is never overridden.
- Missing `sccache` degrades silently to a normal, working configure and build.
- The behaviour is proven by a runnable, gated check.

**Non-Goals:**
- Precompiled headers (incompatible with `sccache`).
- Wrapping local `rustc` (defeats incremental Rust builds).
- Automating cross-worktree cache sharing (documented, not configured).
- Changing CI's explicit flag.

## Decisions

### D1. A CMake default, not `.cargo/config.toml` or a wrapper script

`.cargo/config.toml` cannot express "use `sccache` only if installed" — a hard
path there breaks machines without it. A wrapper script adds a file to maintain
for a one-line `find_program`. CMake's `<LANG>_COMPILER_LAUNCHER` is the native
mechanism, so the default lives in `CMakeLists.txt`.

### D2. Soft detection, guarded twice

Use `find_program(SCCACHE_EXECUTABLE sccache)` (not `REQUIRED`) inside
`if(NOT CMAKE_CXX_COMPILER_LAUNCHER)`, and guard the assignment with a nested
`if(SCCACHE_EXECUTABLE)`:

```cmake
if(NOT CMAKE_CXX_COMPILER_LAUNCHER)
    find_program(SCCACHE_EXECUTABLE sccache)
    if(SCCACHE_EXECUTABLE)
        set(CMAKE_CXX_COMPILER_LAUNCHER "${SCCACHE_EXECUTABLE}")
    endif()
endif()
```

The nested guard is mandatory: without it, a missing `sccache` sets the launcher
to `SCCACHE_EXECUTABLE-NOTFOUND` and every compile fails with
`command not found` (verified). The outer guard is what lets an explicit `-D` or
environment value win.

### D3. Place the block immediately after `project()`

`CMAKE_CXX_COMPILER_LAUNCHER` is a cache variable seeded from the
`CMAKE_CXX_COMPILER_LAUNCHER` environment variable during language enablement
(`project()` at `CMakeLists.txt:8`). Placing the guard after `project()` means an
environment-variable override is already visible and the guard skips it. Placing
it immediately after `project()` (not merely "before `add_library`") keeps CxxQt
FetchContent targets created at `CMakeLists.txt:49` from being pulled in before
the default is set.

### D4. Remove `verify-full.sh`'s conditional flag

Once CMake defaults the launcher, `scripts/verify-full.sh` passing
`-DCMAKE_CXX_COMPILER_LAUNCHER=sccache` is duplicated logic; removing it keeps
one source of truth. CI keeps its explicit flag (the override path proves it
still works).

### D5. Document the native cache defaults rather than configure them

`sccache` already persists to `~/.cache/sccache` on Linux (macOS uses
`~/Library/Caches/Mozilla.sccache`, Windows `%LOCALAPPDATA%\Mozilla\sccache`)
with a 10 GiB cap, overridable via `SCCACHE_DIR` and `SCCACHE_CACHE_SIZE`. The
issue's second item is satisfied by documenting these defaults and overrides; no
project-level cache configuration is added.

### D6. Verify through `build.ninja`, not `compile_commands.json`

Because `compile_commands.json` omits the launcher, the check asserts the
build-rule variable instead: `build.ninja` carries a `LAUNCHER = …` line when a
launcher is set and no such line when one is not. The rebuild hit-rate is proven
separately with `sccache --show-stats` before and after a clean rebuild.

### D7. Protect the repo-root `compile_commands.json` symlink

`CMakeLists.txt:12-19` unconditionally removes and re-links the root
`compile_commands.json` to whichever build directory was just configured. A
check that configures throwaway directories would therefore leave a dangling
symlink (breaking clangd/Serena). The check saves the current link target and
restores it on exit via a `trap`.

### D8. Gate the check

Wire `scripts/check-local-build-caching.sh` into `verify-fast.sh`'s full gate
and the guards workflow, self-skipping when `cmake` or `qmake6` is absent (the
guards runner installs no Qt, so it skips there). The slower rebuild hit-rate
case runs from `verify-full.sh` only, so the fast loop stays free of a full
build.

## Risks / Trade-offs

- [AUTOMOC/moc generation is not cached (`mozilla/sccache#2509`)] → the win is
  the compiled translation units, not `moc`; documented so nobody over-claims.
- [`sccache` fails the build if its server is unreachable] → document
  `SCCACHE_IGNORE_SERVER_IO_ERROR=1` as the escape hatch. The default only
  engages when `sccache` is installed.
- [Cross-worktree rebuilds miss the cache (different absolute paths)] →
  document `SCCACHE_BASEDIRS` as the advanced knob; note that `sccache`'s Rust
  support for it is newer than the installed 0.17.0. Not automated here.
- [The check needs a real configure (Qt + Corrosion; CxxQt via FetchContent if
  absent)] → self-skip when `cmake` or `qmake6` is absent; rebuild case is
  verify-full only.
- [Precompiled headers + gcc + `sccache` fail (`mozilla/sccache#615`)] → PCH
  stays out of scope; no PCH is configured today.
