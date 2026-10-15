# filter-app-ui Specification (delta)

## MODIFIED Requirements

### Requirement: Convert for Smart Filters command

`Filter ▸ Convert for Smart Filters` SHALL convert the active raster layer into
a smart object so that the layer's content is carried as an embedded source,
while leaving the layer's pixels unchanged. It SHALL be enabled only when the
active layer can be converted to a smart object, which now includes the
Background layer; converting a Background SHALL unlock it first. On success it
SHALL recomposite and record one history state named `Convert for Smart Filters`.

#### Scenario: Converting preserves the pixels and records one state

- **WHEN** `Convert for Smart Filters` is invoked on a convertible raster layer
- **THEN** the layer becomes a smart object with the same composited pixels and exactly one history state is recorded

#### Scenario: A Background converts and is unlocked

- **WHEN** `Convert for Smart Filters` is invoked on the Background layer
- **THEN** the layer becomes a smart object, is no longer the Background, its pixels are unchanged, and exactly one history state is recorded

#### Scenario: Refused when not convertible

- **WHEN** `Convert for Smart Filters` is invoked on a group layer or an adjustment layer
- **THEN** the operation returns `false`, the history count is unchanged, and the document is unchanged
