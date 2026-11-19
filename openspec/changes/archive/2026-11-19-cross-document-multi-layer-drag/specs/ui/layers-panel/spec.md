# Spec Delta

## MODIFIED Requirements

### Requirement: Drag a layer into another document

A Layers-panel drag SHALL carry the document it started from. A Move-tool
drag on the canvas with no selection SHALL become such a drag of the current
layer target once the pointer reaches another document's tab, leaving the
source layers where they were. While dragging,
hovering another open document's tab SHALL make that document active, and
releasing the drag on that tab or on its canvas SHALL deep-copy the dragged
layer (children, masks, effects, and attributes included) into it. When the
dragged layer belongs to a multi-layer selection in the source document, the
release SHALL deep-copy every selected layer whose ancestor is not also
selected, each exactly once. The copies SHALL be placed by the New Layer
insertion rule relative to the destination's current layer, SHALL keep their
names and document positions, SHALL keep their source stacking order, and SHALL
all become the selected rows with the copy of the dragged layer as the active
row, or the topmost copy when the dragged layer was not copied, all as one
undoable "Duplicate Layer" step. The source document SHALL be
unchanged. A copy of the Background SHALL be an ordinary, unlocked layer. A
drop on the source document's own tab or canvas SHALL copy nothing. A
destination of a different color mode or bit depth SHALL refuse the whole
drop, leaving both documents unchanged. The Layers panel SHALL NOT treat a
drag from another document as a drop on its rows or strip buttons.

#### Scenario: Drop a layer on another document's tab [lpr_drag_to_document]

- **WHEN** layer `Sky` is dragged from one document onto another document's tab and released
- **THEN** that document becomes active and gains a selected layer `Sky` with
  the same opacity in one undo step, and the source keeps its layers

#### Scenario: Move-tool drag from the canvas onto another document's tab

- **WHEN** the Move tool drags from one document's canvas onto another document's tab and releases
- **THEN** that document becomes active and gains a copy of every selected
  layer, or of the active layer when only it is selected, and the source layers
  have not moved

#### Scenario: Drop a layer on another document's canvas

- **WHEN** a layer is dragged from one document and released on the canvas of another, active document
- **THEN** that document gains a copy of the layer

#### Scenario: A mode mismatch refuses the drop

- **WHEN** a layer of an RGB document is released on a Grayscale document's tab
- **THEN** the Grayscale document is unchanged

#### Scenario: The destination panel ignores the foreign drag

- **WHEN** a drag from one document is released on the Layers panel rows after another document became active
- **THEN** the drop is refused and the active document is unchanged

#### Scenario: A multi-layer selection crosses in full [lpr_drag_to_document_multi]

- **WHEN** layers `Sky` and `Sea` are selected in one document, `Sky` is dragged
  onto another document's tab, and the drag is released
- **THEN** that document gains selected layers `Sky` and `Sea` in their source
  stacking order in one undo step, the copy of `Sky` is the active row, and the
  source keeps both layers
