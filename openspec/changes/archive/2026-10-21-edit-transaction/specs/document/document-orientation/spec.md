# Spec Delta

## MODIFIED Requirements

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
