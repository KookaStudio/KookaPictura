# document-orientation Specification

## Purpose
Document rotation and flip entry points with an exact recursive orientation remap and an oracle.

## Requirements

### Requirement: Document rotation entry point and validation

The system SHALL provide `pictura_render::rotate_document(doc: &mut Document, quarter_turns: u8) -> Result<(), OpsError>`. `quarter_turns` 1 SHALL rotate the document 90° clockwise, 2 SHALL rotate it 180°, and 3 SHALL rotate it 90° counter-clockwise. Any other value, including 0 and values of 4 or greater, SHALL be rejected with `OpsError::InvalidParams` before any mutation, leaving `doc` bit-identical to its state before the call. The function MUST NOT panic for any input, including 1×1 documents and empty layer stacks.

#### Scenario: Quarter turns one, two, and three are accepted

- **WHEN** `rotate_document` is called with `quarter_turns` 1, 2, or 3
- **THEN** it returns `Ok(())` and the document is rotated by the corresponding turn

#### Scenario: Out-of-range quarter turns are rejected untouched

- **WHEN** `rotate_document` is called with `quarter_turns` 0 or 4 or greater
- **THEN** it returns `OpsError::InvalidParams` and every field of `doc` equals its pre-call value

#### Scenario: Tiny and empty documents do not panic

- **WHEN** `rotate_document` is called on a 1×1 document and on a document with no layers
- **THEN** no call panics and each returns `Ok(())` or `OpsError::InvalidParams`

### Requirement: Document flip entry point

The system SHALL provide `pictura_render::flip_document(doc: &mut Document, horizontal: bool)`. When `horizontal` is `true` it SHALL mirror the document along the vertical axis (the `pictura_ops::flip_horizontal` mapping); when `horizontal` is `false` it SHALL mirror along the horizontal axis (the `pictura_ops::flip_vertical` mapping). It MUST NOT panic for any input, including 1×1 documents and empty layer stacks.

#### Scenario: Horizontal and vertical flips are selectable

- **WHEN** `flip_document` is called with `horizontal` true and with `horizontal` false
- **THEN** the first mirrors pixels about the vertical axis and the second about the horizontal axis

### Requirement: Exact recursive orientation remap

`rotate_document` and `flip_document` SHALL apply an exact index remap with no
interpolation to every layer channel, every decoded `LayerMask` `data`, and
every entry of `doc.channels`, and SHALL recurse through group `children`. The
mapping SHALL be the corresponding M10 remap: `rotate90_cw` for `quarter_turns`
1, `rotate180` for 2, `rotate90_ccw` for 3, and `flip_horizontal` /
`flip_vertical` for `flip_document`. Every layer `rect` and mask `rect` SHALL be
remapped consistently with the pixel mapping. A 90° or 270° turn SHALL swap
`doc.width` and `doc.height`, while 180° and the flips SHALL preserve them.
Adjustment layers carry no color channels and SHALL contribute no channel data
while their mask is remapped when present. Every sample MUST be copied
bit-for-bit and no resampling SHALL occur.

#### Scenario: A 90° clockwise turn swaps dimensions and remaps exactly

- **WHEN** `rotate_document` is called with `quarter_turns` 1 on a non-square layered document
- **THEN** `doc.width` and `doc.height` swap, every channel sample equals the source sample at the clockwise-remapped index, and every layer `rect` is the remapped rectangle

#### Scenario: A 90° counter-clockwise turn swaps dimensions and remaps exactly

- **WHEN** `rotate_document` is called with `quarter_turns` 3 on a non-square document
- **THEN** `doc.width` and `doc.height` swap and a clockwise turn followed by a counter-clockwise turn restores the original samples

#### Scenario: A 180° turn and the flips preserve dimensions

- **WHEN** `rotate_document` is called with `quarter_turns` 2, or `flip_document` is called with `horizontal` true or false
- **THEN** `doc.width` and `doc.height` are unchanged and every channel is an exact index mirror of the input

#### Scenario: Nested layers and masks are remapped

- **WHEN** a group contains a nested masked pixel layer and the document is rotated or flipped
- **THEN** the nested layer channel `data` and mask `data` are remapped by the same index mapping and the nested `rect` and mask `rect` are remapped consistently

#### Scenario: Document-level channels are remapped

