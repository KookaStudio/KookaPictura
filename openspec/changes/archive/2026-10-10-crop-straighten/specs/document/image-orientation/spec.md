# Spec Delta

## ADDED Requirements

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
