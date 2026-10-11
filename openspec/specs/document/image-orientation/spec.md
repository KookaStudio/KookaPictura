# image-orientation Specification

## Purpose
Exact right-angle and flip remaps, arbitrary rotation with an expanded bounding box, and an oracle.
## Requirements
### Requirement: Exact right-angle and flip remaps

The system SHALL provide `pictura_ops::rotate90_cw`, `rotate90_ccw`,
`rotate180`, `flip_horizontal`, and `flip_vertical`, each taking `&PixelBuffer`
and returning a newly allocated `PixelBuffer` with the input left bit-identical.
The right-angle rotations SHALL be exact index remaps that swap `width` and
`height` where applicable and MUST NOT resample or interpolate: for `rotate90_cw`
a source pixel `(x, y)` maps to `(height - 1 - y, x)`, `rotate90_ccw` maps to
`(y, width - 1 - x)`, and `rotate180` maps to `(width - 1 - x, height - 1 - y)`.
The flips SHALL be exact index mirrors: `flip_horizontal` maps `(x, y)` to
`(width - 1 - x, y)` and `flip_vertical` to `(x, height - 1 - y)`. All five
remaps SHALL preserve every sample bit-for-bit, operate on every channel, and be
reversible by their inverse (or by applying the same op twice for `rotate180`
and the flips). They MUST NOT panic on 1×1, 1-pixel, or non-square buffers.

#### Scenario: 90° clockwise swaps dimensions and remaps exactly

- **WHEN** `rotate90_cw` is applied to a non-square buffer
- **THEN** the output has swapped width and height and every sample equals the source sample at the remapped index

#### Scenario: Quarter turns are reversible

- **WHEN** `rotate90_cw` is followed by `rotate90_ccw` on the same input
- **THEN** the result is bit-identical to the original buffer

#### Scenario: 180° and two flips are reversible

- **WHEN** `rotate180` is applied twice, or `flip_horizontal` is followed by `flip_vertical`
- **THEN** both results are bit-identical to the original buffer

#### Scenario: Odd and tiny dimensions do not panic or shift by one

- **WHEN** the exact remaps are applied to odd-sized and 1×1 buffers
- **THEN** no call panics, dimensions are correct, and no off-by-one sample displacement occurs

### Requirement: Arbitrary rotation with expanded bounding box

The system SHALL provide `pictura_ops::rotate_arbitrary(buf: &PixelBuffer, angle_deg: f64, background: [u8; 4]) -> Result<PixelBuffer, OpsError>`. It SHALL rotate about the source center by `angle_deg` and produce a new buffer sized to the axis-aligned bounding box `W' = W·|cos θ| + H·|sin θ|` and `H' = W·|sin θ| + H·|cos θ|`, rounded up to whole pixels. Sampling SHALL be a bilinear inverse map (source coordinate = inverse-rotated destination coordinate) with clamp-to-edge. Pixels that map outside the source rectangle SHALL be set to `background`, using `background[3]` as alpha for 4-channel buffers. `angle_deg` MUST be finite and within `-359.99..=359.99`; any other value, including NaN or infinity, SHALL be rejected with `OpsError::InvalidParams`. An angle of `0.0` SHALL return a buffer bit-identical to the input (same dimensions, no interpolation change). The function MUST leave `buf` untouched and MUST NOT panic on tiny or non-square inputs.

#### Scenario: Angle 0 is a bit-exact no-op

- **WHEN** `rotate_arbitrary` is called with `angle_deg` 0.0
- **THEN** the output has the source dimensions and samples bit-identical to the input

#### Scenario: The canvas grows to the rotated bounding box

- **WHEN** a non-square buffer is rotated by a non-multiple-of-90 angle
- **THEN** the output dimensions equal the computed bounding box and the original rectangle is not clipped

#### Scenario: Corners outside the source use the background

- **WHEN** an opaque-background buffer is rotated by 45° and a corner pixel is inspected
- **THEN** that pixel equals `background` (including its alpha for a 4-channel buffer)

#### Scenario: Out-of-range and non-finite angles are rejected

- **WHEN** `angle_deg` is 360.0, -359.995, NaN, or infinity
- **THEN** `rotate_arbitrary` returns `OpsError::InvalidParams` and does not panic

#### Scenario: Tiny inputs do not panic

- **WHEN** `rotate_arbitrary` is called on a 1×1 or 1-pixel buffer
- **THEN** no call panics and it returns `Ok` or `OpsError::InvalidParams`

### Requirement: Orientation oracle: exact for right angles, measured for arbitrary

The system SHALL diff the exact remaps against ImageMagick in
`crates/pictura-ops/tests/oracle.rs`: `rotate90_cw`/`rotate90_ccw` against
`-rotate 90`/`-rotate 270`, `rotate180` against `-rotate 180`, and
`flip_horizontal`/`flip_vertical` against `-flop`/`-flip`, all required to match
exactly (zero-tolerance, bit-identical). The `rotate_arbitrary` case SHALL be
diffed against ImageMagick `-filter triangle -rotate <angle> -background <color>`
using `scripts/ops_oracle.py` on the **central region only**, because
ImageMagick pads the rotated bounding box by one row/column per side and its
bounding-box parity shifts the centre by half a pixel; the tolerance SHALL be
justified by the measured maximum and mean per-sample delta, which SHALL be
recorded. An angle whose alignment diverges beyond the recorded tolerance SHALL
be classified no-equivalent with the observed delta recorded (45° measures max
132 and has no faithful alignment). The tests SHALL skip with a message when
`magick` is absent and MUST NOT be marked `#[ignore]`.