- **WHEN** a document carrying a document-level channel is rotated 90°
- **THEN** the channel's dimensions swap and its samples equal the source at the remapped index

### Requirement: Orientation exactness and identity properties

`rotate_document` and `flip_document` SHALL satisfy the exactness identities:
because the remaps are exact integer index copies, applying `rotate_document`
with `quarter_turns` 1 four times SHALL return the document to a state
bit-identical to the original, applying `quarter_turns` 2 twice SHALL be the
identity, applying a 90° clockwise turn followed by a 90° counter-clockwise turn
SHALL be the identity, and applying the same `flip_document` twice SHALL be the
identity. These identities MUST hold for the document dimensions, every layer
and mask `rect`, and every channel sample.

#### Scenario: Four 90° turns are the identity

- **WHEN** `rotate_document` is called with `quarter_turns` 1 four times
- **THEN** the resulting `doc.width`, `doc.height`, layer rects, mask rects, and every channel sample are bit-identical to the original

#### Scenario: Two 180° turns and two identical flips are the identity

- **WHEN** `rotate_document` is called with `quarter_turns` 2 twice, or `flip_document` is called twice with the same `horizontal` value
- **THEN** the result is bit-identical to the original document

#### Scenario: Clockwise then counter-clockwise is the identity

- **WHEN** `rotate_document` with `quarter_turns` 1 is followed by `rotate_document` with `quarter_turns` 3
- **THEN** the result is bit-identical to the original document

### Requirement: Composite recomputation after document orientation

After a successful `rotate_document` or `flip_document`, the document's cached `doc.composite` SHALL be left byte-identical to its pre-call value; the operation MUST NOT run the compositor. The app's edit transaction SHALL replace it with `pictura_render::composite_rgba(doc)` at the resulting document size before displaying, capturing history, or saving, so the composite is computed from the remapped layer tree rather than resampled independently. When the document has no layers and a merged composite whose dimensions match the document, the operation SHALL first materialize that composite as a Background layer, preserving its alpha, so the orientation transforms those pixels.

#### Scenario: The cached composite equals a fresh composite after orientation

- **WHEN** the app commits a rotate or flip through the edit transaction
- **THEN** `doc.composite` is bit-identical to `composite_rgba(&doc)` and has the resulting dimensions

#### Scenario: The engine leaves the composite and the app materializes it after orientation [do_engine_then_app]

- **WHEN** the app commits a rotate or flip through the edit transaction
- **THEN** the engine left `doc.composite` byte-identical to its pre-call value, and the transaction's materialized composite is bit-identical to `composite_rgba(&doc)` at the resulting dimensions

#### Scenario: A merged-only document orientation keeps its pixels [do_merged_only]

- **WHEN** `rotate_document` or `flip_document` is called on a document with no layers and a matching merged composite
- **THEN** the composite is materialized as a Background layer before the operation, the transformed pixels are not stale or blank, and the result displays and saves with those pixels

### Requirement: Document orientation oracle

The system SHALL ship an oracle for `rotate_document` and `flip_document` in
`crates/pictura-render/tests/document_oracle.rs` that (a) asserts the engine left
the cached composite untouched, then materializes it with
`pictura_render::composite_rgba(doc)`, (b) writes the rotated or flipped document
with `pictura_codec::write_psd`, re-reads it with `pictura_codec::read_psd`, and
confirms the structural round-trip, (c) opens the written file with the
independent `psd-tools` library and confirms the reported document dimensions and
layer count match the remapped document, and (d) asserts the materialized
composite equals a fresh `composite_rgba(&doc)`. The oracle SHALL skip with a
message when `psd-tools` is not importable and MUST NOT be marked `#[ignore]`.

#### Scenario: The rotated document round-trips structurally

- **WHEN** a layered document is rotated and written, then re-read by `pictura_codec::read_psd`
- **THEN** the re-read document has the swapped dimensions and every layer `rect` matches the remapped rect

#### Scenario: psd-tools sees the rotated and flipped document

- **WHEN** the written PSD is opened with `psd-tools`
- **THEN** psd-tools reports the remapped document dimensions and layer count

#### Scenario: Missing psd-tools skips cleanly

- **WHEN** `psd-tools` is not importable
- **THEN** the oracle prints a skip message and the suite still passes
