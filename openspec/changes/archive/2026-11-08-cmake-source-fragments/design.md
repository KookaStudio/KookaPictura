# Design

## Context

See `proposal.md` for motivation. The current shape that constrains the move:

- `CMakeLists.txt:351-910` sets `PICTURA_SHELL_SOURCES` (558 entries, 556 unique
  after removing two duplicate lines).
- `CMakeLists.txt:979-1056` sets `PICTURA_SELF_TEST_SOURCES` (76 entries).
- Three consumers read each variable whole: `add_library` (`:918/921`),
  `add_executable` (`:1058`), and `pictura_depend_on_bridge_stamp`
  (`:926`, `:1079`).
- `crates/pictura-app/cpp/tests/CMakeLists.txt` is already split and is the
  pattern being scaled.
- `include()` runs in the caller's scope, does not change
  `CMAKE_CURRENT_SOURCE_DIR`, and makes the included file a configure
  dependency, so editing a fragment re-runs configure and relative source paths
  stay rooted at the top-level source directory.

## Goals / Non-Goals

**Goals:**

- Move the two lists into `cmake/sources/*.cmake` with no change to targets,
  flags, compiled translation units, or the release marker.
- Make a source addition touch one area file, not the root build file.
- Keep the existing variables and the bridge-stamp call working unchanged.

**Non-Goals:**

- `file(GLOB ... CONFIGURE_DEPENDS)`, `add_subdirectory` per feature, or moving
  files under `cpp/`.
- `.github/CODEOWNERS` (dropped from the issue: three contributors, no branch
  protection requiring code-owner review).

## Decisions

### Fragments append to the existing variables, not `target_sources()`

Each fragment calls `list(APPEND PICTURA_SHELL_SOURCES ...)` (or
`PICTURA_SELF_TEST_SOURCES`), and the root includes every fragment before the
target calls. This keeps `add_library`, `add_executable`, and
`pictura_depend_on_bridge_stamp(${...})` byte-identical.

Alternative: `target_sources(pictura_shell PRIVATE ...)` per fragment. Rejected:
the target must exist before the include, and the bridge-stamp call would need
its own accumulated list or a per-fragment call — more moving parts for the same
result.

### Flat `cmake/sources/` with 15 concern fragments

| Fragment | Assignment rule |
|---|---|
| `api.cmake` | `cpp/api/` |
| `acr.cmake` | `cpp/acr/` |
| `panels.cmake` | `cpp/panels/` |
| `frame.cmake` | root `frame*` |
| `tools.cmake` | root `tool_*`, `tools_*`, `toolbox*` |
| `options_bar.cmake` | root `options_bar*` |
| `image_view.cmake` | `image_view*`, `canvas_*`, `vp_*`, `screen_*` |
| `control_server.cmake` | `control_*`, `client_*`, `agent_control*` |
| `update.cmake` | root `update_*` |
| `documents.cmake` | document/file/save/pdf/import/export/encode/decode/print/format options |
| `dialogs.cmake` | generic dialogs and chrome not claimed below |
| `filters.cmake` | filter/blur/lens/gradient/lighting/liquify/warp/noise/fill/stroke |
| `adjustments.cmake` | adjustment/color/levels/curves/hue/exposure/channel/blend/hdr/mode |
| `shell.cmake` | remainder: commands, interop, pch, theme, icons, fonts, search, utilities |
| `selftest.cmake` | `main.cpp` + `selftest*` -> `PICTURA_SELF_TEST_SOURCES` |

The shell fragments hold 556 unique paths and `selftest.cmake` 76.

`filters.cmake` and `adjustments.cmake` stay separate because filter and
adjustment ports are the most frequent recent source additions. Exact
path-to-fragment assignment is generated from the current list, reviewed once,
and then lives in the fragments themselves; a dialog follows its concern
(`filter_gallery_dialog` -> filters) unless it is generic (`canvas_size_dialog`
-> dialogs).

### Include placement

The shell fragment includes replace the `set(PICTURA_SHELL_SOURCES ...)` block
in place, after the `pictura_depend_on_bridge_stamp` function definition and
before `add_library`. The selftest include replaces its `set(...)` block before
`add_executable`.

### The source-list check is mechanical

`scripts/check-cmake-source-fragments.py` verifies the invariants without a
manual inspection: no duplicate path, every listed path exists on disk, every
non-test non-Rust-built `.cpp` under `crates/pictura-app/cpp/` is listed, and no
`main.cpp`/`selftest*` source appears in a shell fragment. It supports a
`--self-test` and runs in `scripts/verify-fast.sh` and the guards workflow.

### Documentation

`AGENTS.md:33` and `DEVELOPING.md:102, :156, :163, :345` change from "add to
`CMakeLists.txt`" to "add to the matching `cmake/sources/*.cmake` fragment".
Qt Test sources keep going to `crates/pictura-app/cpp/tests/CMakeLists.txt`.
Both docs are root files, so the `docs/` guard marker does not apply.

## Risks / Trade-offs

- **`panels.cmake` / `filters.cmake` become the new hotspots** -> still ~5x
  smaller than 556; split a fragment further only if it collides.
- **A missing header is invisible to the build** -> headers are optional; the
  mechanical check covers translation units, duplicates, and missing paths.
- **A missing unreferenced `.cpp` is also invisible** -> the mechanical check
  covers it; the build catches referenced ones.
- **CMake include tracking** -> `include()` files are configure dependencies,
  so no `CMAKE_CONFIGURE_DEPENDS` entry is needed.
- **Packaging workflows do not list `cmake/**`** -> they gate on the
  release-please branch and release PRs do not edit fragments; leave them.
- **Order of sources changes** -> CMake does not care; the compiled-set check
  proves equivalence.

## Migration Plan

One pure-move commit: create the fragments, replace the two `set(...)` blocks
with includes, update the docs, and add the source-list check wired into
`verify-fast.sh` and the guards workflow. Reverting the commit restores the
original file. No data or runtime migration.

## Open Questions

None.
