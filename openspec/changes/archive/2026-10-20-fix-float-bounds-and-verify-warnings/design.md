# Design

## Context

See `proposal.md` for the failure evidence and motivation. The relevant current
state:

- `float_screen_bounds` requests a 700x500 resize, but the frame's minimum size
  is 787x839, so the resize is clamped. It leaves the frame position to whatever
  earlier checks and the platform left behind. The check then requires
  `screen.bottom() > frameRect.bottom()` or `screen.right() > frameRect.right()`
  to choose a clamp direction. On the Fedora rpm smoke-check neither held, so the
  clamp assertions were skipped.
- The self-test runs under the offscreen QPA in `--headless` mode, where the
  screen is 800x800 at `(0, 0)` and client positioning is honored.
- `verify-fast.sh` invokes `check-local-build-caching.sh`, which configures up
  to three throwaway build directories; each configure repeats the Qt
  private-module warning because the project deliberately links `Qt6::GuiPrivate`
  for the RHI headers in `interop.cpp`.
- The Cargo workspace package is named `pictura_app`, but the test profile uses
  `pictura-app`, so the intended `opt-level = 2` never applies and every Cargo
  invocation prints a warning.
- Qt 6.11 warns about `QSortFilterProxyModel::invalidateFilter()` (deprecated in
  Qt 6.13) and `QApplication::setActiveWindow()` (deprecated in Qt 6.5), because
  the default `QT_WARN_DEPRECATED_UP_TO` covers both. It also marks
  `QFile::open()` and `QTest::qWaitForWindowActive()` as `[[nodiscard]]`.

## Goals / Non-Goals

**Goals:**

- Make `float_screen_bounds` deterministic under the offscreen QPA without
  weakening its clamping or outside-the-frame assertions.
- Remove every warning caused by project code or configuration from
  `verify-fast.sh`, `verify-full.sh`, and a clean CMake build.
- Keep the change portable across the Qt 6 versions used by the release channels
  (Linux bundle, Flatpak, Snap, macOS, Windows).

**Non-Goals:**

- Enabling `-Werror` or adding automated warning-count enforcement. The issue
  asks for a clean state, not a new gate that could break on a compiler or Qt
  upgrade.
- Fixing the `cargo deny` duplicate and unused license-allowance warnings in the
  lint job. They are not emitted by the named gates and are left for a follow-up.
- Fixing packaging-tool warnings from linuxdeploy, `dpkg-shlibdeps`, or the
  bundled Qt. They are third-party and documented in the archived test plan and
  the PR.
- Migrating `float_screen_bounds` from the self-test to Qt Test. That is the
  long-term GUI-test direction but not needed to stabilize this check.

## Decisions

### Position the frame after resizing

Save `frame.pos()` before the check, call `frame.resize(700, 500)` and then
`frame.move(0, 0)`, and restore both the position and the size after the check.
The frame's minimum size is 787x839, so the resize is clamped; at `(0, 0)` the
client rect is approximately `(2, 2, 787, 839)`. The screen is 800x800, so the
right band exists (`799 > 788`) while the bottom band cannot exist because the
frame is taller than the screen (`799 < 840`). The check therefore uses the
right band deterministically and keeps the same clamp and outside-the-frame
assertions.

Alternatives considered:

- Require both a bottom and a right band. The frame's minimum height makes a
  bottom band impossible at the screen origin; moving the frame off the top to
  create one would make the check depend on an artificial negative position and
  would not represent the screen-edge case better.
- Leave the position untouched and choose whichever band happens to exist. That
  is the current flake: Fedora placed the frame where neither band existed.
- Move without restoring. Later self-test checks would inherit the moved frame;
  restoring the position alongside the size keeps the check isolated.
- Recompute the frame against `availableGeometry` only when it does not fit.
  This adds conditional setup without improving determinism on the offscreen
  platform.

The bottom and top clamp directions remain covered by the separate
`m42_float_overlay` self-test in `selftest.cpp`, which moves a float past the
bottom-right and top-left screen corners.

Wayland clients cannot position top-level windows, but `--headless` selects the
offscreen QPA before `QApplication`, which is the environment the acceptance and
the release smoke-checks use.

### Use `invalidate()` instead of version-gated `beginFilterChange()`

`QSortFilterProxyModel::invalidateFilter()` is deprecated in Qt 6.13 and warns in
Qt 6.11. `beginFilterChange()` arrived in Qt 6.9 and the directional
`endFilterChange()` overload in Qt 6.10, while the project still builds against
older Qt 6 in some channels (the Snap KDE SDK is not pinned to the bundle's
6.11). `invalidate()` is public, not deprecated, and invalidates both filtering
and sorting. The layers proxy does not use a sort column, so the extra re-sort is
negligible. The explicit `modelReset`/`dataChanged` connections and the
`setSourceModel` override are removed: `dynamicSortFilter` defaults to true, so
the base class already refreshes changed rows incrementally, and `invalidate()`
is now used only when the filter parameters change.

