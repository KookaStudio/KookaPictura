# native-depth-save Specification

## Purpose
Re-emits the native composite on a dirty high-depth save and leaves out-of-scope saves unchanged.

## Requirements

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

### Requirement: Out-of-scope saves are unchanged

The refresh SHALL NOT mutate a document with no layers, a converted color mode
(`source_mode` is `Some`), a non-16/32-bit depth, or a clean (non-dirty)
document, and saving such a document SHALL be byte-identical to the existing
behavior.

#### Scenario: A clean open→save is byte-identical

- **WHEN** a 16-bit PSD is opened and saved without any edit
- **THEN** the written bytes match the bytes written before this change

#### Scenario: A no-layer document is not refreshed

- **WHEN** a 16/32-bit document with no layers is saved after an edit
- **THEN** its merged composite is written as before, without a transparent
  recomposite

#### Scenario: A converted-mode document is not refreshed

- **WHEN** a 16-bit CMYK (`source_mode` CMYK) document is edited and saved
- **THEN** the retained source planes are not overwritten with working RGB
- **THEN** the existing depth behavior is preserved
