# Proposal

## Why

Deselecting the last layer in the Layers panel empties the document's layer
target but leaves the tree's current row on the deselected layer (#835). The row
keeps an active highlight while the document has no active layer, so the panel
shows a state the document does not have. A layer is either selected and active,
or neither.

The panel's tree selection is a projection of the Rust target, never a second
source of truth. `writeTarget()` clears the target when the rows are empty but
does not clear the tree's current index, and only the app-to-panel direction
(`setTreeSelection`) clears it.

## What Changes

- When the user clears the last selected row, the panel clears the tree's
  current index together with the target.
- Clicking the empty space below the rows and Ctrl-clicking the last selected
  row both leave no current row and no active highlight.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `document/layer-target`: a new requirement makes an empty layer target clear
  the panel's current row as well as its selected rows.

## Impact

- `crates/pictura-app/cpp/panels/layers_panel_target.cpp` — `writeTarget()`
  clears the current index when the target empties.
- `crates/pictura-app/cpp/tests/tst_layer_target.cpp` — two Qt Test cases for
  the Ctrl-click and empty-space gestures.
- No dependency, data-format, API, or history changes.

Closes #835.
