# Spec Delta

## MODIFIED Requirements

### Requirement: Mask refinement edits

The document SHALL provide `set_layer_mask_density`, `set_layer_mask_feather`,
`set_vector_mask_density`, and `set_vector_mask_feather`, each recorded as one
undoable state and each returning false when the mask is absent or the value
is unchanged, and `invert_layer_mask`, which replaces each raster mask sample
`m` inside the document canvas with `255 - m` as one undoable "Invert" state.
Properties-panel Invert SHALL operate over the entire canvas independently of
the active pixel selection, preserving its existing selection-independent
policy. It SHALL use the canvas-mask domain's import-preservation rule before
inverting, preserve all stored off-canvas samples and unrelated mask metadata,
and leave the normalized exterior default black. Undo SHALL restore the exact
prior stored bounds, exterior default, samples, and retained native data;
allocation failure SHALL leave those and history unchanged. Invert SHALL
return false when the mask is absent. Feather MUST reject negative and
non-finite radii.

#### Scenario: Density change undoes

- **WHEN** a mask's density is set to 128 and then undone
- **THEN** the density is 255 again

#### Scenario: Invert flips the mask permanently

- **WHEN** `invert_layer_mask` runs on a mask whose in-canvas pixel is 40 with
  default colour 0
- **THEN** the pixel is 215, the exterior default stays 0, off-canvas stored
  samples are unchanged, and undo restores the prior mask exactly

#### Scenario: Properties Invert retains off-canvas coverage [cmd_properties_invert_retained]

- **WHEN** a mask with partial coverage outside the canvas and a smaller active
  pixel selection is inverted with the Properties-panel Invert button
- **THEN** every in-canvas sample is inverted independently of that selection,
  off-canvas samples including retained native values are unchanged, the
  exterior remains black, layer pixels are untouched, and exactly one "Invert"
  state supports exact undo and redo
