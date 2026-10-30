# Spec Delta

## ADDED Requirements

### Requirement: Composited raster sources are cached

The renderer SHALL cache the exact straight-alpha RGBA buffer the CPU composites
for a proxy-less type layer, a smart-filtered smart object, and a channel-less
smart-object source, so the GPU can upload the same pixels instead of
reconstructing them. For a smart-filtered smart object with an enabled filter
mask, the cached buffer SHALL include the per-pixel mix between the filtered
chain output and the unfiltered base that the CPU applies. For a channel-less
smart object, the cached buffer SHALL be the embedded source scaled into the
layer rect. The cached buffer MUST be byte-identical to the uncached
per-composite render, and the cache key SHALL cover every input that can change
its pixels: the type model and rect, the smart-object payload and filter chain,
the filter mask, the font registry and missing-glyph setting, and the document
size.

#### Scenario: A smart-filter mask mix is cached exactly

- **WHEN** a smart-filtered smart object has an enabled filter mask and is
  composited twice
- **THEN** the second composite reuses the mixed buffer and the pixels equal a
  purge-then-composite

#### Scenario: A placed smart source is cached exactly

- **WHEN** a channel-less smart object is composited twice
- **THEN** the second composite reuses the placed buffer and the pixels equal a
  purge-then-composite

#### Scenario: A proxy-less type raster is cached exactly

- **WHEN** a proxy-less type layer is composited twice
- **THEN** the second composite reuses the raster and the pixels equal a
  purge-then-composite

#### Scenario: A filter-mask edit invalidates the buffer

- **WHEN** the smart-filter mask or its enabled flag changes and the document is
  composited again
- **THEN** the buffer is rebuilt and the pixels equal a purge-then-composite

#### Scenario: A payload or text edit invalidates the buffer

- **WHEN** the embedded payload, the type model, or the layer rect changes and
  the document is composited again
- **THEN** the buffer is rebuilt and the pixels equal a purge-then-composite
