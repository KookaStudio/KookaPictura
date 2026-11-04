# Spec Delta

## MODIFIED Requirements

### Requirement: A dirty high-depth save re-emits its native composite

The application SHALL, when saving a dirty document whose bit depth was recorded
on read as 16 or 32 in RGB or Grayscale working mode and which has layers,
replace the working composite and the retained composite color planes through
`pictura_render::refresh_native_composite` before encoding, so the written
composite color planes are the native samples rather than the widened 8-bit
bytes. The raw source-depth composite (`composite_native`) is internal to
`pictura-render`.

#### Scenario: An edited depth-16 layered document saves at 16-bit with native samples

- **WHEN** a depth-16 RGB document with layers is edited and saved
- **THEN** the output header declares bit depth 16 and at least one composite
  color sample is not the 8-bit widening of the edited 8-bit composite

#### Scenario: Alpha and extra channels are preserved

- **WHEN** such a document is saved
- **THEN** its composite alpha/document extra planes and their channel count are
  unchanged from a save without this feature
