# Proposal

## Why

Issue #681: the Type tools' caret is one static vertical line. CS6 draws an
I-beam whose cross-line marks the baseline (`docs/03-tools/type-tools.md`,
*Point type*) and blinks it, so the user can see where the type will sit and
that the session is live.

## What Changes

- **Blink:** the caret's vertical line blinks on a 1.5 s cycle (0.75 s shown,
  0.75 s hidden). A keystroke, click, or other edit shows it again and restarts
  the cycle.
- **Placement point:** point type marks its origin (the click) with a small
  square. The square stays visible while the line blinks.
- **Baseline guide:** once text is typed, a line runs along each line's baseline
  (the centre axis of a vertical column) under the text. It stays visible, and
  the caret keeps blinking at the end of the text.
- New engine call `pictura_render::type_baselines` gives the guide segments
  from the same layout the caret stops use.
- **BREAKING**: none.

## Capabilities

- Added `tools/type-caret`.
