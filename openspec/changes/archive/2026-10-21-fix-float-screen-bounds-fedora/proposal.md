# Proposal

## Why

The release PR #293 `linux-packages` run 39551127184, job `packages`, step
`Smoke-check the .rpm on Fedora`, fails on the headless self-test with
`FAIL core float_screen_bounds 437 float screen bounds` (exit 181 = 437 % 256).

The product behaviour is correct: `PanelColumn::moveFloat` clamps a top-level
float to the screen's available geometry, as `ui/panel-column` requires, and
check 136 already asserts that clamp from both screen corners. Check 437 is the
broken part: it runs its assertions only when a band exists between the main
window and the screen edge.

The window's minimum size (787-803 x 839-841 px) exceeds the 800x800 offscreen
screen, so a bottom band can never exist, and a right band exists only when
the platform's fonts keep the window narrower than the screen: Ubuntu (793 px)
left a 7 px band and passed, Fedora (803 px) left none and failed. The band
cannot be repaired, and the "sits outside the window rect" assertion built on
it is unsatisfiable.

## What Changes

- Assert the clamp contract directly instead of through a window/screen band:
  the check moves the float past the screen's bottom-right corner and requires
  it to come back clamped inside the screen's available geometry and resting on
  the screen's edge. Landing on the screen's edge is what tells a screen-relative
  clamp apart from a window-relative one, so the check keeps its teeth without
  depending on the window's size or position.
- Drop the `frameRect` band precondition and the "sits outside the frame"
  assertion, which the offscreen screen size makes unsatisfiable. The product
  requirement that a float may sit outside the main window's rect stays in
  `ui/panel-column`; it is a "may", and no offscreen screen is large enough to
  observe it.
- Drop the `resize(700, 500)` setup, which the window's minimum size turned
  into a no-op, and keep the window at the screen origin so the check's setup is
  explicit and the previous position is restored afterwards.

No user-facing behaviour changes. The self-test still asserts that a float moved
past the screen edge is clamped to the screen's available geometry.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `verification/verification-harness`: replaces the band precondition of the
  screen-relative self-test requirement with a screen-edge clamp assertion that
  does not depend on the main window's size or position.

## Impact

- `crates/pictura-app/cpp/selftest_shell_round4.cpp` — the `float_screen_bounds`
  check (437) asserts the screen clamp instead of a window/screen band.
- No bridge, API, data-format, or dependency changes.
