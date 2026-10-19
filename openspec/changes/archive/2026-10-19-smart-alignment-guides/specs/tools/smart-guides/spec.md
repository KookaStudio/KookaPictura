## ADDED Requirements

### Requirement: Alignment guides while moving content

While the Move tool drags a layer, or the Type tool drags text, the canvas SHALL
draw magenta alignment guides and snap the moving content when one of its
content-box edges or centres (left, horizontal centre, right; top, vertical
centre, bottom) comes within eight screen pixels of the document's matching edge
or centre, or of another visible layer's matching content-box edge or centre.
Snapping SHALL apply the smallest offset that aligns the edge; when the document
and a layer are equally close, the layer alignment SHALL win. The moving layer
and its descendants SHALL NOT be used as references, and groups and adjustment
layers SHALL contribute no reference box. The guides SHALL clear when the drag
ends or when no alignment is active.

#### Scenario: Snap to the document centre [sg_document_centre]

- **WHEN** `tst_smart_guides::documentCentre` drags a layer's content centre to within eight screen pixels of the document centre
- **THEN** a magenta guide is drawn at the document centre and, on release, the layer's content centre equals the document centre

#### Scenario: Snap to another layer's edge [sg_layer_edge]

- **WHEN** `tst_smart_guides::layerEdge` drags one layer's left content edge to within eight screen pixels of another layer's left content edge
- **THEN** a magenta vertical guide is drawn at that x and, on release, the two left edges coincide

#### Scenario: No snap beyond the threshold [sg_threshold]

- **WHEN** `tst_smart_guides::threshold` drags the same edge just outside eight screen pixels of the target
- **THEN** no guide is drawn and the layer keeps the pointer's offset

#### Scenario: Guides clear on release [sg_clears]

- **WHEN** `tst_smart_guides::documentCentre` releases the drag
- **THEN** the canvas reports no smart guides

#### Scenario: The Type tool's text move snaps too [sg_type_move]

- **WHEN** `tst_smart_guides::typeMove` drags live text so its ink box centre approaches the document centre
- **THEN** the guide is drawn during the drag and the committed type layer's ink box is centred

### Requirement: Gap distance readout

While a smart guide is active, the canvas SHALL show the distance between the
moving content box and the reference content box along the guide's axis, in the
rulers' unit, near the guide.

#### Scenario: The gap reads in the rulers' unit [sg_gap]

- **WHEN** `tst_smart_guides::gapReadout` aligns a layer to another layer that sits 36 px below it at 72 ppi with the rulers in inches
- **THEN** the guide's gap label reads `0.500 in`

### Requirement: Smart guide geometry

The smart-guide solver SHALL choose the smallest alignment offset per axis
within the threshold, prefer the earlier reference on ties, then the document,
and report every coincident guide line after snapping.

#### Scenario: Two coincident guides [sg_two_lines]

- **WHEN** `tst_smart_guides::twoLines` snaps a layer so its left edge meets a reference's left edge and its centre meets that reference's right edge
- **THEN** two vertical guide lines are reported at the two aligned positions
