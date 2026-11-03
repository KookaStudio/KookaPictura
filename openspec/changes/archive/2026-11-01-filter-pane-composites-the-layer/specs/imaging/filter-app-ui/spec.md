## ADDED Requirements

### Requirement: Reduced whole-picture filter previews composite like the commit

The Shear pane and the Blur Gallery canvas SHALL render a reduced copy of the document with the filter applied to the active layer, composited as the document is, so the layer's blend mode, Blend If, opacity, fill, raster mask and vector mask apply, and the selection SHALL gate the filter as it gates the commit. The other layers SHALL stay unfiltered. The filter SHALL start from the pixels before any open canvas preview.

#### Scenario: The blend mode shows [pane_blend_mode]

- **WHEN** the preview runs on a Multiply layer over another layer
- **THEN** it shows the multiplied colours, equal at scale 1 to the composite of the committed filter

#### Scenario: Blend If shows [pane_blend_if]

- **WHEN** the preview runs on a layer whose Blend If hides part of it
- **THEN** that part shows the layer below, equal at scale 1 to the composite of the committed filter

#### Scenario: The vector mask shows [pane_vector_mask]

- **WHEN** the preview runs on a layer with a vector mask hiding part of it
- **THEN** the hidden part shows the layer below at the mask's position in the reduced picture

#### Scenario: The selection gates the filter [pane_selection]

- **WHEN** the preview runs with a selection
- **THEN** pixels outside the selection stay unfiltered, as the commit leaves them

#### Scenario: An open preview does not compound [pane_open_preview]

- **WHEN** a canvas preview is open on the active layer
- **THEN** the pane equals the pane of the same filter with no preview open
