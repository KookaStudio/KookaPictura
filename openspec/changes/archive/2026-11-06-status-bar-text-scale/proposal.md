# Proposal

## Why

Issue #1231: at a UI scale below 100 % the status bar's readouts and tool hints
are squeezed into a bar that is too short for them. The footer text runs
`QApplication::font().pixelSize() - 2` px (10 px at 100 %), but that font is set
once on the `QStatusBar` in `buildStatusBar()` and never reaches the readouts:
the application stylesheet makes a child widget resolve its font against the
application font, so `statusSizeLabel`, `statusBackendLabel`, the tool hint strip
and the search hint all render at the full 12 px application size. At 100 % that
is already 2 px larger than the intended footer; at 75 % the bar and its items
shrink to 21 px and 17 px while the labels stay at 12 px, so the text fills the
strip and its descenders are clipped.

`applyStatusBarScale()` re-sizes the bar and its items when the scale changes but
never recomputes the footer font, so even the bar's own font stays at the
percentage it was built with.

## What Changes

- Make `applyStatusBarScale()` the single owner of the status bar's footer font:
  it derives the size from the current application font (2 px under, floored at
  1 px) and sets it on the bar and on every readout, hint strip, and the search
  hint directly, next to the item heights it already applies.
- Add a `font-size-footer` stylesheet token and a rule for the zoom field's
  nested edit and suffix, which the generic input rule would otherwise leave
  1 px (edit) and 2 px (suffix) larger than the footer; the rule survives a
  theme re-polish that would reset a directly set font on the edit.
- Call `applyStatusBarScale()` at the end of `buildStatusBar()` instead of the
  old bar-only font block, so construction, a live scale change, and a session
  restore all recompute the footer text and it fits its items at every choice
  from 75 % to 200 %.
- Add a Qt Test that asserts the footer font is the application font minus two
  pixels on the bar and every text carrier, including the zoom field's edit and
  suffix, that each carrier's height holds its font metrics, and that both hold
  after a live change to 75 %.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `ui/ui-scaling`: the live-scale requirement now names the status-bar footer
  text, which must be recomputed from the new application font and applied to
  the readouts, with a scenario at 75 %.

## Impact

- `crates/pictura-app/cpp/frame_build.cpp` (the footer font block moves out of
  `buildStatusBar()`; the scale helper is called after the readouts exist).
- `crates/pictura-app/cpp/frame_ui_scale.cpp` (`applyStatusBarScale()` applies
  the footer font to the bar and every readout as well as the item heights).
- `crates/pictura-app/cpp/theme.cpp` (the `font-size-footer` token and the zoom
  edit/suffix rule).
- `crates/pictura-app/cpp/tests/tst_ui_scale.cpp` (new
  `statusBarTextFollowsScale`).
- No header, document, codec, compositing, or dependency changes.
