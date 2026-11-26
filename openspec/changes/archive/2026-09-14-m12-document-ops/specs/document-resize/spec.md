## ADDED Requirements

### Requirement: Document resize entry point and error contract

The system SHALL provide `pictura_render::resize_document(doc: &mut Document, width: u32, height: u32, resample: Resample) -> Result<(), OpsError>`. On success it SHALL set `doc.width` to `width` and `doc.height` to `height` and return `Ok(())`, performing all validation before mutating `doc`. A `width` or `height` below 1 SHALL be rejected with `OpsError::InvalidParams`, and a pixel layer channel or decoded mask whose `data.len()` does not equal the pixel count implied by its `rect` SHALL likewise be rejected with `OpsError::InvalidParams`. On any error `doc` MUST be left bit-identical to its state before the call. The function MUST NOT panic for any input, including a 1×1 document, a document with no layers, and a document with an empty layer channel set.

#### Scenario: A successful resize sets the document dimensions

- **WHEN** `resize_document` is called with `width` and `height` of 1 or greater on a valid document
- **THEN** it returns `Ok(())` and `doc.width` and `doc.height` equal the requested values

#### Scenario: Invalid dimensions are rejected and the document is untouched

- **WHEN** `resize_document` is called with `width` 0 or `height` 0
- **THEN** it returns `OpsError::InvalidParams` and every field of `doc` equals its pre-call value

#### Scenario: A malformed layer channel errors instead of panicking

- **WHEN** a pixel layer's channel or decoded mask `data.len()` does not match the pixel count implied by its `rect`
- **THEN** `resize_document` returns `OpsError::InvalidParams`, leaves `doc` unchanged, and does not panic

#### Scenario: Empty and tiny documents do not panic

- **WHEN** `resize_document` is called on a document with no layers and on a 1×1 document
- **THEN** no call panics and each returns `Ok(())` or `OpsError::InvalidParams`

### Requirement: Recursive resample of pixel layers, masks, and groups

`resize_document` SHALL traverse the layer tree and, for every pixel layer (a non-group layer without an adjustment), SHALL resample each of its channels — the color channels and the `-1` transparency channel — through `pictura_ops::resize` with the requested `Resample` kernel, producing the layer's scaled bounds. It SHALL resample a decoded `LayerMask`'s `data` the same way when the mask has data, producing the mask's scaled bounds. It SHALL scale every layer `rect` and mask `rect` to the document scale factors `width / old_width` and `height / old_height`, mapping each rect edge with round-to-nearest so adjacent edges stay adjacent. A group SHALL recurse through its `children` and SHALL resample its own mask when present. An adjustment layer carries no color channels, so only its mask (when present) SHALL be resampled. Every layer's `name`, `blend`, `opacity`, `clipping`, `visible`, and `adjustment` payload MUST be preserved unchanged.

#### Scenario: A pixel layer's channels are resampled to its scaled bounds

- **WHEN** a document with a full-canvas RGBA pixel layer is resized to half its width and height
- **THEN** each of the layer's channels has the scaled pixel count and the layer `rect` width and height are halved

#### Scenario: An adjustment layer resamples only its mask

- **WHEN** a document containing an adjustment layer with a mask is resized
- **THEN** the adjustment layer's `channels` remain empty, its `adjustment` payload is unchanged, and its mask `data` matches the scaled mask `rect`

#### Scenario: Groups are traversed recursively

- **WHEN** a group contains a nested pixel layer and the document is resized
- **THEN** the nested layer's channels and `rect` are resampled, not only the top-level layers

#### Scenario: Layer metadata is preserved

- **WHEN** a document whose layers carry a name, a non-Normal blend, partial opacity, clipping, hidden visibility, a mask, and an adjustment is resized
- **THEN** every layer's `name`, `blend`, `opacity`, `clipping`, `visible`, and `adjustment` are bit-identical to the input

### Requirement: Document-level channels are resampled

`resize_document` SHALL resample every entry of `doc.channels` (the document-level saved-selection and spot channels) through `pictura_ops::resize` with the requested `Resample` kernel to the new document size, preserving each channel's `id`. A document with no extra channels SHALL resample them as a no-op and still succeed.

#### Scenario: Extra document channels are resampled

- **WHEN** a document carrying a document-level channel at the old document size is resized
- **THEN** the channel's `data.len()` equals the new `width * height`, its `id` is unchanged, and its samples are a resample of the source channel

#### Scenario: A document with no extra channels still resizes

- **WHEN** `resize_document` is called on a document whose `channels` vector is empty
- **THEN** it returns `Ok(())` with the new dimensions

### Requirement: Composite recomputation after document resize

After a successful `resize_document`, `doc.composite` SHALL be replaced by `pictura_render::composite_rgba(doc)`: a 4-channel (R,G,B,A) buffer at the new document size. The composite MUST be recomputed from the resampled layer tree rather than resampled independently, so that `doc.composite` always equals `composite_rgba(doc)` immediately after the call.

#### Scenario: The cached composite equals a fresh composite after resize

- **WHEN** `resize_document` succeeds
- **THEN** `doc.composite` is bit-identical to `composite_rgba(&doc)` and has the new height and width with 4 channels

### Requirement: Document resize oracle

The system SHALL ship an oracle for `resize_document` in `crates/pictura-render/tests/document/document_oracle.rs` that (a) writes the resized document with `pictura_codec::write_psd`, re-reads it with `pictura_codec::read_psd`, and confirms the structural round-trip matches the resized document, (b) opens the written file with the independent `psd-tools` library and confirms the reported document dimensions and layer count match the resized document, and (c) asserts `doc.composite == composite_rgba(&doc)`. The oracle SHALL skip with a message when `psd-tools` is not importable and MUST NOT be marked `#[ignore]`.

#### Scenario: The resized document round-trips structurally

- **WHEN** a layered document is resized and written, then re-read by `pictura_codec::read_psd`
- **THEN** the re-read document has the new dimensions and every layer's `rect` and channel lengths match the resized document

#### Scenario: psd-tools sees the resized document

- **WHEN** the written PSD is opened with `psd-tools`
- **THEN** psd-tools reports the new document width, height, and layer count

#### Scenario: Missing psd-tools skips cleanly

- **WHEN** `psd-tools` is not importable
- **THEN** the oracle prints a skip message and the suite still passes
