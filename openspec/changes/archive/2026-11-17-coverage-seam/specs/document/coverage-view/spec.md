# Spec Delta

## Purpose

Provides one public engine interface for reading editing coverage from
selections and stored masks, preserving document coordinates, fractional
values, sparse storage, and existing editing behavior across consumers.

## ADDED Requirements

### Requirement: Shared coverage input

The engine SHALL expose one coverage value that downstream editing operations
can consume independently of its source. Selection, alpha-channel, Quick Mask,
and raw raster-mask coverage SHALL be projectable into this value without
attaching layer-mask metadata or changing stored bytes. Each sample SHALL
retain its exact 8-bit value, including partial coverage. An absent optional
editing constraint SHALL mean unrestricted editing; a present zero-coverage
constraint SHALL protect every pixel. Quick Mask's implicit-full state SHALL
remain distinct from its independently active editing constraint.

#### Scenario: Coverage sources preserve their values [cov_sources]

- **WHEN** a selection, alpha channel, Quick Mask, and raster-mask source with values 0, 1, 64, 128, 254, and 255 are exposed as coverage
- **THEN** consumers read those exact values without changing the sources or adding mask metadata to a selection

#### Scenario: Absent and empty constraints differ [cov_none_empty]

- **WHEN** the same supported edit runs with no constraint, an all-zero constraint, and a partially covered constraint
- **THEN** the first uses its existing unrestricted behavior, the second changes nothing, and the third uses the existing fractional blending rule

#### Scenario: Quick Mask retains its independent selection clip [cov_quick_mask]

- **WHEN** an implicit-full Quick Mask is edited with a temporary partial selection and the edit is undone and redone
- **THEN** only selected Quick Mask coverage changes, the temporary selection remains independent, and history restores the exact prior and resulting coverage

### Requirement: Coordinate-safe coverage reads

Coverage SHALL carry signed, half-open bounds in document coordinates and an
explicit exterior value. Sampling inside the stored bounds SHALL return the
corresponding stored byte; sampling outside SHALL return the exterior value.
Selection and alpha-channel projections SHALL use zero exterior coverage.
Row reads SHALL initialize every requested output sample, including exterior
parts, and agree with point sampling. Rebasing to an operation's local origin
SHALL preserve storage and samples while translating their coordinates.
Invalid storage geometry or an unrepresentable coordinate transformation
SHALL be reported without modifying source data or an edit destination.

#### Scenario: Offset bounds and exterior samples [cov_coordinates]

- **WHEN** coverage with negative and positive document offsets is sampled on every edge and outside its bounds, using exterior values 0, 128, and 255
- **THEN** interior samples equal the stored bytes and exterior samples equal the specified exterior value without row wrapping

#### Scenario: Row reads agree with point samples [cov_rows]

- **WHEN** rows crossing the stored bounds and tile edges, and rows entirely outside coverage, are read into prefilled output buffers
- **THEN** every output byte equals point sampling at that document coordinate and no previous output value leaks through

#### Scenario: A local edit rebases without copying coverage [cov_rebase]

- **WHEN** a selection view is rebased for an offset mask-local edit
- **THEN** each local coordinate reads the original document-coordinate byte, the source storage is shared without a whole-selection copy, and source coordinates and bytes are unchanged

#### Scenario: Invalid geometry and translation are refused [cov_invalid]

- **WHEN** a coverage value is requested with mismatched plane dimensions or sample count, or a rebase would exceed the signed coordinate range
- **THEN** the request reports failure without panic, materialization, or mutation of sources and destinations

### Requirement: Sparse coverage access

Point reads, row reads, bounds queries, uniform-region queries, and rebasing
SHALL support flat and sparse tiled storage without materializing a tiled
plane. Uniform regions SHALL be answerable without expanding uniform or empty
tiles into dense storage. Consumers SHALL retain their existing ability to
skip zero-covered regions and process uniform regions without scanning every
pixel. Coverage reads SHALL NOT leave a cached contiguous source mirror.

#### Scenario: Uniform queries include the exterior [cov_uniform]

- **WHEN** uniform, empty, mixed, and partially exterior regions are queried on flat and tiled versions of the same coverage
- **THEN** both versions return the same uniform value or mixed result, constant tiles stay constant, and neither version creates a contiguous tiled mirror

