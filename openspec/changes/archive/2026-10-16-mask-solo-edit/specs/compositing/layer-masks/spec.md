# compositing/layer-masks Specification (delta)

## ADDED Requirements

### Requirement: Loading a layer mask as a selection

The system SHALL build a document-sized pixel selection from a raster layer mask's
coverage, following the mask's own coverage rule: the mask's stored coverage
inside its rectangle, its `default_color` outside it, and fully selected where the
mask is disabled or carries no data. White (non-zero coverage) SHALL mean selected,
black unselected, grey partial, matching `docs/08-selection/selection-model.md`.
A path that does not resolve, or a layer with no raster mask, SHALL report no
selection. A selection built from a mask SHALL be combined into the current
selection (`Add`, `Subtract`, `Intersect`) or replace it (`New`) in one undoable
step.

#### Scenario: The mask coverage becomes the selection

- **WHEN** a mask that is white in its left half and black in its right half is
  loaded as a selection
- **THEN** the left half of the document is selected and the right half is not

#### Scenario: A disabled mask loads as fully selected

- **WHEN** a disabled mask is loaded as a selection
- **THEN** the whole document is selected

#### Scenario: A layer without a mask reports no selection

- **WHEN** a layer with no raster mask is asked for its mask selection
- **THEN** no selection is produced and the current selection is unchanged

#### Scenario: Loading combines with the current selection

- **WHEN** a mask is loaded as a selection with the `Subtract` mode
- **THEN** the mask's white areas are removed from the current pixel selection
