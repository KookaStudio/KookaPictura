# Proposal

## Why

The Qt Test layer takes 249 s of wall time and a bridge-file edit costs 52 s
before any test runs (#1043, measured 2026-10-30 after #1050). Profiling shows
that more than half of every `PicturaMainWindow` construction is spent
re-polishing widgets after `Theme::apply` installs a new application style. The
suites build that window about 430 times per run, and CTest runs only four
suites at a time on a 16-core machine. On the build side, a body edit in any of
the 77 cxx-qt bridge files reruns the cxx-qt build script, which regenerates and
recompiles the C++ for every bridge.

## What Changes

- **Cheaper window construction.**
  - The theme is applied once, after the UI scale and font are resolved and
    before the window builds its menus, panels and tool controls.
  - A theme application is skipped when the application's live style, palette
    and style sheet already match.
  - The first-run panel layout is built once instead of being built and then
    regrouped.
- **CTest parallel level from the CPU count.** `scripts/test-report.sh` defaults
  to 80% of the available CPUs, and CI defaults to the runner's CPU count.
  `CTEST_PARALLEL_LEVEL` still overrides both. `tst_ui_scale`, the slowest
  single entry, is sharded, and so are `tst_print`, `tst_edit_refresh` and
  `tst_loading_tab`.
- **Condition waits instead of fixed sleeps** in the Qt suites, wherever the
  wait stands in for a condition that can be checked. A Rust test enforces a
  reason comment on every remaining delay.
- **One window per suite where it is proven safe.** The six suites that build a
  window in each test function are reviewed. A suite gets one shared window only
  when an `init()` reset makes its functions independent again. Otherwise it
  keeps its per-test window and its shards. The sharded test main honours the
  test functions named on its command line, so a single function can be run
  alone.
- **Bridge files hold declarations only.** The function bodies move out of the
  `#[cxx_qt::bridge]` files into ordinary modules, so an implementation edit no
  longer reruns the cxx-qt build script.
- **Measured compile changes.** Each one is kept only if neither a cold build
  nor a warm sccache rebuild gets slower and at least one gets faster, or the
  trade is recorded:
  - a precompiled header for `pictura_shell`, plus a matching one for the test
    targets;
  - less per-suite AUTOMOC work for the Qt Test targets;
  - cheaper debug and optimisation flags (`-Og`, `-gsplit-dwarf`) in the
    optimized development configuration.

  CI builds without precompiled headers, so a missing `#include` cannot hide
  behind the PCH.
- **Rust targets cached in CI** for the `lint` and `test` jobs, saved only on
  `master`. Their effect after the first `master` run is recorded in a
  follow-up issue (#1056).
- **A clean rebuild works.** `ninja -t clean` followed by a build regenerates the
  cxx-qt export directory instead of failing the shell link.
- **Engine-crate edit cost is measured first**, under `dev-app` and `--release`.
- **Measured and declined, recorded in the design notes:**
  - Lazy options-bar pages: they would change the bar's height, the enabled
    state of late pages and the test hooks, and `options_bar.cpp` is at its
    size cap.
  - Lazy panel creation: panels are 13% of window construction, and 220 test
    call sites would need changing.
  - Linking each test executable only once: a test link takes about 0.1 s with
    mold.

The window keeps the same palette, style sheet, fonts and panel arrangement.
What changes is the order and number of theme applications, and how many times
the first-run layout is built. One visible side effect was measured: panels are
now sized under the final style instead of under plain Fusion metrics, so the
first-run Adjustments group is 8 px taller (122 px instead of 114 px), and
Color and Layers each give up 4 px (design.md §1).

## Capabilities

### New Capabilities

None. The requirements extend existing capabilities.

### Modified Capabilities

- `verification/test-reporting`: the default CTest parallel level comes from
  the CPU count instead of a fixed four.
- `verification/verification-harness`: CI's CTest level is the runner's CPU
  count, and CI builds without precompiled headers.
- `verification/qt-test-harness`:
  - adds condition-based waiting;
  - adds the rule for when a suite may share one window between its test
    functions;
  - adds that a suite runs only the functions named on its command line;
  - extends the critical-path requirement with a bound on the longest entry.
- `meta/build-performance`:
  - adds the declarations-only bridge files, the clean-rebuild guarantee, CI
    target caching and the engine-edit measurement;
  - modifies the heaviest-translation-unit requirement into the cold-and-warm
    keep rule for PCH, moc and flag changes;
  - modifies the known-heavy-units requirement to drop `command_tree.cpp`,
    which no longer exists.
- `ui/application-shell`: adds the rule that building the main window applies
  the theme once, before its menus, panels and tool controls exist, and never
  re-applies a theme that is already live.
- `ui/workspaces`: adds the rule that the first-run layout is built once and
  equals the Essentials layout.

## Impact

- **C++:**
  - `crates/pictura-app/cpp/theme.cpp`, `frame.cpp`, `frame_build.cpp`,
    `frame_workspaces.cpp`;
  - the Qt suites under `crates/pictura-app/cpp/tests/`:
    - waits and shared windows;
    - the new suites `tst_theme_apply` and `tst_test_support`;
    - `qt_test_support.h`'s argument handling;
    - `tests/CMakeLists.txt`, for shards and AUTOMOC;
  - the root `CMakeLists.txt` (PCH, development flags, cxx-qt export
    regeneration).
- **Rust:**
  - Of the 77 bridge files build.rs lists, 76 are split into bridge and
    implementation modules as pure moves: the 75 under
    `crates/pictura-app/src/cxxqt_object/`, plus `src/decode_bridge.rs`.
    `build.rs` keeps its file list.
  - `src/build_config.rs` gains two source-scanning tests.
- **Scripts and CI:** `scripts/test-report.sh`, `.github/workflows/ci.yml`.
- **Docs:**
  - `AGENTS.md` and `DEVELOPING.md` (the CTest parallel default);
  - `docs/dev/STATE.md`, in its own `TASK-ALLOWS-DOCS` commit.
- **No new dependencies.**
