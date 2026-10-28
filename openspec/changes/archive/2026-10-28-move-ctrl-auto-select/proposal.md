# Proposal: move-ctrl-auto-select

## Why

Issue #981: with the Move tool, CS6 treats a held Ctrl as a temporary flip of
the Auto-Select checkbox. Pictura ignored Ctrl, so a Ctrl-click on a lower
layer's pixels left the active layer alone and the checkbox never changed.

## What Changes

- A Move press with Ctrl held applies the inverse of the stored Auto-Select
  option: off becomes on (the topmost opaque layer under the pointer, resolved
  by the Group/Layer dropdown, becomes active) and on becomes off.
- While Ctrl is held the options bar shows the flipped checkbox (and the
  dropdown's matching enabled state); the stored option does not change.

## Capabilities

### New Capabilities

- `tools/move-ctrl-auto-select`: Ctrl temporarily inverts Move Auto-Select.

## Impact

`tool_move.cpp`, `tools.h`/`tools.cpp` (a `modifiersChanged` signal),
`options_bar_move.cpp`, `tst_options_bars_parity`.
