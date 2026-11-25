# tools/selection-content-move Specification

## MODIFIED Requirements

### Requirement: Move selected pixels
When a selection is active, the Move tool SHALL drag the selected pixels rather
than the whole layer: the covered pixels are cleared from the source layer and
re-created at the dragged offset inside that same layer, and the selection mask
moves with them by the same offset. With no active selection, the Move tool
SHALL continue to move the whole layer. When the active target cannot take a
content move — a type or shape layer, a hidden target, or a layer whose
position, pixel, or cut-transparency lock refuses it — the Move tool SHALL move
the whole layer instead, except that a position lock SHALL refuse the press
before any preview or history state. The preflight SHALL agree with the engine's
refusals, so a content move the engine would refuse never moves the selection
outline first.

#### Scenario: Drag moves the selected pixels
- **WHEN** a selection is active and the Move tool is dragged by `(dx, dy)`
- **THEN** the selected pixels appear offset by `(dx, dy)`, the source area is cleared, and the layer count is unchanged

#### Scenario: The selection follows the pixels
- **WHEN** the Move tool commits a selected-pixel drag by `(dx, dy)`
- **THEN** the active selection mask is translated by `(dx, dy)`

#### Scenario: No selection still moves the layer
- **WHEN** no selection is active and the Move tool is dragged
- **THEN** the whole layer moves, as before

#### Scenario: A type layer moves as a whole under a selection [scm_type_whole]
- **WHEN** a selection is active over a type layer and the Move tool drags it
- **THEN** the whole type layer moves with the drag, the selection stays, and one `Move Layer` state is recorded

#### Scenario: A position-locked target refuses cleanly [scm_locked_clean]
- **WHEN** a selection is active over a position-locked layer and the Move tool is pressed and dragged
- **THEN** no preview starts, the layer's pixels and rect are unchanged, and no history state is added
