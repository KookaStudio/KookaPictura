# Spec Delta — document/layer-target

## ADDED Requirements

### Requirement: An empty layer target clears the panel's current row

The Layers panel SHALL project the document's layer target, not hold a second
source of truth. When the user clears the last selected row, the panel SHALL
clear the tree's current index together with the target, so no row keeps an
active highlight while the document has no active layer. The panel SHALL do this
for every deselect gesture, including Ctrl-clicking the last selected row and
clicking the empty space below the rows.

#### Scenario: Deselecting the last row clears the current row [lt_deselect_current]

- **WHEN** the user deselects the only selected layer with Ctrl-click or by
  clicking the empty space below the rows
- **THEN** the document's selected set and active path are empty, the panel's
  selected rows are empty, and the panel has no current row or active highlight
