# Proposal

## Why

The release PR #293 `linux-packages` run #166, job `packages`, fails
intermittently in the Fedora rpm smoke-check at
`pictura self-test: FAIL core float_screen_bounds 437`. The check requests a
700x500 resize but the frame's minimum size is 787x839, so the resize is clamped;
it never places the frame, so whether a band exists between the frame and the
800x800 offscreen screen depends on the platform's default window placement.
Ubuntu left the frame at x=93 (`fr=792 < sr=799`) and passed; the captured Fedora
run printed `sb=799 fb=840 sr=799 fr=802`, so neither band existed and the clamp
assertions were skipped. The product clamping behavior is correct; the check's
setup is the flake.

In the same CI logs, the verify scripts and the CMake build repeat avoidable
warnings that bury real output: an unmatched Cargo profile key, the deliberate
Qt `GuiPrivate` dependency warning, three deprecated filter invalidation calls,
a deprecated active-window call, and two ignored `[[nodiscard]]` results. These
are project-caused and should be removed rather than filtered out of logs.

## What Changes

- Place the self-test frame at a known position `(0, 0)` after the resize, so
  the right band exists and the clamp direction is deterministic, and restore the
  previous position together with the previous size when the check finishes. The
  frame's minimum height is larger than the offscreen screen, so a bottom band is
  not available; the bottom/top clamp stays covered by the separate
  `m42_float_overlay` self-test. The clamp assertion stays unchanged.
- Rename the `pictura-app` test-profile key to the package's real name,
  `pictura_app`, removing the warning from every Cargo invocation and activating
  the intended `opt-level = 2` for the application crate's tests.
- Suppress Qt's private-module warning narrowly with
  `QT_NO_PRIVATE_MODULE_WARNING` before `find_package(Qt6)`, because
  `Qt6::GuiPrivate` is required for the RHI headers used by the canvas interop.
- Replace the deprecated `QSortFilterProxyModel::invalidateFilter()` calls with
  `invalidate()` and remove the redundant explicit `modelReset`/`dataChanged`
  connections, so `dynamicSortFilter` keeps source changes incremental and the
  proxy only remaps fully when the filter parameters change.
- Handle the two ignored `[[nodiscard]]` results explicitly, and replace the
  deprecated `QApplication::setActiveWindow()` call with
  `QWidget::activateWindow()`.
- Suppress GCC 16's warnings inside cxx-qt and Qt generated glue
  (`-Wsfinae-incomplete` and the `Vec<LayerRow>` `-Wmaybe-uninitialized` false
  positive) at the `cc_builder` boundary in the application build script.
- Record the remaining warnings that come from third-party packaging tools
  (linuxdeploy, dpkg-shlibdeps, and the bundled Qt) as documented noise; they are
  not caused by project code. The separate `cargo deny` duplicate and
  license-allowance warnings in the lint job are observed but left for a
  follow-up so this change stays scoped to the issue's named gates.

No user-facing behavior changes. The self-test continues to assert that a
floating overlay is clamped to the screen's available geometry and may sit
outside the main window's rect.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/verification-harness`: adds requirements that screen-relative
  self-tests are independent of default window placement and that the local fast
  and full verification gates and a clean CMake build emit no avoidable warnings.

## Impact

- `crates/pictura-app/cpp/selftest_shell_round4.cpp` — deterministic frame
  position for `float_screen_bounds` and restoration of the previous geometry.
- `Cargo.toml` — correct the `[profile.test.package.pictura_app]` key.
- `CMakeLists.txt` — suppress the Qt private-module warning at the boundary.
- `crates/pictura-app/cpp/panels/layers_filter_proxy.cpp` and
  `crates/pictura-app/cpp/panels/layers_filter_proxy.h` — remove the redundant
  source-model connections and replace the deprecated filter invalidation.
- `crates/pictura-app/cpp/selftest_session.cpp` and
  `crates/pictura-app/cpp/tests/tst_edit_shortcuts.cpp` — handle ignored
  `[[nodiscard]]` results.
- `crates/pictura-app/cpp/tests/tst_keyboard_shortcuts.cpp` — use
  `activateWindow()`.
- `crates/pictura-app/build.rs` — suppress GCC 16's cxx-qt/Qt glue warnings at
  the generated-bindings build boundary.
- No bridge, API, data-format, or dependency changes.
