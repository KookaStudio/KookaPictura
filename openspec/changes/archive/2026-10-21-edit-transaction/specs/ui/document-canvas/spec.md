# Spec Delta

## MODIFIED Requirements

### Requirement: Composite recomputation after document canvas resize

After a successful `resize_canvas_document`, the document's cached `doc.composite` SHALL be left byte-identical to its pre-call value; the operation MUST NOT run the compositor. The app's edit transaction SHALL replace it with `pictura_render::composite_rgba(doc)` at the new document size before displaying, capturing history, or saving, so the composite is computed from the translated layer tree. When the document has no layers and a merged composite whose dimensions match the document, the operation SHALL first materialize that composite as a Background layer, preserving its alpha, so the canvas resize translates those pixels.

#### Scenario: The cached composite equals a fresh composite after canvas resize

- **WHEN** the app commits a canvas resize through the edit transaction
- **THEN** `doc.composite` is bit-identical to `composite_rgba(&doc)` and has the new dimensions

#### Scenario: The engine leaves the composite and the app materializes it after canvas resize [dc_engine_then_app]

- **WHEN** the app commits a canvas resize through the edit transaction
- **THEN** `resize_canvas_document` left `doc.composite` byte-identical to its pre-call value, and the transaction's materialized composite is bit-identical to `composite_rgba(&doc)` at the new dimensions

#### Scenario: A merged-only document canvas resize keeps its pixels [dc_merged_only]

- **WHEN** `resize_canvas_document` is called on a document with no layers and a matching merged composite
- **THEN** the composite is materialized as a Background layer before the operation, the translated pixels are not stale or blank, and the result displays and saves with those pixels

### Requirement: Document canvas resize oracle

The system SHALL ship an oracle for `resize_canvas_document` in
`crates/pictura-render/tests/document_oracle.rs` that (a) asserts the engine left
the cached composite untouched, then materializes it with
`pictura_render::composite_rgba(doc)`, (b) writes the resized document with
`pictura_codec::write_psd`, re-reads it with `pictura_codec::read_psd`, and
confirms the structural round-trip, (c) opens the written file with the
independent `psd-tools` library and confirms the reported document dimensions and
layer count match the anchor-translated document, and (d) asserts the
materialized composite equals a fresh `composite_rgba(&doc)`. The oracle SHALL
skip with a message when `psd-tools` is not importable and MUST NOT be marked
`#[ignore]`.

#### Scenario: The canvas-resized document round-trips structurally

- **WHEN** a layered document is canvas-resized and written, then re-read by `pictura_codec::read_psd`
- **THEN** the re-read document has the new dimensions and every layer's `rect` matches the translated rect

#### Scenario: psd-tools sees the canvas-resized document

- **WHEN** the written PSD is opened with `psd-tools`
- **THEN** psd-tools reports the new document dimensions and layer count

#### Scenario: Missing psd-tools skips cleanly

- **WHEN** `psd-tools` is not importable
- **THEN** the oracle prints a skip message and the suite still passes

### Requirement: Layer translation uses the active backend

A layer move committed through the app's edit transaction SHALL composite the
declared dirty rectangle through the active backend — the GPU compositor when GPU
compute is enabled and a usable adapter exists, otherwise the CPU compositor —
and SHALL remain within ±1 LSB of the CPU oracle. The engine `translate_layer`
and `translate_layer_path` operations SHALL NOT recompute the composite as a
side effect; the `translate_layer_active` CPU/GPU helper remains the oracle and
MUST NOT change.

#### Scenario: A Move commit on a large document composites on the GPU

- **WHEN** a layer move is committed on a 4000×4000 document with GPU compute
  enabled and a usable adapter present
- **THEN** the document composite is recomputed on the GPU by the app's
  transaction and every channel differs from the CPU oracle by at most 1 LSB
