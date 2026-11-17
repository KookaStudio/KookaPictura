## ADDED Requirements

### Requirement: Fit coverage to a different document

`pictura-select` SHALL provide `Selection::fit_centered(target_width,
target_height) -> Selection`. Same-size input SHALL return an identical
selection. Otherwise the method SHALL scale the source plane by
`min(target_width / source_width, target_height / source_height)`, preserve
aspect ratio, centre the fitted plane in the target, and fill the uncovered
border with 0. Scaling SHALL sample the source bilinearly and SHALL NOT
threshold soft coverage. A zero-sized source or target SHALL yield an empty
selection of the requested size.

#### Scenario: Same-size fit is the identity [sc_fit_same_size]

- **WHEN** a selection is fitted to its own dimensions
- **THEN** every byte equals the source selection

#### Scenario: A wide source is letterboxed [sc_fit_letterbox]

- **WHEN** an all-selected 8×4 selection is fitted to 8×8
- **THEN** rows 2–5 are selected and rows 0–1 and 6–7 are empty

#### Scenario: A small source upscales [sc_fit_upscale]

- **WHEN** an all-selected 2×2 selection is fitted to 4×4
- **THEN** the result is all selected

### Requirement: Cross-document selection drag

Dragging a live selection outline with a selection tool from one document to
another document's tab or canvas SHALL copy the selection into the destination.
The source SHALL keep its selection and SHALL NOT record history. The
destination SHALL receive the source coverage fitted with
`Selection::fit_centered` and SHALL record exactly one `Load Selection` state.
A same-size destination SHALL receive the coverage exactly. Dropping on the
source document or outside a document SHALL do nothing. The Move tool and
Ctrl-drag content moves SHALL NOT use this path.

#### Scenario: Same-size drag copies exactly [sc_drag_same_size]

- **WHEN** a selection outline is dragged onto a same-size document
- **THEN** the target selection bytes equal the source selection bytes and the
  target gains one `Load Selection` state

#### Scenario: Different-size drag fits and centres [sc_drag_different_size]

- **WHEN** a selection outline is dragged onto a different-size document
- **THEN** the target selection is the fitted, centred source coverage

#### Scenario: The source keeps its selection [sc_drag_source_unchanged]

- **WHEN** a selection outline is dragged to another document
- **THEN** the source selection bytes and history are unchanged

#### Scenario: A drop on the source is ignored [sc_drag_same_doc_ignored]

- **WHEN** a selection outline is dropped on its own document
- **THEN** the source selection and history are unchanged and the target gains
  nothing
