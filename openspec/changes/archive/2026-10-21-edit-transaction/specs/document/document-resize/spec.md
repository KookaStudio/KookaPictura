# Spec Delta

## MODIFIED Requirements

### Requirement: Composite recomputation after document resize

After a successful `resize_document`, the document's cached `doc.composite` SHALL be left byte-identical to its pre-call value; the operation MUST NOT run the compositor. The app's edit transaction SHALL replace it with `pictura_render::composite_rgba(doc)` — a 4-channel (R,G,B,A) buffer at the new document size — before displaying, capturing history, or saving, so the displayed and saved composite is computed from the resampled layer tree rather than resampled independently. When the document has no layers and a merged composite whose dimensions match the document, `resize_document` SHALL first materialize that composite as a Background layer, preserving its alpha, so the resize transforms those pixels.

#### Scenario: The cached composite equals a fresh composite after resize

- **WHEN** the app commits a resize through the edit transaction
- **THEN** `doc.composite` is bit-identical to `composite_rgba(&doc)` and has the new height and width with 4 channels

#### Scenario: The engine leaves the composite and the app materializes it after resize [dr_engine_then_app]

- **WHEN** the app commits a resize through the edit transaction
- **THEN** `resize_document` left `doc.composite` byte-identical to its pre-call value, and the transaction's materialized composite is bit-identical to `composite_rgba(&doc)` at the new size with 4 channels

#### Scenario: A merged-only document resizes its pixels [dr_merged_only]

- **WHEN** `resize_document` is called on a document with no layers and a matching merged composite
- **THEN** the composite is materialized as a Background layer before the resize, the resized pixels are transformed rather than stale or blank, and the result displays and saves with those pixels

### Requirement: Document resize oracle

The system SHALL ship an oracle for `resize_document` in `crates/pictura-render/tests/document_oracle.rs` that (a) asserts the engine left the cached composite untouched, then materializes it with `pictura_render::composite_rgba(doc)`, (b) writes the resized document with `pictura_codec::write_psd`, re-reads it with `pictura_codec::read_psd`, and confirms the structural round-trip matches the resized document, (c) opens the written file with the independent `psd-tools` library and confirms the reported document dimensions and layer count match the resized document, and (d) asserts the materialized composite equals a fresh `composite_rgba(&doc)`. The oracle SHALL skip with a message when `psd-tools` is not importable and MUST NOT be marked `#[ignore]`.

#### Scenario: The resized document round-trips structurally

- **WHEN** a layered document is resized and written, then re-read by `pictura_codec::read_psd`
- **THEN** the re-read document has the new dimensions and every layer's `rect` and channel lengths match the resized document

#### Scenario: psd-tools sees the resized document

- **WHEN** the written PSD is opened with `psd-tools`
- **THEN** psd-tools reports the new document width, height, and layer count

#### Scenario: Missing psd-tools skips cleanly

- **WHEN** `psd-tools` is not importable
- **THEN** the oracle prints a skip message and the suite still passes