### Requirement: Renderer edits accept coverage independently of its source

Selection-constrained filters, adjustments, Apply Image, matting, reduced
filter previews, selected-content moves, Layer via Copy/Cut, and
selection-driven mask creation SHALL consume the shared coverage interface.
They SHALL preserve their existing pixel results, coordinate placement,
refusals, and transparency policies without requiring a synthetic layer mask.

#### Scenario: Renderer operations share the coverage input [cov_render]

- **WHEN** the supported renderer operation families receive the same soft coverage view over an offset pixel layer
- **THEN** each accepts that interface, agrees with its existing expected pixel and refusal results, and leaves uncovered pixels unchanged according to its existing policy

### Requirement: Paint and healing accept shared coverage

Selection inputs to fills, gradients, bucket operations, erasing, stroke
clipping, Patch, and Content-Aware Move SHALL consume the shared coverage
interface. Layer growth and mask-local editing SHALL retain their existing
document-coordinate clipping. Healing SHALL preserve its existing source and
destination coverage interpretation; solver work arrays are not coverage
source materializations.

#### Scenario: Paint retains clipping and uniform processing [cov_paint]

- **WHEN** fill, gradient, bucket, eraser, and stroke edits cross soft selection bounds on offset and growing layers, and on a mask-local target
- **THEN** their results retain existing fractional clipping, pixels outside the selection remain unchanged, and sparse uniform inputs remain usable without a source materialization

#### Scenario: Healing reads the same coverage [cov_healing]

- **WHEN** Patch source, destination, and content-aware modes and Content-Aware Move and Extend use equivalent flat and tiled shared coverage
- **THEN** each result and refusal agrees between storage forms, preserves its existing expected result, and does not materialize the coverage source

### Requirement: Clipboard operations accept shared coverage

Copy, Copy Merged, Cut, Clear, Paste Into, and Paste Outside SHALL take their
selection constraint through the shared coverage interface. They SHALL
preserve clipboard bounds, fractional alpha, existing placement and mask
creation rules, and their current history and refusal behavior. A small
selection SHALL NOT require a canvas-sized coverage copy.

#### Scenario: Clipboard coverage remains lossless and bounded [cov_clipboard]

- **WHEN** clipboard operations run with equivalent flat and tiled soft coverage over an offset layer, including empty coverage and a locked target
- **THEN** copied bounds and bytes, pasted masks, refusals, and document results agree, and reads do not materialize the tiled source or expand a small coverage crop to the canvas

### Requirement: Mask metadata stays with real masks

Coverage used as an editing constraint SHALL NOT expose invented enabled
state, density, feather, link flags, or persistence payload. Actual mask
construction SHALL assign the metadata required by the existing mask-creation
contract. Real-mask rendering, editing, and native-depth resolution SHALL
continue to interpret their actual metadata, including operation-specific
disabled-mask and missing-data policies. Sharing raw coverage access SHALL
NOT apply mask feather or density twice or change stored off-canvas samples.

#### Scenario: Persistent masks and refinements retain their semantics [cov_mask]

- **WHEN** selection coverage creates a real mask, and real masks with non-default metadata, exterior coverage, disabled state, missing data, and retained native/off-canvas samples are rendered or edited
- **THEN** selection-driven masks retain their expected metadata and coverage, real masks preserve existing rendering and edit policies, and unrelated metadata and retained samples remain unchanged

### Requirement: Feathered sparse coverage matches flat storage

The regression suite SHALL compare an actually feathered selection represented
as sparse tiled coverage with the same bytes represented flat. It SHALL
exercise a renderer edit and a paint edit through the shared interface over
tile boundaries, retain partial edge coverage, and assert that the tiled
source is not materialized. Existing filter, matting, mask-refinement,
clipboard, healing, and selection-clipping assertions SHALL continue to pass.

#### Scenario: A feathered sparse selection produces the flat result [cov_feather]

- **WHEN** a rectangular selection spanning tile boundaries is feathered, retains empty and uniform tiles plus partial edge values, and constrains a filter and a paint edit in tiled and flat form
- **THEN** the resulting pixels agree byte-for-byte, partial coverage remains effective, and the tiled reads produce zero source materializations
