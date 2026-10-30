# App architecture (pictura-app)

Entry points: `mem:core`, `mem:conventions`, `mem:task_completion`.

`crates/pictura-app` is **the only Qt crate**. Rust is built as a `staticlib` (+ `rlib` for Rust
tests) and linked into the C++ executable `pictura` by CMake via `cxx_qt_import_crate`
(Corrosion). Engine crates stay Qt-free.

## cxx-qt bridge

- Codegen is driven by `crates/pictura-app/build.rs` + `src/cxxqt_object.rs`.
- `cxxqt_object.rs` is a **Rust-2018 root** that keeps the `#[cxx_qt::bridge]` and shared helpers,
  with concern submodules under `src/cxxqt_object/`:
  `impl_{core,layers,selection,transform,paint,history,filters}.rs`, `helpers.rs`,
  `helpers_composite.rs`, `tests.rs`, `tests_impl.rs`.
- Keep the root + directory layout: the generated-header path and the 14 C++ includes depend on it.
- **Bridge files hold declarations only.** Every file build.rs hands to cxx-qt (`cxxqt_object.rs`,
  `decode_bridge.rs`, `cxxqt_object/**/<name>.rs` with a `#[cxx_qt::bridge]`) keeps only the bridge
  module, body-less `mod` declarations, `use`/`pub use` items, and empty trait impls;
  the guard `build_config::bridge_files_hold_declarations_only` enforces it. Bodies, helpers, types
  and tests live in the child module `<name>/imp.rs` (out-of-line test files under
  `<name>/imp/`), so editing them does not rerun the cxx-qt build script. `<name>.rs` declares
  `mod imp;` and re-exports with a glob (`use imp::*;`, or a `pub(...)`/`pub` glob matching the
  widest item, e.g. `pub(in crate::cxxqt_object) use imp::*;`), and `imp.rs` starts with
  `use super::*;` to see the bridge module, the kept imports and the sibling child modules. An item that was private in `<name>.rs` is `pub(super)` in
  `imp.rs`; one that was `pub(super)` is `pub(in crate::<parent>)`.
- Qt decodes images at the app boundary to packed RGBA8888 (`cpp/decode_image.cpp`); the engine
  never sees Qt types.

## C++/Qt shell (`crates/pictura-app/cpp/`)

- **New `.cpp`/`.h` files MUST be added to `CMakeLists.txt` explicitly — there is no globbing.**
- `main.cpp` is startup only: selects the offscreen QPA plugin before `QApplication` for `--headless`
  (which implies `--self-test` when no document is given), then calls `runSelfTest`.
  `--dump-theme` prints the theme token JSON and exits before any window/session code.
- Large classes are split across several TUs registered in `CMakeLists.txt` (`frame.cpp` →
  `frame_{columns,session,menus,build,test}.cpp`; `panels/panel_column.cpp` →
  `panel_column{,_drag,_iconic,_menu,_test}.cpp` + `panel_float.cpp`; etc.).

## Design system (theme tokens)

- `cpp/theme.{h,cpp}` is the token layer: `Theme::tokens(level)` returns
  `ThemeToken{key, value, color}` — every colour (including derived shades) plus the shared metrics
  — and `styleSheet` builds from the same list. `--dump-theme` prints it for all four levels.
- Each standard primitive has **one global QSS definition** (type- or object-name-scoped); widgets
  must not carry static-colour `setStyleSheet` calls. A new colour belongs in `Theme::tokens`
  (and the QSS), never in a widget.
- Data-driven colour fills use `panels/color_swatch_widget.{h,cpp}` (`ColorSwatchWidget`, a
  `QPushButton` that paints its colour; keeps the `color` dynamic property).
- `scripts/gen-styleguide.py` regenerates the committed `design-system.html` from `--dump-theme`;
  run it after any theme change. `tst_design_system` (Qt Test) guards palette/token equality,
  primitive selectors, and the object-name/state contract.

## The C++ self-test (hand-rolled oracle, not Qt Test/CTest)

- `runSelfTest()` (`cpp/selftest.cpp`, one long sequential function) is called from `main.cpp`.
  GUI checks now grow in Qt Test suites (`cpp/tests/tst_*.cpp`, `ctest -R '^tst_'`); the self-test
  only shrinks (`scripts/check-selftest-budget.sh`).
- Emits one token per check on stderr:
  `pictura self-test: PASS|SKIP|FAIL <suite> <name> ...`, closed by
  `pictura self-test: SUMMARY passed=<n> failed=<n> skipped=<n>`.
- **The exit code is the failure identity.** `ST_FAIL(code, ...)` returns its code; codes run from
  `2` upward and are **append-only**, so they stay stable identifiers. Exit `0` = all passed.
- Macros: `ST_BEGIN` / `ST_PASS` / `ST_SKIP` / `ST_FAIL` / `ST_FINISH`. Names carry **no milestone**.
- Checks live in per-concern `selftest_*.cpp` TUs (registered in `CMakeLists.txt`).
  `selftest.cpp` itself is the **single allowlisted** file with a fixed ceiling in
  `scripts/file-size-allowlist.txt` — add new checks to a `selftest_*.cpp`, never grow it.
- `scripts/report_tests.py` parses both self-test invocations' token streams and merges checks by
  `(suite, name)`.
- **State isolation**: `main.cpp` points `XDG_STATE_HOME` at a temp dir, so the self-test never
  touches real preferences/recovery state; if that temp dir cannot be created the run aborts
  (exit 199) instead of falling back to the real store. The session store is
  `$XDG_STATE_HOME/kooka-pictura/state.json` (default `~/.local/state/kooka-pictura/state.json`).
  Every write stamps a per-launch `writerToken`/`writtenAt`; `saveSession` refuses a store another
  launch wrote at or after this launch started, and a refused frame save skips the workspace
  snapshot (`session-store-single-writer`).

## GPU interop

`crates/pictura-app/GPU-INTEROP-NOTES.md`: offscreen wgpu→readback→QImage works and QRhi imports
the wgpu VkDevice/image, but on-screen present via `QRhiWidget` is blocked. `--interop-probe`
requires a real platform Vulkan instance (not offscreen-compatible).