#### Scenario: Right-angle rotations and flips match ImageMagick exactly

- **WHEN** `rotate90_cw`/`rotate90_ccw` are compared with `-rotate 90`/`-rotate 270`, `rotate180` with `-rotate 180`, and `flip_horizontal`/`flip_vertical` with `-flop`/`-flip`
- **THEN** no sample differs (zero tolerance)

#### Scenario: Arbitrary rotation is compared on the central region

- **WHEN** `rotate_arbitrary` at 30° is diffed against ImageMagick `-rotate 30` on the central 12×12 region
- **THEN** the result is within the recorded measured tolerance (max 8, mean 0.6), while an angle like 45° is classified no-equivalent with its observed delta recorded

#### Scenario: Missing ImageMagick skips cleanly

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential tests print a skip message and the suite still passes

### Requirement: Document-scope arbitrary rotation

The system SHALL provide `pictura_render::rotate_document_in(doc: &mut Document,
angle_deg: f64, pivot: (f64, f64)) -> bool`. It SHALL resample every layer's
channels and mask about `pivot` using the `pictura_ops::rotate_in` sampling
kernel, expand `doc` dimensions to the axis-aligned bounding box of the rotated
content, and remap every layer rect, mask rect, vector-mask rect, smart-object
bound, and document channel so they stay aligned with their content. It SHALL
support every layer kind, including a Background layer, and SHALL leave `doc`
bit-identical when it returns `false`. It SHALL return `false` for a
non-finite or out-of-range angle, a missing document, or an empty document,
without panic. An angle of `0.0` SHALL leave `doc` unchanged.

#### Scenario: Rotating expands the document [lio_doc_rotate_bbox]

- **WHEN** `rotate_document_in` is called on a non-square layered document with a non-multiple-of-90 angle
- **THEN** the document dimensions equal the rotated content's bounding box and the original content is not clipped

#### Scenario: Every layer kind is carried [lio_doc_rotate_kinds]

- **WHEN** a document containing a Background layer, a pixel layer with a mask, a vector mask, and a type layer is rotated
- **THEN** every layer's rect and payload are remapped consistently and the content stays registered

#### Scenario: A right angle matches the exact remap [lio_doc_rotate_right_angle]

- **WHEN** `rotate_document_in` is called with 90° about the document centre
- **THEN** the result is bit-identical to `rotate_document` with one clockwise quarter turn

#### Scenario: Invalid angle is refused [lio_doc_rotate_refusal]

- **WHEN** `rotate_document_in` is called with a non-finite or out-of-range angle, or without a document
- **THEN** it returns `false`, leaves the document bit-identical, and does not panic

### Requirement: Arbitrary image-rotation entry point

The system SHALL expose `Image > Image Rotation > Arbitrary` in the Image menu
with a numeric angle field. Applying it SHALL rotate the document about its
centre by the entered angle through `rotate_document_in`, recomposite, and record
one history state; a refusal SHALL record nothing. The entered angle SHALL be
validated and a non-finite or out-of-range value SHALL be rejected before any
mutation.

#### Scenario: Arbitrary command rotates and records [lio_arbitrary_command]

- **WHEN** `Image > Image Rotation > Arbitrary` is applied with a valid angle
- **THEN** the document rotates about its centre, the canvas grows to the bounding box, and exactly one history state is added

#### Scenario: Arbitrary command refuses bad input [lio_arbitrary_refusal]

- **WHEN** the command is applied with an invalid or empty angle
- **THEN** the document is unchanged and no history state is added

### Requirement: Document-scope rotation oracle

The system SHALL ship an oracle for `rotate_document_in` in
`crates/pictura-render/tests/document_oracle.rs`. Right-angle calls SHALL be
compared bit-exactly against `rotate_document` (zero tolerance). Arbitrary
angles SHALL be compared against the `pictura_ops::rotate_in` kernel on the
document's composited result, and against ImageMagick `magick -rotate` on the
central region with the tolerance recorded by the existing
`pictura-ops/tests/oracle.rs` case. The oracle SHALL skip with a message when
`magick` is absent and MUST NOT be marked `#[ignore]`.

#### Scenario: Right-angle document rotation is exact [lio_doc_oracle_exact]

- **WHEN** `rotate_document_in` at 90° is compared with `rotate_document`
- **THEN** every document field and sample is bit-identical

#### Scenario: Arbitrary document rotation matches the kernel [lio_doc_oracle_arbitrary]

- **WHEN** `rotate_document_in` at 30° is compared with `rotate_in` at 30° on the same composite
- **THEN** the results match within the recorded tolerance

#### Scenario: Missing ImageMagick skips cleanly [lio_doc_oracle_skip]

- **WHEN** `magick` is not on `PATH`
- **THEN** the differential test prints a skip message and the suite passes

