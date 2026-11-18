# Native Depth Save Delta

## MODIFIED Requirements

### Requirement: A dirty high-depth save re-emits its native composite

When a real merged image is requested, the application SHALL, when saving a dirty document whose bit depth was recorded
on read as 16 or 32 in RGB or Grayscale working mode and which has layers,
replace the working composite and the retained composite color planes through
`pictura_render::refresh_native_composite` before encoding, creating the native composite store first if an omitted-image input has none, so the written
composite color planes are the native samples rather than the widened 8-bit
bytes. The raw source-depth composite (`composite_native`) is internal to
`pictura-render`.

#### Scenario: An edited depth-16 layered document saves at 16-bit with native samples

- **WHEN** a depth-16 RGB document with layers and an existing real merged image is edited and saved with Include
- **THEN** the output header declares bit depth 16 and at least one composite
  color sample is not the 8-bit widening of the edited 8-bit composite

#### Scenario: Alpha and extra channels are preserved

- **WHEN** such a document with an existing real merged image is saved with Include
- **THEN** its composite alpha/document extra planes and their channel count are
  unchanged from a save without this feature

### Requirement: Out-of-scope saves are unchanged

Except when an explicit omission or restoration of a missing real merged image is requested, the refresh SHALL NOT mutate a document with no layers, a converted color mode
(`source_mode` is `Some`), a non-16/32-bit depth, or a clean (non-dirty)
document, and saving such a document SHALL be byte-identical to the existing
behavior. Explicit omission SHALL omit the real merged image; Include after an omitted-image input SHALL reconstruct it at native depth even when the document is clean, for supported RGB or Grayscale documents with retained native layer samples.

#### Scenario: A clean open→save is byte-identical

- **WHEN** a 16-bit PSD with a real merged image is opened and saved with Include without any edit
- **THEN** the written bytes match the bytes written before this change

#### Scenario: A no-layer document is not refreshed

- **WHEN** a 16/32-bit document with no layers is saved with its existing output policy after an edit
- **THEN** its merged composite is written as before, without a transparent
  recomposite

#### Scenario: A converted-mode document is not refreshed

- **WHEN** a 16-bit CMYK (`source_mode` CMYK) document is edited and saved with its existing output policy
- **THEN** the retained source planes are not overwritten with working RGB
- **THEN** the existing depth behavior is preserved

#### Scenario: A missing native composite is restored at its original depth [native-depth-restore]

- **WHEN** a layered RGB or Grayscale 16-bit or 32-bit PSD or PSB with retained native layer samples is saved with Never, reopened, and saved with Always
- **THEN** its restored composite has the original native depth and layer-derived native samples
