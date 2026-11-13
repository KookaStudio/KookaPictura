# Spec Delta

## ADDED Requirements

### Requirement: Rasterization clears the pixel-data-irrelevant flag

Rasterizing a layer whose shape or fill appearance is baked into its own pixels
SHALL clear the stored pixel-data-irrelevant flag. `rasterize_shape` and
`rasterize_fill_content` (and Rasterize Layer and Rasterize All Layers where
they route through them) SHALL leave the layer's `pixel_data_irrelevant` field
false and SHALL write a layer record with bit 4 clear. The writer SHALL also
leave bit 3 clear on that record. An independent PSD decoder SHALL report the
saved layer with a non-shape kind.

#### Scenario: Rasterizing a shape clears the flag [lm_rasterize_shape_flag]

- **WHEN** a flagged shape layer is rasterized with Rasterize Shape and the document is saved
- **THEN** the layer's pixel-data-irrelevant field is false, its saved record has bit 4 clear, and `psd-tools` reports a non-shape kind

#### Scenario: Rasterizing fill content clears the flag [lm_rasterize_fill_flag]

- **WHEN** a flagged fill-content layer is rasterized with Rasterize Fill Content and the document is saved
- **THEN** the layer's pixel-data-irrelevant field is false and its saved record has bit 4 clear
