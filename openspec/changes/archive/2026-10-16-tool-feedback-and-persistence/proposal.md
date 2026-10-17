# Proposal

## Why

Owner review of #541's paragraph-type and caret work found six gaps in the
type and Move workflows:

1. Show Transform Controls should start checked.
2. The black caret disappears over a dark image; CS6 draws it white there.
3. Tool settings reset on every launch; the options bar should survive a restart.
4. New type layers show a padlock in their Layers row.
5. and 6. A Move drag gives no feedback: CS6 (and CC) draw marching ants round
   the moving content and a ΔX / ΔY readout in the configured units.

All six ship on the #541 branch by owner decision.

## What Changes

- **Show Transform Controls** is on by default. This is an owner decision:
  CS6 ships it off.
- **Caret contrast:** the caret, baseline guides, and placement point draw
  white when the image under the caret is dark (mean luminance below half),
  and black otherwise.
- **Tool options persist:** every tool's options-bar settings are saved in the
  session store (`toolOptions`, keyed by option group) on close, on quit, on a
  tool switch, and on Move / Type / Shape option edits. They are restored at
  launch. The Crop options keep their existing keys. Clone Source points and
  values that mirror the document (the shape W / H readout, the text colour,
  the mixer reservoir) are not settings and are not saved.
- **Forced locks have no row badge:** a type or shape layer keeps its forced
  Transparency and Image locks (the Lock strip still shows them, per
  `docs/02-ui-ux/panels/layers-panel.md`), but its row shows a padlock only for
  locks beyond those.
- **Move drag feedback:** a Move drag outlines the moving layer's content box
  with marching ants that follow the move, and shows a two-line readout
  (`ΔX: 0.500 in` / `ΔY: 0.250 in`) by the pointer, in the rulers' unit and the
  document's resolution.
- **Idle controls box:** the box is drawn round the layer's content without a
  transform session; the session (and its trim of the layer to the box) starts
  only when a handle is grabbed. With the box on by default it is up whenever
  the Move tool is, and the old session-per-box trimmed every selected layer,
  clipping any filter run while it showed.
- **Controls box fixes** that the new default exposed: rebuilding the passive
  controls box no longer takes keyboard focus from a text field (only the
  modal Ctrl+T session does), opening it no longer trims a type layer's rect
  (which shifted the text on its next edit), dismissing it without a transform
  pads a trimmed layer back to its extent (or every Move-tool selection would
  shrink layers and clip later filters), and Space still pans over it.
- **BREAKING**: none.

## Capabilities

- Added `tools/move-drag-feedback`, `ui/tool-options-persistence`.
- Added requirements to `tools/type-caret` and `tools/move-transform-controls`;
  modified `tools/move-transform-controls`: *Transform controls box on the
  active layer* (the idle box).
- Modified `ui/layers-panel`: *Layer row badges and delegate* (forced locks show
  no badge).
