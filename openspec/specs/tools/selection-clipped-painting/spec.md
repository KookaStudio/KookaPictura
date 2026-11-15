# tools/selection-clipped-painting Specification

## Purpose
The active selection constrains stroke-based painting: a stroke changes only
the pixels its selection covers, on layer pixels and on a layer mask, matching
the selection's role as the document editing mask.

## Requirements

### Requirement: Paint strokes are clipped to the active selection

A stroke-based paint tool SHALL limit the change at each document pixel to the
active selection's coverage there. A coverage-based tool SHALL multiply its dab
coverage by that coverage; a per-dab tool SHALL blend its result toward the
pre-stroke pixel by the same fraction. A pixel the selection does not cover
SHALL NOT change; a partially covered pixel SHALL take the same fraction of the
stroke. A layer smaller than the canvas SHALL be clipped at its document
position, and a layer that grows under the stroke SHALL remain clipped to the
same document pixels. Without an active selection the stroke SHALL behave
exactly as before.

#### Scenario: A brush stroke stops at the selection edge [scp_layer_stroke]

- **WHEN** a selection covers a rectangle and the Brush strokes across it on a
  pixel layer
- **THEN** pixels outside the selection keep their pre-stroke values and
  pixels inside take the brush colour

#### Scenario: A retouch stroke stops at the selection edge [scp_per_dab]

- **WHEN** a selection covers a rectangle and a Dodge stroke crosses it on a
  pixel layer
- **THEN** pixels outside the selection keep their pre-stroke values and
  pixels inside are dodged

#### Scenario: An empty layer grows but stays clipped [scp_growth]

- **WHEN** a selection covers part of the canvas and the Brush strokes across
  it on a newly added empty layer
- **THEN** the layer grows to the dab extent, and only the selected pixels
  show the paint

#### Scenario: A soft selection scales the paint [scp_soft_clip]

- **WHEN** a selection with partial coverage lies under a fully covered dab
- **THEN** the changed pixel moves toward the brush colour by that coverage

#### Scenario: Without a selection the stroke is unchanged [scp_no_selection]

- **WHEN** no selection is active and the Brush strokes across the layer
- **THEN** every dab pixel takes the brush colour as before

### Requirement: Layer-mask strokes are clipped to the active selection

A stroke whose target is a layer mask SHALL be clipped to the active selection
in document coordinates, even though the mask's stroke document is stored at
the mask rectangle's origin. Quick Mask painting SHALL keep its existing
temporary-selection clip.

#### Scenario: Painting a mask only changes selected coverage [scp_mask_stroke]

- **WHEN** a selection is active and the Brush paints black on a reveal-all
  layer mask across the selection boundary
- **THEN** mask coverage inside the selection becomes black, coverage outside
  stays white, and the layer's pixels are untouched

### Requirement: The live preview honours the selection

The reduced-resolution preview and the GPU stroke SHALL present only the
selected pixels. When a selection is active the GPU dab SHALL NOT be used, so
the committed pixels come from the exact clipped CPU stroke.

#### Scenario: A previewed stroke commits clipped pixels [scp_preview]

- **WHEN** a brush large enough to preview strokes across a selection
- **THEN** the preview shows paint only inside the selection, and the committed
  pixels outside the selection are unchanged