Alternatives considered:

- `#if QT_VERSION >= QT_VERSION_CHECK(6, 10, 0)` around
  `beginFilterChange()`/`endFilterChange()` with `invalidateFilter()` otherwise.
  This is correct but adds conditional code to the filter-change call site for no
  behavioral gain.
- Keep the explicit connections and use `invalidate()` at each one. That makes
  every source `dataChanged` rebuild the whole proxy mapping, which is the
  signal-granularity regression this design avoids.
- Suppress the deprecation warning locally. That hides a real API migration and
  would accumulate debt.

### Suppress the Qt private-module warning at the CMake boundary

Set `QT_NO_PRIVATE_MODULE_WARNING ON` before `find_package(Qt6 ...)`. Qt's own
CMake helper documents this switch. `Qt6::GuiPrivate` is required for
`<rhi/qrhi.h>` in the canvas interop, so the warning is expected and not
actionable. Suppressing it once removes one warning from the main configure and
up to three from `check-local-build-caching.sh` on every fast gate.

Alternatives considered:

- Filter the log in the verify scripts. The issue explicitly rejects log
  filtering; the warning would remain in a clean CMake build.
- Drop `GuiPrivate`. The RHI interop would not compile.

### Suppress GCC 16 cxx-qt/Qt glue warnings at the bindings boundary

A clean build with GCC 16 emits 85 `-Wsfinae-incomplete` warnings from Qt's
`QChar` in the generated cxx-qt glue and one false-positive
`-Wmaybe-uninitialized` in cxxbridge's generated `Vec<LayerRow>` constructor.
Both are in generated or third-party code. Add
`cc.flag_if_supported("-Wno-sfinae-incomplete")` and
`cc.flag_if_supported("-Wno-maybe-uninitialized")` to the `cc_builder` closure in
`crates/pictura-app/build.rs`, with a comment naming the source. The flags are
scoped to the cxx-qt build, not the CMake shell.

Alternatives considered:

- Filter the build log. The issue rejects log filtering; the warnings would
  remain in a clean build.
- Change `LayerRow` or the bridge to avoid the GCC false positive. The warning
  is in cxxbridge's generated vector constructor, not in the project's bridge
  definition, so this would be guesswork against generated code.
- Drop the flags and accept the warnings. They dominate local build output and
  would violate the warning-free acceptance.

### Rename the Cargo test-profile key

Use `[profile.test.package.pictura_app]`, matching the package name. This removes
the warning from every Cargo invocation and makes the intended `opt-level = 2`
for application tests take effect. The alternative—deleting the stanza—would
silence the warning but leave application tests at `opt-level = 0`, contrary to
the profile comment that all workspace crates are optimized.

### Handle `[[nodiscard]]` and deprecated Qt calls directly

Cast the ignored results to `void` where the surrounding check already reports
failure (`selftest_session.cpp`) or where the wait is best-effort cleanup
(`tst_edit_shortcuts.cpp`). Replace `QApplication::setActiveWindow(&dialog)`
with `dialog.activateWindow()`, the documented replacement, keeping the existing
`qWaitForWindowActive` assertion.

## Risks / Trade-offs

- **App-test compile time rises** when the profile key starts applying
  `opt-level = 2` to `pictura_app` → task 3.7 records the nextest and clippy
  durations; if they approach the CI timeout budget, revisit the profile in a
  separate change rather than silently keeping the typo.
- **Removing the explicit connections relies on `dynamicSortFilter` staying at
  its default `true`** → the proxy never disables it; if a future change does,
  the source-model connections must be restored or `invalidate()` called at that
  point.
- **`invalidate()` remaps fully when filter parameters change** → this is the
  intended path (the filter set can change any row); source updates stay
  incremental through `dynamicSortFilter`.
- **Suppressing the Qt private-module warning could hide a future accidental
  private dependency** → the only private module is the explicit
  `Qt6::GuiPrivate` link with its RHI comment; reviewers see any new one in the
  CMake diff.
- **The cxx-qt `cc` build also compiles three small hand-written bridge edges**
  → `-Wno-maybe-uninitialized` could hide a warning there; those files are tiny
  and covered by the bridge and export tests, and the flag is absent from the
  CMake shell build.
- **`float_screen_bounds` covers only the right axis on the offscreen screen**
  → the frame's minimum height exceeds the screen; the bottom/top clamp remains
  covered by `m42_float_overlay`. If that check is removed, add a bottom-axis
  test.
- **Warning-free gates are verified manually** → the PR records the captured
  logs; a future compiler or Qt upgrade may introduce a new warning, which is
  acceptable and preferable to a brittle warning-count gate.
- **`move(0, 0)` is ignored on Wayland** → the self-test's supported acceptance
  path is `--headless` (offscreen), which honors client positioning; the check
  was already platform-dependent before this change.

## Migration Plan

No data, API, or format migration. Rollback is a straight revert of the change.
